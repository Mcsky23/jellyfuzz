use rand::Rng;
use tokio::sync::{Mutex, mpsc};
use tokio::fs as async_fs;
use tokio::task::JoinHandle;
use std::sync::Arc;

use crate::{compute_reward, corpus};
use crate::corpus::CorpusManager;
use crate::mutators::{ManagedMutator, get_random_splicer, get_weighted_ast_mutator_choice};
use crate::parsing::parser::{generate_js, parse_js};
use crate::runner::pool::{FuzzPool, JobResult};

pub async fn fuzz_sample(
    corpus_manager: Arc<Mutex<CorpusManager>>,
    mutators: &[Arc<ManagedMutator>],
    handles: &mut Vec<JoinHandle<()>>, 
    pool: &mut FuzzPool
) {
    // pick a random sample from the corpus
    let (seed, id) = {
        let mut mgr = corpus_manager.lock().await;
        let sample = mgr.pick_random()
        .expect("should always be able to pick sample");
        let source = async_fs::read(&sample.path).await
        .expect("should be able to read corpus sample");
        let source = String::from_utf8(source).unwrap_or(String::new());
        (parse_js(source), sample.id)
    };
    if seed.is_err() {
        return;
    }
    let mut seed = seed.unwrap();
    let mut rng = rand::rng();
    
    // execute mutation on the sample
    // TODO: make the number consecutive mutations an option rather than hardcoding it
    for _ in 0..10 {
        // with a random probability splice
        let mutator = get_weighted_ast_mutator_choice(mutators);
        let mutated_seed = mutator.mutate(seed.clone());
        if mutated_seed.is_err() {
            continue;
        }
        let mutated_seed = mutated_seed.unwrap();
        
        // execute the mutation
        let mutated_source = generate_js(mutated_seed.clone());
        if mutated_source.is_err() {
            continue;
        }
        let mutated_source = mutated_source.unwrap();
        
        // schedule execution
        let result_rx = match pool.schedule_job(mutated_source.clone()).await {
            Ok(rx) => rx,
            Err(err) => {
                eprintln!("Failed to schedule job: {:?}", err);
                continue;
            }
        };
        handles.push(tokio::task::spawn(result_handler(result_rx, mutator, corpus_manager.clone(), id, mutated_source)));
        seed = mutated_seed;
        
        // with a probability also splice
        if rng.random_bool(0.2) {
            let splicer = get_random_splicer(mutators);
            if !splicer.is_none() {
                let splicer = splicer.unwrap();
                let donor = {
                    let mgr = corpus_manager.lock().await;
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
                let mutated_seed = splicer.splice(&seed, &donor).expect("splicing failed");
                let mutated_source = generate_js(mutated_seed.clone());
                if mutated_source.is_err() {
                    continue;
                }
                let mutated_source = mutated_source.unwrap();
                let result_rx = match pool.schedule_job(mutated_source.clone()).await {
                    Ok(rx) => rx,
                    Err(err) => {
                        eprintln!("Failed to schedule job: {:?}", err);
                        continue;
                    }
                };
                handles.push(tokio::task::spawn(result_handler(result_rx, splicer, corpus_manager.clone(), id, mutated_source)));
            }
        }
    }
}

async fn result_handler(
    mut result_rx: mpsc::Receiver<Result<JobResult, anyhow::Error>>,
    mutator: Arc<ManagedMutator>,
    corpus_manager: Arc<Mutex<CorpusManager>>,
    id: u64,
    mutated_source: Vec<u8>
) {
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
        mutator.record_invalid(job_result.is_timeout);
    }
    {
        let mut mgr = corpus_manager.lock().await;
        let _ = mgr.record_result(id, reward, job_result.exec_time_ms)
        .await;
        
        if job_result.is_crash {
            println!(
                "Crash detected (exit {}, signal {}); reward {}",
                job_result.status_code, job_result.signal, reward
            );
            let _ = mgr.persist_crash(&mutated_source);
        }
        
        if job_result.new_coverage && job_result.status_code == 0 && !job_result.is_timeout {
            let _ = mgr.add_entry(
                &mutated_source, 
                job_result.edge_hits.clone(), 
                reward, 
                job_result.exec_time_ms, 
                job_result.is_timeout
            ).await;
        }
    }
}