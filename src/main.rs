mod runner;
mod profiles;

use std::fs;
use tokio::fs::File as TokioFile;
use tokio::io::AsyncWriteExt;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::path::PathBuf;
use tokio::task::JoinHandle;

use crate::runner::pool::JobResult;

#[tokio::main]
async fn main() {
    const NUM_JOBS: usize = 150000;
    let (job_results_tx, mut job_results_rx) = tokio::sync::mpsc::channel::<(JobResult, PathBuf)>(NUM_JOBS);

    let profile = profiles::v8::V8Profile;
    let mut pool = runner::pool::FuzzPool::new(14, &profile).unwrap();

    let results_file = TokioFile::create("results.txt").await.unwrap();
    let mut results_writer = tokio::io::BufWriter::new(results_file);
    let mut job_handles: Vec<JoinHandle<()>> = Vec::new();
    let mut submitted_jobs = 0;

    tokio::spawn({
        async move {
            let mut cnt = 0;
            while let Some((job_result, path)) = job_results_rx.recv().await {
                let result_line = format!(
                    "\"{:?}\": status_code={}, signal={}, new_coverage={}, is_crash={}, is_timeout={}, edge_hits={}\n",
                    path,
                    job_result.status_code,
                    job_result.signal,
                    job_result.new_coverage,
                    job_result.is_crash,
                    job_result.is_timeout,
                    job_result.edge_hits.len()
                );
                if let Err(e) = results_writer.write_all(result_line.as_bytes()).await {
                    eprintln!("Failed to write result: {:?}", e);
                }
                cnt += 1;
            }
            println!("Processed {} job results", cnt);
        }
    });

    let start_time = std::time::Instant::now();

    for path in fs::read_dir("corpus/").unwrap() {
        let path = path.unwrap().path();
        let js_code = fs::read(&path).unwrap();
        if js_code.windows(7).any(|w| w == b"quit();") {
            println!("Skipping quit(); script");
            continue;
        }
        let res_rx = pool.schedule_job(js_code).await.unwrap();
        let job_results_tx = job_results_tx.clone();
        let handle = tokio::spawn(async move {
            let mut res_rx = res_rx;
            match res_rx.recv().await {
                Some(Ok(job_result)) => {
                    job_results_tx.send((job_result, path)).await.unwrap();
                }
                Some(Err(e)) => {
                    eprintln!("Job Error: {:?}", e);
                }
                None => {
                    eprintln!("Job result channel closed before receiving outcome");
                }
            }
        });
        job_handles.push(handle);
        if job_handles.len() >= 10000 {
            for handle in job_handles.drain(..) {
                if let Err(err) = handle.await {
                    eprintln!("Result task panicked: {:?}", err);
                }
            }
        }
        submitted_jobs += 1;
        if submitted_jobs >= NUM_JOBS {
            break;
        }
    }

    for handle in job_handles {
        if let Err(err) = handle.await {
            eprintln!("Result task panicked: {:?}", err);
        }
    }

    let duration = start_time.elapsed();
    println!("Executed {} jobs in {:?}", NUM_JOBS, duration);

}
