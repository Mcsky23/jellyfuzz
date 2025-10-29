mod runner;
mod profiles;

use libc::{c_int, c_void};
use std::mem::{self, MaybeUninit};
use runner::{process::FuzzProcess, coverage::*};

const D8_PATH: &str= "/home/mcsky/Desktop/CTF/v8_research2/v8/out/fuzzbuild/d8";

fn main() {
    let mut cov_ctx = unsafe {
        let mut ctx = MaybeUninit::<CovContext>::zeroed().assume_init();
        ctx.id = 0;
        if cov_initialize(&mut ctx) != 0 {
            panic!("cov_initialize failred");
        }
        ctx
    };
    let shm_id = format!("shm_id_{}_{}", std::process::id(), cov_ctx.id);

    let mut target = FuzzProcess::spawn(&[D8_PATH, &"--fuzzing"], &shm_id)
        .expect("failed to spawn target");
    target.handshake()
        .expect("handshake failed");

    unsafe {
        cov_finish_initialization(&mut cov_ctx, 0);
    }

    let script = br#"
        fuzzilli("FUZZILLI_PRINT", "hello from reprl");
        Math.sin(0.1);
    "#;

    unsafe { cov_clear_bitmap(&mut cov_ctx); }
    let rc = target.execute(script)
        .expect("execution failed");
    println!("engine returned {}", rc);

    let mut edges = EdgeSet {
        count: 0,
        edge_indices: std::ptr::null_mut(),
    };
    let new_cov = unsafe { cov_evaluate(&mut cov_ctx, &mut edges) };
    if new_cov == 1 && !edges.edge_indices.is_null() {
        let slice =
            unsafe { std::slice::from_raw_parts(edges.edge_indices, edges.count as usize) };
        // println!("new edges: {:?}", slice);
        unsafe { libc::free(edges.edge_indices as *mut c_void) };
    }

    unsafe { cov_shutdown(&mut cov_ctx); }    
}
