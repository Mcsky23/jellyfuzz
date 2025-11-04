use libc::c_void;
use std::collections::{HashMap, HashSet};
use std::io;
use std::mem::MaybeUninit;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, AtomicU32, AtomicUsize, Ordering};
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::{OwnedSemaphorePermit, RwLock, Semaphore, mpsc};
use tokio::task::yield_now;

static NEXT_COV_CONTEXT_ID: AtomicI32 = AtomicI32::new(0);

use crate::profiles::profile::JsEngineProfile;
use crate::runner::{coverage::*, process::FuzzProcess};

lazy_static::lazy_static! {
    pub static ref TOTAL_EDGE_COUNT: AtomicU32 = AtomicU32::new(0);
}

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
    edge_tracker: Arc<RwLock<EdgeTracker>>,
}

/// The fuzzer pool contains multiple fuzz processes
pub struct FuzzPool {
    job_senders: Vec<mpsc::Sender<Job>>,
    next_worker: usize,
    job_capacity: Arc<Semaphore>,
    edge_tracker: Arc<RwLock<EdgeTracker>>,
}

/// Helper struct to track seen edges during executions
///
/// When we see new coverage on an execution, we re-run the same input and then intersect
/// the 2 edge sets to filter out flaky edges. If we see that a certain edge is not part
/// of the intersection more than `max_resets` times, we blacklist it.
pub struct EdgeTracker {
    seen_edges: HashSet<u32>,
    blacklist: HashMap<u32, usize>, // edge -> reset count
    max_resets: usize,
}

impl FuzzWorker {
    pub fn new<T: JsEngineProfile>(
        profile: &T,
        edge_tracker: Arc<RwLock<EdgeTracker>>,
    ) -> anyhow::Result<Self> {
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

        let (job_queue_tx, job_queue_rx) = mpsc::channel(profile.fuzz_worker_job_queue_size());

        Self::set_edge_count(&mut cov_ctx);

        Ok(Self {
            process: target,
            cov_ctx,
            job_queue: job_queue_rx,
            job_tx: job_queue_tx,
            edge_tracker,
        })
    }

    pub fn set_edge_count(cov_ctx: &mut CovContext) {
        let total_edge_count = cov_ctx.num_edges;
        if TOTAL_EDGE_COUNT.load(Ordering::SeqCst) == 0 {
            TOTAL_EDGE_COUNT.store(total_edge_count, Ordering::SeqCst);
        }
    }

    pub fn get_job_sender(&self) -> mpsc::Sender<Job> {
        self.job_tx.clone()
    }

    fn start_internal(&mut self, js_code: &[u8]) -> anyhow::Result<JobResult> {
        unsafe {
            cov_clear_bitmap(&mut self.cov_ctx);
        }
        let exec_status = self.process.execute(js_code);
        let timed_out =
            matches!(exec_status, Err(ref err) if err.kind() == io::ErrorKind::TimedOut);

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
                let slice =
                    unsafe { std::slice::from_raw_parts(edges.edge_indices, edges.count as usize) };
                edge_hits.extend_from_slice(slice);
                unsafe { libc::free(edges.edge_indices as *mut c_void) };
            }
            new_cov_flag = new_cov == 1;
        }

        if new_cov_flag && !edge_hits.is_empty() {
            match self.confirm_new_edges(js_code, &edge_hits) {
                Ok(stable_edges) => {
                    if stable_edges.is_empty() {
                        new_cov_flag = false;
                        edge_hits.clear();
                    } else {
                        edge_hits = stable_edges;
                    }
                }
                Err(err) => {
                    eprintln!("Failed to confirm new coverage: {:?}", err);
                    new_cov_flag = false;
                    edge_hits.clear();
                }
            }
        }
        let (status_code, signal, is_crash) = match exec_status {
            Ok(status) => (status.exit_code, status.signal, false),
            Err(err) => {
                if err.kind() == io::ErrorKind::TimedOut {
                    (-1, 0, false)
                } else {
                    println!("code: {:?}", js_code);
                    println!("Execution error: {:?}", err);
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

    /// Run the js code a second time and do intersection of edges to confirm stable new edges
    fn confirm_new_edges(
        &mut self,
        js_code: &[u8],
        candidate_edges: &[u32],
    ) -> anyhow::Result<Vec<u32>> {
        if candidate_edges.is_empty() {
            return Ok(Vec::new());
        }

        for &edge in candidate_edges {
            unsafe { cov_clear_edge_data(&mut self.cov_ctx, edge) };
        }

        unsafe { cov_clear_bitmap(&mut self.cov_ctx) };

        let exec_status = self.process.execute(js_code);
        let timed_out =
            matches!(exec_status, Err(ref err) if err.kind() == io::ErrorKind::TimedOut);
        if timed_out {
            self.process.restart()?;
            self.process.handshake()?;
            return Ok(Vec::new());
        }

        if exec_status.is_err() {
            self.process.restart()?;
            self.process.handshake()?;
            return Ok(Vec::new());
        }

        let mut edges = EdgeSet {
            count: 0,
            edge_indices: std::ptr::null_mut(),
        };
        let new_cov = unsafe { cov_evaluate(&mut self.cov_ctx, &mut edges) };

        if edges.edge_indices.is_null() {
            return Ok(Vec::new());
        }

        let second_slice =
            unsafe { std::slice::from_raw_parts(edges.edge_indices, edges.count as usize) };
        let mut first_set: HashSet<u32> = HashSet::with_capacity(candidate_edges.len());
        let mut second_set: HashSet<u32> = HashSet::with_capacity(second_slice.len());
        
        first_set.extend(candidate_edges.iter());
        second_set.extend(second_slice.iter());
        let mut stable_edges = Vec::new();

        // aux tracking buffer(so we don't have to lock multiple times)
        let mut aux_tracker = HashMap::new();

        for &edge in candidate_edges {
            if !second_set.contains(&edge) {
                let reset_count = aux_tracker.entry(edge).or_insert(0);
                *reset_count += 1;
                // unsafe { cov_clear_edge_data(&mut self.cov_ctx, edge) };
            }
        }

        for &edge in second_slice {
            if first_set.contains(&edge) {
                stable_edges.push(edge);
            } else {
                let reset_count = aux_tracker.entry(edge).or_insert(0);
                *reset_count += 1;
                unsafe { cov_clear_edge_data(&mut self.cov_ctx, edge) };
            }
        }
        unsafe { libc::free(edges.edge_indices as *mut c_void) };

        if new_cov != 1 {
            return Ok(Vec::new());
        }

        // Update edge tracker
        let stable_edges = {
            let mut tracker = self.edge_tracker.blocking_write();
            for (edge, count) in aux_tracker.iter() {
                let entry = tracker.blacklist.entry(*edge).or_insert(0);
                *entry += count;
                if *entry >= tracker.max_resets {
                    // eprintln!("Blacklisting edge {}", edge);
                }
            }
            let mut stable_edges_curated = vec![];
            for &edge in &stable_edges {
                if tracker.blacklist.get(&edge).unwrap_or(&0) >= &tracker.max_resets || 
                    tracker.seen_edges.contains(&edge) {
                    continue;
                }
                stable_edges_curated.push(edge);
                tracker.seen_edges.insert(edge);
            }
            stable_edges_curated
        };

        Ok(stable_edges)
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
        let edge_tracker = Arc::new(RwLock::new(EdgeTracker {
            seen_edges: HashSet::new(),
            blacklist: HashMap::new(),
            max_resets: 1000,
        }));
        for _ in 0..num_workers {
            let worker = FuzzWorker::new(profile, edge_tracker.clone())?;
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
            edge_tracker,
        })
    }

    /// Schedule a job to be executed by one of the FuzzWorkers
    pub async fn schedule_job(
        &mut self,
        js_code: Vec<u8>,
    ) -> anyhow::Result<mpsc::Receiver<anyhow::Result<JobResult>>> {
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
        self.schedule_job(js_code)
            .await?
            .recv()
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to receive job result"))?
    }

    pub async fn print_pool_stats(&self) {
        let tracker = self.edge_tracker.read().await;
        println!(
            "Edge tracker: seen edges: {}, blacklisted edges: {}, total edges: {}, coverage: {:.2}%",
            tracker.seen_edges.len(),
            tracker.blacklist.iter().filter(|&(_, &count)| count >= tracker.max_resets).count(),
            TOTAL_EDGE_COUNT.load(Ordering::SeqCst),
            (tracker.seen_edges.len() as f64 / TOTAL_EDGE_COUNT.load(Ordering::SeqCst) as f64) * 100.0
        );
    }
}

impl Drop for FuzzWorker {
    fn drop(&mut self) {
        let _ = self.process.child.kill();
    }
}
