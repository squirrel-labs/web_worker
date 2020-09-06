#![feature(link_llvm_intrinsics)]
#![feature(stdsimd)]
//! Utilities to work with web workers and rayon.
use rayon::ThreadPool;

#[macro_use]
extern crate log;

mod pool;

pub use pool::*;

/// Creates a new `rayon::ThreadPool` with concurrency
pub fn default_thread_pool(
    concurrency: usize,
    stack_size: u32,
    tls_size: u32,
) -> Option<(ThreadPool, pool::WorkerPool)> {
    let worker_pool = pool::WorkerPool::new(concurrency, stack_size, tls_size);
    match worker_pool {
        Ok(pool) => Some((new_thread_pool(concurrency, &pool), pool)),
        Err(e) => {
            log::error!("Failed to create WorkerPool: {:?}", e);
            None
        }
    }
}

/// Creates a new `rayon::ThreadPool` with concurrency
pub fn set_global_thread_pool(
    concurrency: usize,
    stack_size: u32,
    tls_size: u32,
) -> Result<pool::WorkerPool, String> {
    let worker_pool = pool::WorkerPool::new(concurrency, stack_size, tls_size);
    match worker_pool {
        Ok(pool) => {
            create_global_threadpool(concurrency, &pool).map_err(|e| format!("{}", e));
            Ok(pool)
        }
        Err(e) => {
            log::error!("Failed to create WorkerPool: {:?}", e);
            Err(e)
        }
    }
}

/// Creates a new `rayon::ThreadPool` from the provided WorkerPool
/// and the concurrency value, which indicates the number of threads to use.
pub fn new_thread_pool(concurrency: usize, pool: &WorkerPool) -> ThreadPool {
    rayon::ThreadPoolBuilder::new()
        .num_threads(concurrency)
        .spawn_handler(|thread| Ok(pool.run(|| thread.run()).unwrap()))
        .build()
        .unwrap()
}

/// Creates a new `rayon::ThreadPool` from the provided WorkerPool (created in the javascript code)
/// and the concurrency value, which indicates the number of threads to use.
pub fn create_global_threadpool(
    concurrency: usize,
    pool: &WorkerPool,
) -> Result<(), rayon::ThreadPoolBuildError> {
    rayon::ThreadPoolBuilder::new()
        .num_threads(concurrency)
        .spawn_handler(|thread| Ok(pool.run(|| thread.run()).unwrap()))
        .build_global()
}
