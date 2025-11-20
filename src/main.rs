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
use crate::mutators::{ManagedMutator, get_ast_mutators, get_mutator_by_name, get_weighted_mutator_choice};
use crate::parsing::parser::{generate_js, parse_js};
use crate::profiles::profile::JsEngineProfile;
use crate::runner::pool::{FuzzPool, JobResult};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, help = "Path to output progress output directory")]
    output_dir: PathBuf,
    
    // overwrite existing corpus files and start from scratch
    #[arg(short, long, action=clap::ArgAction::SetTrue, help = "Overwrite existing corpus directory if it exists and restart progress")]
    overwrite: Option<bool>,
    // if overwrite is true, we need an initial corpus directory to read from
    #[arg(
        short,
        long,
        requires = "overwrite",
        help = "Path to initial corpus directory to ingest when starting from scratch"
    )]
    initial_corpus: Option<PathBuf>,
    
    // resume from existing corpus
    #[arg(short, long, action=clap::ArgAction::SetTrue, help = "Resume progress from existing corpus directory")]
    resume: Option<bool>,
    // the profile to use
    #[arg(short, long, help = "Fuzzing profile to use")]
    profile: String,
    // number of workers
    #[arg(
        short,
        long,
        default_value_t = 1,
        help = "Number of worker processes to use"
    )]
    workers: usize,
    // single test mode
    #[arg(long, help = "DEBUG: Run tests with a single specified input file")]
    single_test: Option<String>,
    // mutator test mode
    #[arg(
        long,
        help = "DEBUG: Run a specified mutator on a given input file and output the result"
    )]
    mutator_test: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let output_dir = args.output_dir.clone();
    
    if let Some(test_path) = args.single_test.as_deref() {
        single_test(test_path, &args.profile).await;
        return Ok(());
    }
    if let Some(mutator) = args.mutator_test.as_deref() {
        let mutator = get_mutator_by_name(mutator).expect("unknown mutator");
        mutator_test("test.js", mutator, &args.profile).await;
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
                // eprintln!("Failed to parse {:?}: {:?}", path, err);
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
            
            let exec_time = exec_start.elapsed();
            // if job_result.status_code != 0 {
            //     skipped_clone.fetch_add(1, Ordering::Relaxed);
            //     return;
            // }
            
            let reward = compute_reward(&job_result);
            let mut manager = corpus_manager_clone.lock().await;
            match manager
            .add_entry(
                &new_code,
                job_result.edge_hits.clone(),
                reward,
                exec_time,
                job_result.is_timeout,
            )
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
            break; // TODO: remove this break
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
    let mut total_iterations: u64 = 0;
    let mut handles = vec![];
    let mut start = Instant::now();
    loop {
        iteration += 1;
        total_iterations += 1;
        
        // if iteration >= 50_000 {
        //     println!("Reached maximum iterations; exiting fuzz loop.");
        //     break;
        // }
        
        let corpus_manager = corpus_manager.clone();
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
                corpus_manager.lock().await.remove_entry(selection.id).await.unwrap_or(());
                continue;
            }
        };
        
        let mutator = get_weighted_mutator_choice(mutators);
        
        let mutated_ast = match mutator.is_splicer() {
            true => {
                let donor = {
                    let mut mgr = corpus_manager.lock().await;
                    mgr.get_random_script().await
                };
                let donor = match donor {
                    Ok(Some(script)) => script,
                    Ok(None) => {
                        eprintln!("No donor script available for splicing");
                        continue;
                    }
                    Err(err) => {
                        eprintln!("Failed to get donor script for splicing: {:?}", err);
                        continue;
                    }
                };
                mutator.splice(&seed_script, &donor).expect("splicing failed")
            },
            false => {
                mutator.mutate(seed_script).expect("mutation failed")
            }
        };
        
        
        let mutated_code = match generate_js(mutated_ast) {
            Ok(code) => code,
            Err(err) => {
                eprintln!("Code generation failed: {:?}", err);
                continue;
            }
        };
        
        let mut result_rx = match pool.schedule_job(mutated_code.clone()).await {
            Ok(rx) => rx,
            Err(err) => {
                eprintln!("Failed to schedule job: {:?}", err);
                continue;
            }
        };
        let handle = tokio::task::spawn(async move {
            let job_result = match result_rx.recv().await {
                Some(Ok(res)) => res,
                Some(Err(err)) => {
                    eprintln!("Worker execution error: {:?}", err);
                    return;
                }
                None => {
                    eprintln!("Worker dropped execution result");
                    return;
                }
            };
            let reward = compute_reward(&job_result);
            
            mutator.record_reward(reward);
            if job_result.is_timeout || job_result.status_code != 0 {
                mutator.record_invalid();
            }
            if job_result.is_timeout {
                mutator.record_timeout();
            }
            {
                let mut mgr = corpus_manager.lock().await;
                // Update stats for the original corpus entry.
                mgr.record_result(selection.id, reward, job_result.exec_time_ms)
                .await
                .unwrap_or(());
                // Persist timeouts into corpus/timeouts for later triage.
                if job_result.is_timeout {
                    let _ = mgr
                    .add_entry(
                        &mutated_code,
                        Vec::new(),
                        0.0,
                        job_result.exec_time_ms,
                        true,
                    )
                    .await;
                }
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
                persist_crash(crash_path.as_path(), &mutated_code).await.unwrap_or_else(|err| {
                    eprintln!("Failed to persist crash repro: {:?}", err);
                });
                return;
            }
            // println!(
            //     "[iter {}] exec_time: {} ms, reward: {}, new_coverage: {}, exit_code: {}, signal: {}, path: {:?}",
            //     iteration,
            //     exec_time_ms,
            //     reward,
            //     job_result.new_coverage,
            //     job_result.status_code,
            //     job_result.signal,
            //     selection.path
            // );
            if job_result.new_coverage && job_result.status_code == 0 && !job_result.is_timeout {
                let add_result = {
                    let mut mgr = corpus_manager.lock().await;
                    mgr.add_entry(
                        &mutated_code,
                        job_result.edge_hits.clone(),
                        reward.max(0.0),
                        job_result.exec_time_ms,
                        job_result.is_timeout,
                    )
                    .await
                };
                if let Ok(Some(entry)) = add_result {
                    // println!(
                    //     "[iter {}] new corpus entry {} ({} bytes)",
                    //     iteration, entry.id, entry.size_bytes
                    // );
                }
            }
        });
        handles.push(handle);
        if handles.len() >= 10000 {
            for handle in handles.drain(..) {
                handle.await.expect("fuzz loop task failed");
            }
            pool.print_pool_stats().await;
            println!("executed {} iterations", total_iterations);
            let elapsed = start.elapsed();
            println!(
                "[{:?}] Execs/sec: {:.2}",
                chrono::Utc::now().timestamp(),
                (iteration) as f64 / elapsed.as_secs_f64()
            );
            start = Instant::now();
            iteration = 0;
            for mutator in mutators {
                let stats = mutator.stats_snapshot();
                let success_rate = if stats.uses == 0 {
                    0.0
                } else {
                    (stats.uses - stats.invalid_count) as f64 / stats.uses as f64 * 100.0
                };
                println!(
                    "[mut] {}: success rate: {:.2}%, reward: {:.2}, mean: {:.4}, uses: {}, timeouts: {}, invalids: {}",
                    mutator.name(),
                    success_rate,
                    stats.total_reward,
                    stats.mean_reward,
                    stats.uses,
                    stats.timeout_count,
                    stats.invalid_count
                );
            }
        }
    }
    
    for handle in handles {
        handle.await.expect("fuzz loop task failed");
    }
    let elapsed = start.elapsed();
    println!("Fuzz loop completed in {:?}", elapsed);
    println!("Total iterations: {}", total_iterations);
    println!(
        "Execs/sec: {:.2}",
        (iteration) as f64 / elapsed.as_secs_f64()
    );
    
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
    
    let mut pool = FuzzPool::new(14, &profiles::get_profile(profile).unwrap())
    .expect("failed to create fuzz pool");
    
    let mutators = get_ast_mutators();
    let root_path = std::path::PathBuf::from("single_test_output");
    
    let (task_tx, mut task_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
    
    let runner = tokio::task::spawn(async move {
        let mut handles = vec![];
        let corpus_manager = CorpusManager::load(root_path.clone())
        .await
        .expect("failed to load corpus manager");
        let corpus_manager = Arc::new(Mutex::new(corpus_manager));
        
        let mut total_iters = 0;
        let start = Instant::now();
        while let Some(js_code) = task_rx.recv().await {
            total_iters += 1;
            let mut result_rx = pool
            .schedule_job(js_code.clone())
            .await
            .expect("failed to schedule job");
            
            let corpus_manager = Arc::clone(&corpus_manager);
            let handle = tokio::spawn(async move {
                let job_result = result_rx
                .recv()
                .await
                .expect("failed to receive job result")
                .expect("job execution failed");
                if job_result.new_coverage && job_result.status_code == 0 {
                    corpus_manager
                    .lock()
                    .await
                    .add_entry(
                        &js_code,
                        job_result.edge_hits.clone(),
                        1.0,
                        job_result.exec_time_ms,
                        job_result.is_timeout,
                    )
                    .await
                    .expect("failed to add new corpus entry");
                }
                if job_result.is_crash {
                    println!("Code: {:?}", js_code);
                    panic!("How did we find a crash?");
                }
                // if job_result.is_timeout || job_result.status_code != 0 {
                //     mutator.record_invalid();
                // }
            });
            handles.push(handle);
            
            if handles.len() >= 10000 {
                for handle in handles.drain(..) {
                    handle.await.expect("single test task failed");
                }
                pool.print_pool_stats().await;
                println!("executed {} iterations", total_iters);
                let elapsed = start.elapsed();
                println!(
                    "Execs/sec: {:.2}",
                    (total_iters) as f64 / elapsed.as_secs_f64()
                );
            }
        }
        for handle in handles.drain(..) {
            handle.await.expect("single test task failed");
        }
        let elapsed = start.elapsed();
        println!("Single test completed in {:?}", elapsed);
        println!("Total iterations: {}", total_iters);
        println!(
            "Execs/sec: {:.2}",
            (total_iters) as f64 / elapsed.as_secs_f64()
        );
    });
    
    
    let start = Instant::now();
    const TOTAL_ITERATIONS: usize = 30000;
    
    for i in 0..TOTAL_ITERATIONS {
        for mutator in &mutators {
            let mutated_ast = mutator
            .mutate(mutated_ast.clone())
            .expect("numeric mutation failed");
            let mutated_code = generate_js(mutated_ast).expect("code generation failed");
            
            task_tx
            .send(mutated_code)
            .expect("failed to send code to runner");
        }
        
    }
    println!(
        "Submitted {} iterations in {:?}",
        TOTAL_ITERATIONS,
        start.elapsed()
    );
    drop(task_tx);
    
    runner.await.expect("runner task failed");
    // for mutator in &mutators {
    // let stats = mutator.stats_snapshot();
    //     let success_rate = if stats.uses == 0 {
    //         0.0
    //     } else {
    //         (stats.uses - stats.invalid_count) as f64 / stats.uses as f64 * 100.0
    //     };
    //     println!(
    //         "Mutator {}: success rate: {:.2}%",
    //         mutator.name(),
    //         success_rate
    //     );
    // }
}

async fn mutator_test(script_path: &str, mutator: Arc<ManagedMutator>, profile: &str) {
    let source = fs::read_to_string(script_path).expect("failed to read test script");
    let ast = parse_js(source).expect("failed to parse test script");
    let mutated_ast = mutator.mutate(ast).expect("mutation failed");
    let mutated_code = generate_js(mutated_ast).expect("code generation failed");
    fs::write("test_out.js", &mutated_code).expect("failed to write mutated code");
    
    let profile = profiles::get_profile(profile).expect("unknown profile");
    let mut pool = FuzzPool::new(1, &profile).expect("failed to create fuzz pool");
    let mut result_rx = pool
    .schedule_job(mutated_code.clone())
    .await
    .expect("failed to schedule job");
    let job_result = result_rx
    .recv()
    .await
    .expect("failed to receive job result")
    .expect("job execution failed");
    println!(
        "Mutator test result: exit {}, signal {}, timeout {}, new coverage {}",
        job_result.status_code, job_result.signal, job_result.is_timeout, job_result.new_coverage
    );
}



#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test(flavor = "multi_thread")]
    async fn run_single_script_test() {
        let script_path = "corpus/crashes/seed_3137.js";
        let profile = "v8";
        let profile = profiles::get_profile(profile).expect("unknown profile");
        let mut pool = FuzzPool::new(1, &profile).expect("failed to create fuzz pool");
        let source = fs::read_to_string(script_path).expect("failed to read test script");
        let ast = parse_js(source).expect("failed to parse test script");
        let minifier = Minifier;
        let mutated_ast = minifier.mutate(ast).expect("minification failed");
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
        println!(
            "Single script test result: exit {}, signal {}, timeout {}, new coverage {}",
            job_result.status_code, job_result.signal, job_result.is_timeout, job_result.new_coverage
        );
    }
}
