use tokio::sync::mpsc;

use crate::{profiles::profile::JsEngineProfile};
use crate::runner::{process::FuzzProcess, coverage::*};
use crate::MaybeUninit;

/// A job to be executed by a FuzzWorker
pub struct Job {
    js_code: Vec<u8>,
    result_tx: mpsc::Sender<anyhow::Result<JobResult>>,
}

/// The result of a job executed by a FuzzWorker
pub struct JobResult {
    status_code: i32,
    new_coverage: bool,
    edge_hits: Vec<u32>,
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
    pool: Vec<(FuzzWorker, mpsc::Sender<Job>)>,
}

impl FuzzWorker {
    pub fn new<T: JsEngineProfile>(profile: &T) -> anyhow::Result<Self> {
        let mut cov_ctx = unsafe {
            let mut ctx = MaybeUninit::<CovContext>::zeroed().assume_init();
            ctx.id = 0;
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

    /// Schedule a job to be executed sometime in the future by this fuzz worker
    /// Returns a receiver to get the result of the job once it is done
    pub async fn schedule_job(&mut self, js_code: Vec<u8>) -> anyhow::Result<mpsc::Receiver<anyhow::Result<JobResult>>> {
        let (result_tx, result_rx) = mpsc::channel(1);
        let job = Job {
            js_code,
            result_tx,
        };
        let job_tx = self.get_job_sender();
        job_tx.send(job).await.unwrap();
        Ok(result_rx)
    }

    async fn run(&mut self) -> anyhow::Result<()> {
        unimplemented!();
    }
}

impl FuzzPool {
    pub fn new<T: JsEngineProfile>(num_workers: usize, profile: &T) -> anyhow::Result<Self> {
        let mut pool = Vec::new();
        for _ in 0..num_workers {
            let worker = FuzzWorker::new(profile)?;
            let job_tx = worker.get_job_sender();
            pool.push((worker, job_tx));
        }
        Ok(Self { pool })
    }
}
