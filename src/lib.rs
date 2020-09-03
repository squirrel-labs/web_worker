#![feature(link_llvm_intrinsics)]
#![feature(stdsimd)]
//! Utilities to work with web workers and rayon.
use rayon::ThreadPool;

#[macro_use]
extern crate log;

mod pool;

pub use pool::*;

/// Creates a new `rayon::ThreadPool` with default concurrency value provided by the browser
pub fn default_thread_pool(concurrency: usize) -> Option<ThreadPool> {
    let worker_pool = pool::WorkerPool::new(concurrency, 1024 * 64);
    match worker_pool {
        Ok(pool) => Some(new_thread_pool(concurrency, &pool)),
        Err(e) => {
            log::error!("Failed to create WorkerPool: {:?}", e);
            None
        }
    }
}

/// Creates a new `rayon::ThreadPool` from the provided WorkerPool (created in the javascript code)
/// and the concurrency value, which indicates the number of threads to use.
pub fn new_thread_pool(concurrency: usize, pool: &WorkerPool) -> ThreadPool {
    rayon::ThreadPoolBuilder::new()
        .num_threads(concurrency)
        .spawn_handler(|thread| Ok(pool.run(|| thread.run()).unwrap()))
        .build()
        .unwrap()
}
