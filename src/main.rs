mod runner;
mod profiles;
mod parsing;
mod mutators;
mod utils;

use core::num;
use std::fs;
use tokio::fs::File as TokioFile;
use tokio::io::AsyncWriteExt;
use std::sync::Arc;
use tokio::sync::mpsc;
use std::path::PathBuf;
use tokio::task::JoinHandle;
use clap::Parser;

use crate::mutators::literals::NumericTweaker;
use crate::parsing::parser::{generate_js, parse_js};
use crate::runner::pool::JobResult;
use crate::mutators::AstMutator;
use crate::mutators::minifier::Minifier;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    output_dir: PathBuf,
    
    // overwrite existing corpus files and start from scratch
    #[arg(short, long, action=clap::ArgAction::SetTrue)]
    overwrite: Option<bool>,
    // if overwrite is true, we need an initial corpus directory to read from
    #[arg(short, long)]
    initial_corpus: Option<PathBuf>,
    
    // resume from existing corpus
    #[arg(short, long, action)]
    resume: Option<bool>,
    // the profile to use
    #[arg(short, long)]
    profile: String,
    // number of workers
    #[arg(short, long, default_value_t = 1)]
    workers: usize,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let output_dir = args.output_dir;
    let mutators = vec![
    Arc::new(NumericTweaker),
    ];
    
    let profile = profiles::get_profile(&args.profile).expect("Failed to get profile");
    let pool_size = args.workers;
    
    let mut pool = runner::pool::FuzzPool::new(
        pool_size,
        &profile
    ).expect("Failed to create FuzzPool");
    
    if args.overwrite.unwrap_or(false) {
        // check if the directory exists
        if output_dir.exists() {
            // prompt user to confirm deletion
            println!("Output directory {:?} already exists. Are you sure you want to overwrite it? (y/N)", output_dir);
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            if input.trim().to_lowercase() == "y" {
                fs::remove_dir_all(&output_dir).unwrap();
                fs::create_dir_all(&output_dir).unwrap();
                println!("Overwritten existing directory.");
            } else {
                println!("Aborting.");
                return;
            }
        } else {
            fs::create_dir_all(&output_dir).unwrap();
        }
        
        // read initial corpus, minify and then check if the engine can execute it without syntax errors
        let start = std::time::Instant::now();
        let mut num_files = 0;
        let mut err_cnt = 0;
        let intial_corpus_dir = args.initial_corpus.expect("Initial corpus directory must be provided when overwrite is set");
        let mut job_handles = vec![];
        
        // start the ingestor task
        let (job_result_tx, mut job_result_rx) = mpsc::channel::<(PathBuf, JobResult, Vec<u8>)>(5000);
        tokio::task::spawn(init_job_result_ingestor(output_dir, job_result_rx));
        
        for entry in fs::read_dir(intial_corpus_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let path_clone = path.clone();
            let src = fs::read(&path).unwrap();
            let src_clone = src.clone();
            let script = parse_js(String::from_utf8(src).unwrap());
            if script.is_err() {
                err_cnt += 1;
                continue;
            }
            let script = script.unwrap();
            
            let minified_script = Minifier::mutate(script);
            if let Err(e) = minified_script {
                unreachable!("Minification failed: {:?}", e);
            }
            let minified_script = minified_script.unwrap();
            
            let new_code = generate_js(minified_script);
            if let Err(e) = new_code {
                unreachable!("Code generation failed: {:?}", e);
            }
            let new_code = new_code.unwrap();
            
            // schedule a job to check if the engine can execute it
            let mut result_tx = pool.schedule_job(new_code.clone()).await.unwrap();
            let job_result_tx_clone = job_result_tx.clone();
            let handle = tokio::task::spawn(async move {
                let job_result = result_tx.recv().await.unwrap();
                if let Err(_) = job_result {
                    panic!("Engine failed to execute minified script from {:?}, original size: {}, minified size: {}", path_clone, src_clone.len(), new_code.len());
                }
                let job_result = job_result.unwrap();
                if job_result.status_code != 0 {
                    return;
                }
                job_result_tx_clone.send((path_clone, job_result, new_code)).await.unwrap();
            });
            job_handles.push(handle);
            
            if job_handles.len() >= 10000 {
                for handle in job_handles.drain(..) {
                    if let Err(err) = handle.await {
                        eprintln!("Result task panicked: {:?}", err);
                    }
                }
                println!("Processed {} files...", num_files);
            }
            num_files += 1;
        }
        for handle in job_handles {
            handle.await.unwrap();
        }
        let duration = start.elapsed();
        println!("Processed {} files in {:?}", num_files, duration);
        println!("{} files had errors and were skipped", err_cnt);
    }
    
}

/// Ingests initial job results and writes successful scripts to the output directory
pub async fn init_job_result_ingestor(
    output_dir: PathBuf, mut result_rx: mpsc::Receiver<(PathBuf, JobResult, Vec<u8>)>
) {
    while let Some((job_path, _job_result, script_bytes)) = result_rx.recv().await {
        let mut output_path = output_dir.clone();
        output_path.push(job_path.file_name().unwrap());
        let mut output_file = TokioFile::create(output_path).await.unwrap();
        output_file.write_all(&script_bytes).await.unwrap();
    }
}
