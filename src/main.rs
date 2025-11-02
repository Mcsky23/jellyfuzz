mod corpus;
mod mutators;
mod parsing;
mod profiles;
mod runner;
mod utils;

use anyhow::{Context, Result, bail};
use clap::Parser;
use rand::seq::IndexedRandom;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::fs as async_fs;
use tokio::sync::Mutex;
use tokio::time::{Instant, sleep};

use crate::corpus::CorpusManager;
use crate::mutators::minifier::Minifier;
use crate::mutators::{ManagedMutator, get_ast_mutators};
use crate::parsing::parser::{generate_js, parse_js};
use crate::runner::pool::{FuzzPool, JobResult};

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
    // single test mode
    #[arg(long)]
    single_test: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let output_dir = args.output_dir.clone();

    if let Some(test_path) = args.single_test.as_deref() {
        single_test(test_path, &args.profile).await;
        return Ok(());
    }

    if args.overwrite.unwrap_or(false) {
        handle_overwrite(&output_dir)?;
    } else if !output_dir.exists() {
        fs::create_dir_all(&output_dir)
            .with_context(|| format!("failed to create output directory {:?}", output_dir))?;
    }

    let corpus_manager = Arc::new(Mutex::new(CorpusManager::load(output_dir.clone()).await?));
    let profile = profiles::get_profile(&args.profile)
        .unwrap_or_else(|| panic!("unknown profile {}", args.profile));
    let pool_size = args.workers;
    let mut pool = FuzzPool::new(pool_size, &profile)?;

    if args.overwrite.unwrap_or(false) {
        let initial_corpus = args
            .initial_corpus
            .clone()
            .expect("initial corpus directory is required when overwrite is set");
        ingest_initial_corpus(&mut pool, Arc::clone(&corpus_manager), initial_corpus).await?;
    } else if args.resume.unwrap_or(false) {
        let len = {
            let mgr = corpus_manager.lock().await;
            mgr.len()
        };
        println!("Resuming with {} corpus entries loaded from disk", len);
    }

    let is_empty = {
        let mgr = corpus_manager.lock().await;
        mgr.is_empty()
    };
    if is_empty {
        println!("Corpus is empty; nothing to fuzz.");
        return Ok(());
    }

    let mutators = get_ast_mutators();
    run_fuzz_loop(&mut pool, Arc::clone(&corpus_manager), &mutators).await
}

fn handle_overwrite(output_dir: &PathBuf) -> Result<()> {
    if output_dir.exists() {
        println!(
            "Output directory {:?} already exists. Overwrite it? (y/N)",
            output_dir
        );
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .context("failed to read overwrite confirmation")?;
        if input.trim().eq_ignore_ascii_case("y") {
            fs::remove_dir_all(output_dir)
                .with_context(|| format!("failed to remove {:?}", output_dir))?;
            fs::create_dir_all(output_dir)
                .with_context(|| format!("failed to recreate {:?}", output_dir))?;
            println!("Existing corpus removed.");
        } else {
            bail!("aborted by user");
        }
    } else {
        fs::create_dir_all(output_dir)
            .with_context(|| format!("failed to create {:?}", output_dir))?;
    }
    Ok(())
}

async fn ingest_initial_corpus(
    pool: &mut FuzzPool,
    corpus_manager: Arc<Mutex<CorpusManager>>,
    corpus_dir: PathBuf,
) -> Result<()> {
    let start = std::time::Instant::now();
    let processed = Arc::new(AtomicUsize::new(0));
    let accepted = Arc::new(AtomicUsize::new(0));
    let skipped = Arc::new(AtomicUsize::new(0));
    let minifier = Minifier;
    let mut dir = async_fs::read_dir(&corpus_dir)
        .await
        .with_context(|| format!("failed to read corpus directory {:?}", corpus_dir))?;
    let mut handles = Vec::new();

    while let Some(entry) = dir
        .next_entry()
        .await
        .with_context(|| "failed to iterate corpus directory".to_string())?
    {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .await
            .with_context(|| format!("failed to determine file type for {:?}", path))?;
        if !file_type.is_file() {
            continue;
        }

        let processed_now = processed.fetch_add(1, Ordering::Relaxed) + 1;

        let source = match async_fs::read(&path).await {
            Ok(data) => data,
            Err(err) => {
                eprintln!("Failed to read {:?}: {:?}", path, err);
                skipped.fetch_add(1, Ordering::Relaxed);
                continue;
            }
        };

        let source_str = match String::from_utf8(source) {
            Ok(src) => src,
            Err(_) => {
                skipped.fetch_add(1, Ordering::Relaxed);
                continue;
            }
        };

        let script = match parse_js(source_str) {
            Ok(script) => script,
            Err(err) => {
                eprintln!("Failed to parse {:?}: {:?}", path, err);
                skipped.fetch_add(1, Ordering::Relaxed);
                continue;
            }
        };

        let minified = match minifier.mutate(script) {
            Ok(script) => script,
            Err(err) => {
                eprintln!("Failed to minify {:?}: {:?}", path, err);
                skipped.fetch_add(1, Ordering::Relaxed);
                continue;
            }
        };

        let new_code = match generate_js(minified) {
            Ok(code) => code,
            Err(err) => {
                eprintln!("Failed to regenerate code for {:?}: {:?}", path, err);
                skipped.fetch_add(1, Ordering::Relaxed);
                continue;
            }
        };

        let exec_start = Instant::now();
        let result_rx = match pool.schedule_job(new_code.clone()).await {
            Ok(rx) => rx,
            Err(err) => {
                eprintln!("Failed to schedule job for {:?}: {:?}", path, err);
                skipped.fetch_add(1, Ordering::Relaxed);
                continue;
            }
        };

        let corpus_manager_clone = Arc::clone(&corpus_manager);
        let accepted_clone = Arc::clone(&accepted);
        let skipped_clone = Arc::clone(&skipped);
        let path_clone = path.clone();
        let handle = tokio::spawn(async move {
            let mut result_rx = result_rx;
            let job_result = match result_rx.recv().await {
                Some(Ok(res)) => res,
            Some(Err(err)) => {
                eprintln!("Worker rejected {:?}: {:?}", path_clone, err);
                skipped_clone.fetch_add(1, Ordering::Relaxed);
                return;
            }
                None => {
                    eprintln!("Worker dropped job for {:?}", path_clone);
                    skipped_clone.fetch_add(1, Ordering::Relaxed);
                    return;
                }
            };

            let exec_time = exec_start.elapsed().as_millis() as u64;
            if job_result.status_code != 0 || job_result.is_timeout {
                skipped_clone.fetch_add(1, Ordering::Relaxed);
                return;
            }

            let reward = compute_reward(&job_result);
            let mut manager = corpus_manager_clone.lock().await;
            match manager
                .add_entry(&new_code, job_result.edge_hits.clone(), reward, exec_time)
                .await
            {
                Ok(Some(_)) => {
                    accepted_clone.fetch_add(1, Ordering::Relaxed);
                }
                Ok(None) => {}
                Err(err) => {
                    eprintln!(
                        "Failed to add initial corpus entry {:?}: {:?}",
                        path_clone, err
                    );
                    skipped_clone.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
        handles.push(handle);
        if handles.len() >= 10000 {
            for handle in handles.drain(..) {
                if let Err(err) = handle.await {
                    eprintln!("Ingestion task failed: {:?}", err);
                }
            }
            println!("Ingested {} files...", processed_now);
        }
    }

    for handle in handles {
        if let Err(err) = handle.await {
            eprintln!("Ingestion task failed: {:?}", err);
        }
    }

    let processed = processed.load(Ordering::Relaxed);
    let accepted = accepted.load(Ordering::Relaxed);
    let skipped = skipped.load(Ordering::Relaxed);
    let elapsed = start.elapsed();
    println!(
        "Initial corpus ingestion complete: {} accepted, {} skipped out of {} files in {:?}",
        accepted, skipped, processed, elapsed
    );
    Ok(())
}

async fn run_fuzz_loop(
    pool: &mut FuzzPool,
    corpus_manager: Arc<Mutex<CorpusManager>>,
    mutators: &[Arc<ManagedMutator>],
) -> Result<()> {
    let mut iteration: u64 = 0;
    loop {
        iteration += 1;
        let selection = {
            let mut mgr = corpus_manager.lock().await;
            mgr.pick_random()
        };
        let selection = match selection {
            Some(sel) => sel,
            None => {
                sleep(Duration::from_millis(50)).await;
                continue;
            }
        };

        let seed_bytes = match async_fs::read(&selection.path).await {
            Ok(data) => data,
            Err(err) => {
                eprintln!("Failed to read seed {:?}: {:?}", selection.path, err);
                continue;
            }
        };
        let seed_source = match String::from_utf8(seed_bytes) {
            Ok(src) => src,
            Err(err) => {
                eprintln!("Skipping non-UTF8 seed {:?}: {:?}", selection.path, err);
                continue;
            }
        };
        let seed_script = match parse_js(seed_source) {
            Ok(ast) => ast,
            Err(err) => {
                eprintln!("Failed to parse seed {:?}: {:?}", selection.path, err);
                continue;
            }
        };

        let mut rng = rand::rng();
        let mutator = match mutators.choose(&mut rng) {
            Some(m) => Arc::clone(m),
            None => {
                eprintln!("No mutators registered; aborting fuzz loop.");
                break;
            }
        };

        let mutated_ast = match mutator.mutate(seed_script) {
            Ok(ast) => ast,
            Err(err) => {
                eprintln!(
                    "Mutator {} failed on {:?}: {:?}",
                    mutator.name(),
                    selection.path,
                    err
                );
                continue;
            }
        };

        let mutated_code = match generate_js(mutated_ast) {
            Ok(code) => code,
            Err(err) => {
                eprintln!("Code generation failed: {:?}", err);
                continue;
            }
        };

        let exec_start = Instant::now();
        let mut result_rx = match pool.schedule_job(mutated_code.clone()).await {
            Ok(rx) => rx,
            Err(err) => {
                eprintln!("Failed to schedule job: {:?}", err);
                continue;
            }
        };
        let job_result = match result_rx.recv().await {
            Some(Ok(res)) => res,
            Some(Err(err)) => {
                eprintln!("Worker execution error: {:?}", err);
                continue;
            }
            None => {
                eprintln!("Worker dropped execution result");
                continue;
            }
        };
        let exec_time_ms = exec_start.elapsed().as_millis() as u64;
        let reward = compute_reward(&job_result);

        mutator.record_reward(reward);
        {
            let mut mgr = corpus_manager.lock().await;
            mgr.record_result(selection.id, reward, exec_time_ms)
                .await?;
        }

        if job_result.is_crash {
            println!(
                "Crash detected (exit {}, signal {}); reward {}",
                job_result.status_code, job_result.signal, reward
            );
            let root_path = {
                let mgr = corpus_manager.lock().await;
                mgr.root().to_path_buf()
            };
            let crash_path = output_crash_path(root_path.as_path(), iteration);
            persist_crash(crash_path.as_path(), &mutated_code).await?;
        }
        println!(
            "[iter {}] exec_time: {} ms, reward: {}, new_coverage: {}, exit_code: {}, signal: {}, path: {:?}",
            iteration,
            exec_time_ms,
            reward,
            job_result.new_coverage,
            job_result.status_code,
            job_result.signal,
            selection.path
        );
        if job_result.new_coverage {
            let add_result = {
                let mut mgr = corpus_manager.lock().await;
                mgr.add_entry(
                    &mutated_code,
                    job_result.edge_hits.clone(),
                    reward.max(0.0),
                    exec_time_ms,
                )
                .await
            };
            if let Ok(Some(entry)) = add_result {
                println!(
                    "[iter {}] new corpus entry {} ({} bytes)",
                    iteration, entry.id, entry.size_bytes
                );
            }
        }
    }
    Ok(())
}

fn compute_reward(result: &JobResult) -> f64 {
    if result.is_crash {
        5.0
    } else if result.new_coverage {
        1.0
    } else if result.is_timeout {
        -1.0
    } else {
        0.0
    }
}

async fn persist_crash(path: &std::path::Path, contents: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        async_fs::create_dir_all(parent)
            .await
            .with_context(|| format!("failed to create crash directory {:?}", parent))?;
    }
    async_fs::write(path, contents)
        .await
        .with_context(|| format!("failed to save crash repro {:?}", path))?;
    Ok(())
}

fn output_crash_path(root: &std::path::Path, iteration: u64) -> PathBuf {
    let mut path = root.to_path_buf();
    path.push("crashes");
    path.push(format!("crash_{iteration}.js"));
    path
}

async fn single_test(script_path: &str, profile: &str) {
    let source = fs::read_to_string(script_path).expect("failed to read test script");
    let ast = parse_js(source).expect("failed to parse test script");
    let minifier = Minifier;
    let mutated_ast = minifier.mutate(ast).expect("minification failed");
    // let mutated_code = generate_js(mutated_ast).expect("code generation failed");

    let mut pool = FuzzPool::new(1, &profiles::get_profile(profile).unwrap())
        .expect("failed to create fuzz pool");

    let mutators = get_ast_mutators();
    let root_path = std::path::PathBuf::from("single_test_output");
    let mut corpus_manager = CorpusManager::load(root_path.clone())
        .await
        .expect("failed to load corpus manager");

    for i in 0..10000 {
        if i % 1000 == 0 {
            println!("Single test iteration {}", i);
        }
        let mutated_ast = mutators[0]
            .mutate(mutated_ast.clone())
            .expect("numeric mutation failed");
        let mutated_code = generate_js(mutated_ast).expect("code generation failed");

        let mut result_rx = pool
            .schedule_job(mutated_code.clone())
            .await
            .expect("failed to schedule job");
        let job_result = result_rx
            .recv()
            .await
            .expect("failed to receive job result")
            .expect("job execution failed");

        if job_result.new_coverage {
            corpus_manager
                .add_entry(
                    &mutated_code,
                    job_result.edge_hits.clone(),
                    1.0,
                    0,
                )
                .await
                .expect("failed to add new corpus entry");
        }

        // println!(
        //     "Execution result: exit {}, signal {}, new_coverage {}, is_crash {}, is_timeout {}, edge_hits {:?}",
        //     job_result.status_code,
        //     job_result.signal,
        //     job_result.new_coverage,
        //     job_result.is_crash,
        //     job_result.is_timeout,
        //     job_result.edge_hits
        // );
    }
}
