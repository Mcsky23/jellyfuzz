mod runner;
mod profiles;
mod parsing;
mod mutators;

use std::fs;
use tokio::fs::File as TokioFile;
use tokio::io::AsyncWriteExt;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::path::PathBuf;
use tokio::task::JoinHandle;

use crate::parsing::parser::{generate_js, parse_js};
use crate::runner::pool::JobResult;

#[tokio::main]
async fn main() {
    // const NUM_JOBS: usize = 10000;
    // let (job_results_tx, mut job_results_rx) = tokio::sync::mpsc::channel::<(JobResult, PathBuf)>(NUM_JOBS);

    // let profile = profiles::v8::V8Profile;
    // let mut pool = runner::pool::FuzzPool::new(14, &profile).unwrap();

    // let results_file = TokioFile::create("results.txt").await.unwrap();
    // let mut results_writer = tokio::io::BufWriter::new(results_file);
    // let mut job_handles: Vec<JoinHandle<()>> = Vec::new();
    // let mut submitted_jobs = 0;

    // tokio::spawn({
    //     async move {
    //         let mut cnt = 0;
    //         while let Some((job_result, path)) = job_results_rx.recv().await {
    //             let result_line = format!(
    //                 "\"{:?}\": status_code={}, signal={}, new_coverage={}, is_crash={}, is_timeout={}, edge_hits={}\n",
    //                 path,
    //                 job_result.status_code,
    //                 job_result.signal,
    //                 job_result.new_coverage,
    //                 job_result.is_crash,
    //                 job_result.is_timeout,
    //                 job_result.edge_hits.len()
    //             );
    //             if let Err(e) = results_writer.write_all(result_line.as_bytes()).await {
    //                 eprintln!("Failed to write result: {:?}", e);
    //             }
    //             cnt += 1;
    //         }
    //         println!("Processed {} job results", cnt);
    //     }
    // });

    let start_time = std::time::Instant::now();

    // for path in fs::read_dir("corpus/").unwrap() {
    //     let path = path.unwrap().path();
    //     let js_code = fs::read(&path).unwrap();
    //     if js_code.windows(7).any(|w| w == b"quit();") {
    //         println!("Skipping quit(); script");
    //         continue;
    //     }
    //     let res_rx = pool.schedule_job(js_code).await.unwrap();
    //     let job_results_tx = job_results_tx.clone();
    //     let handle = tokio::spawn(async move {
    //         let mut res_rx = res_rx;
    //         match res_rx.recv().await {
    //             Some(Ok(job_result)) => {
    //                 job_results_tx.send((job_result, path)).await.unwrap();
    //             }
    //             Some(Err(e)) => {
    //                 eprintln!("Job Error: {:?}", e);
    //             }
    //             None => {
    //                 eprintln!("Job result channel closed before receiving outcome");
    //             }
    //         }
    //     });
    //     job_handles.push(handle);
    //     if job_handles.len() >= 10000 {
    //         for handle in job_handles.drain(..) {
    //             if let Err(err) = handle.await {
    //                 eprintln!("Result task panicked: {:?}", err);
    //             }
    //         }
    //     }
    //     submitted_jobs += 1;
    //     if submitted_jobs >= NUM_JOBS {
    //         break;
    //     }
    // }

    // for handle in job_handles {
    //     if let Err(err) = handle.await {
    //         eprintln!("Result task panicked: {:?}", err);
    //     }
    // }

    let mut cnt = 0;
    let mut err = 0;
    for path in fs::read_dir("corpus_raw/").unwrap() {
        let path = path.unwrap().path();
        let js_code = fs::read(&path).unwrap();
        // println!("{:?}", path);
        let script = parse_js(String::from_utf8(js_code).unwrap_or("".to_string()));
        if let Err(e) = script {
            // println!("Error found in {:?}: {:?}", path, e);
            err += 1;
        } else if let Ok(script) = script {
            let new_code = generate_js(script);
            if let Ok(new_code) = new_code {
                let new_path = PathBuf::from("corpus").join(path.file_name().unwrap());
                fs::write(new_path, new_code).unwrap();
            } else {
                unreachable!();
            }
        }
        cnt += 1;
    }
    // let src = fs::read("corpus/ba1b4b8a61dfbae7172efef9d2bb8628.js").unwrap();
    // let module = parse_js(String::from_utf8(src).unwrap());
    let duration = start_time.elapsed();
    println!("Executed {} jobs in {:?}", cnt, duration);
    println!("{} erros", err);

}
