use libc::{c_int, c_void};
use std::fs::File;
use std::io::{self, Read, Write};
use std::mem::{self, MaybeUninit};
use std::os::unix::fs::FileExt;
use std::os::unix::io::{FromRawFd, RawFd};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};

#[repr(C)]
pub struct ShmemData {
    pub num_edges: u32,
    pub edges: [u8; 0],
}

#[repr(C)]
pub struct CovContext {
    pub id: c_int,
    pub should_track_edges: c_int,
    pub virgin_bits: *mut u8,
    pub crash_bits: *mut u8,
    pub num_edges: u32,
    pub bitmap_size: u32,
    pub found_edges: u32,
    pub shmem: *mut ShmemData,
    pub edge_count: *mut u32,
}

#[repr(C)]
pub struct EdgeSet {
    pub count: u32,
    pub edge_indices: *mut u32,
}

// These contexts own process-local shared memory handles, so moving them across
// threads is safe as long as we uphold the unique access invariants ourselves
unsafe impl Send for CovContext {}

unsafe extern "C" {
    pub fn cov_initialize(ctx: *mut CovContext) -> c_int;
    pub fn cov_finish_initialization(ctx: *mut CovContext, track_edges: c_int);
    pub fn cov_shutdown(ctx: *mut CovContext);
    pub fn cov_clear_bitmap(ctx: *mut CovContext);
    pub fn cov_evaluate(ctx: *mut CovContext, new_edges: *mut EdgeSet) -> c_int;
}
