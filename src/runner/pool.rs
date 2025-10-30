use tokio::sync::{mpsc, Semaphore, OwnedSemaphorePermit};
use tokio::sync::mpsc::error::TrySendError;
use tokio::task::yield_now;
use libc::c_void;
use std::io;
use std::mem::MaybeUninit;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

static NEXT_COV_CONTEXT_ID: AtomicI32 = AtomicI32::new(0);

use crate::{profiles::profile::JsEngineProfile};
use crate::runner::{process::FuzzProcess, coverage::*};

/// A job to be executed by a FuzzWorker
pub struct Job {
    js_code: Vec<u8>,
    result_tx: mpsc::Sender<anyhow::Result<JobResult>>,
    permit: Option<OwnedSemaphorePermit>,
}

impl Job {
    fn new(
        js_code: Vec<u8>,
        result_tx: mpsc::Sender<anyhow::Result<JobResult>>,
        permit: OwnedSemaphorePermit,
    ) -> Self {
        Self {
            js_code,
            result_tx,
            permit: Some(permit),
        }
    }

    fn into_parts(mut self) -> (Vec<u8>, mpsc::Sender<anyhow::Result<JobResult>>) {
        // Dropping the permit here releases global queue capacity.
        self.permit.take();
        (self.js_code, self.result_tx)
    }
}

/// The result of a job executed by a FuzzWorker
pub struct JobResult {
    pub status_code: i32,
    pub signal: i32,
    pub new_coverage: bool,
    pub edge_hits: Vec<u32>,
    pub is_crash: bool,
    pub is_timeout: bool,
    // pub edge_hash: Option<Vec
}

/// Stand-alone FuzzProcess wrapper that holds a queue for Js code to be executed in said process
pub struct FuzzWorker {
    process: FuzzProcess,
    cov_ctx: CovContext,
    job_queue: mpsc::Receiver<Job>,
    job_tx: mpsc::Sender<Job>, // interface to send jobs to this worker
}

/// The fuzzer pool contains multiple fuzz processes
pub struct FuzzPool {
    job_senders: Vec<mpsc::Sender<Job>>,
    next_worker: usize,
    job_capacity: Arc<Semaphore>,
}

impl FuzzWorker {
    pub fn new<T: JsEngineProfile>(profile: &T) -> anyhow::Result<Self> {
        let mut cov_ctx = unsafe {
            let mut ctx = MaybeUninit::<CovContext>::zeroed().assume_init();
            ctx.id = NEXT_COV_CONTEXT_ID.fetch_add(1, Ordering::Relaxed);
            if cov_initialize(&mut ctx) != 0 {
                panic!("cov_initialize failed");
            }
            ctx
        };
        let shm_id = format!("shm_id_{}_{}", std::process::id(), cov_ctx.id);
        
        let mut target = FuzzProcess::spawn(profile, &shm_id)?;
        target.handshake()?;
        
        unsafe {
            cov_finish_initialization(&mut cov_ctx, 0);
        }
        
        let (job_queue_tx, job_queue_rx) = mpsc::channel(
            profile.fuzz_worker_job_queue_size()
        );
        
        Ok(
            Self {
                process: target,
                cov_ctx,
                job_queue: job_queue_rx,
                job_tx: job_queue_tx
            }
        )
    }
    
    pub fn get_job_sender(&self) -> mpsc::Sender<Job> {
        self.job_tx.clone()
    }
    
    fn start_internal(&mut self, js_code: &[u8]) -> anyhow::Result<JobResult> {
        unsafe { cov_clear_bitmap(&mut self.cov_ctx); }
        let exec_status = self.process.execute(js_code);
        let timed_out = matches!(exec_status, Err(ref err) if err.kind() == io::ErrorKind::TimedOut);

        if timed_out {
            self.process.restart()?;
            self.process.handshake()?;
            return Ok(JobResult {
                status_code: -1,
                signal: 0,
                new_coverage: false,
                edge_hits: Vec::new(),
                is_crash: false,
                is_timeout: true,
            });
        }
        
        let mut edge_hits = Vec::new();
        let mut new_cov_flag = false;
        if exec_status.is_ok() {
            let mut edges = EdgeSet {
                count: 0,
                edge_indices: std::ptr::null_mut(),
            };
            let new_cov = unsafe { cov_evaluate(&mut self.cov_ctx, &mut edges) };
            if new_cov == 1 && !edges.edge_indices.is_null() {
                let slice = unsafe {
                    std::slice::from_raw_parts(edges.edge_indices, edges.count as usize)
                };
                edge_hits.extend_from_slice(slice);
                unsafe { libc::free(edges.edge_indices as *mut c_void) };
            }
            new_cov_flag = new_cov == 1;
        }
        let (status_code, signal, is_crash) = match exec_status {
            Ok(status) => {
                (status.exit_code, status.signal, false)
            }
            Err(err) => {
                if err.kind() == io::ErrorKind::TimedOut {
                    (-1, 0, false)
                } else {
                    (-1, -1, true)
                }
            }
        };
        if is_crash {
            self.process.restart()?;
            self.process.handshake()?;
        }
        let job_result = JobResult {
            status_code,
            signal,
            new_coverage: if timed_out { false } else { new_cov_flag },
            edge_hits: if timed_out { Vec::new() } else { edge_hits },
            is_crash,
            is_timeout: timed_out,
        };
        Ok(job_result)
    }
    
    /// Start the fuzz worker's main loop
    pub async fn run(mut self) -> anyhow::Result<()> {
        while let Some(job) = self.job_queue.recv().await {
            let (js_code, result_tx) = job.into_parts();
            let job_result = tokio::task::block_in_place(|| self.start_internal(&js_code))?;
            result_tx
                .send(Ok(job_result))
                .await
                .map_err(|_| anyhow::anyhow!("failed to deliver job result"))?;
        }
        Ok(())
    }
}

impl FuzzPool {
    pub fn new<T: JsEngineProfile>(num_workers: usize, profile: &T) -> anyhow::Result<Self> {
        let mut job_senders = Vec::new();
        for _ in 0..num_workers {
            let worker = FuzzWorker::new(profile)?;
            let job_tx = worker.get_job_sender();
            tokio::spawn(async move {
                if let Err(err) = worker.run().await {
                    eprintln!("FuzzWorker exited with error: {:?}", err);
                }
            });
            job_senders.push(job_tx);
        }
        let queue_capacity = num_workers * profile.fuzz_worker_job_queue_size().max(1);
        Ok(Self {
            job_senders,
            next_worker: 0,
            job_capacity: Arc::new(Semaphore::new(queue_capacity)),
        })
    }

    /// Schedule a job to be executed by one of the FuzzWorkers
    pub async fn schedule_job(&mut self, js_code: Vec<u8>) -> anyhow::Result<mpsc::Receiver<anyhow::Result<JobResult>>> {
        if self.job_senders.is_empty() {
            return Err(anyhow::anyhow!("No fuzz workers available"));
        }

        let (result_tx, result_rx) = mpsc::channel(1);
        let worker_count = self.job_senders.len();
        let permit = self
            .job_capacity
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| anyhow::anyhow!("Fuzz pool capacity semaphore closed"))?;
        let mut job = Job::new(js_code, result_tx, permit);

        loop {
            for offset in 0..worker_count {
                let idx = (self.next_worker + offset) % worker_count;
                match self.job_senders[idx].try_send(job) {
                    Ok(()) => {
                        self.next_worker = (idx + 1) % worker_count;
                        return Ok(result_rx);
                    }
                    Err(TrySendError::Full(returned_job)) => {
                        job = returned_job;
                    }
                    Err(TrySendError::Closed(_)) => {
                        return Err(anyhow::anyhow!("Fuzz worker channel closed"));
                    }
                }
            }
            yield_now().await;
        }
    }
    
    /// Execute a job and wait for the result
    pub async fn execute_job(&mut self, js_code: Vec<u8>) -> anyhow::Result<JobResult> {
        self.schedule_job(js_code).await?
            .recv().await
            .ok_or_else(|| anyhow::anyhow!("Failed to receive job result"))?
    }
}
