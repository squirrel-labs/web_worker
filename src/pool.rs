//! A small module that's intended to provide an example of creating a pool of
//! web workers which can be used to execute `rayon`-style work.
use std::cell::RefCell;
use std::rc::Rc;

const STACK_ALIGN: usize = 1024 * 64;

/// The `WorkerPool`. This is a special type of thread pool that works on wasm and provide a way to
/// run work they way rayon does it.
pub struct WorkerPool {
    state: Rc<PoolState>,
    stack_size: u32,
    tls_size: u32,
}

struct PoolState {
    workers: RefCell<Vec<Worker>>,
}

struct Worker {
    available: i32,
    work_item: Option<Work>,
}

struct Work {
    func: Box<dyn FnOnce() + Send>,
}

extern "C" {
    fn spawn_pool_worker(id: u32, stack_top: u32, ptr: u32, tls: u32);
}

impl WorkerPool {
    /// Creates a new `WorkerPool` which immediately creates `initial` workers.
    ///
    /// The pool created here can be used over a long period of time, and it
    /// will be initially primed with `initial` workers. Currently workers are
    /// never released or gc'd until the whole pool is destroyed.
    ///
    /// # Errors
    ///
    /// Returns any error that may happen while a JS web worker is created and a
    /// message is sent to it.
    pub fn new(initial: usize, stack_size: u32, tls_size: u32) -> Result<WorkerPool, String> {
        let pool = WorkerPool {
            state: Rc::new(PoolState {
                workers: RefCell::new(Vec::with_capacity(initial)),
            }),
            stack_size,
            tls_size,
        };
        for _ in 0..initial {
            let worker = pool.spawn()?;
            pool.state.workers.borrow_mut().push(worker);
        }

        Ok(pool)
    }

    /// Unconditionally spawns a new worker
    ///
    /// The worker isn't registered with this `WorkerPool` but is capable of
    /// executing work for this wasm module.
    ///
    /// # Errors
    ///
    /// Returns any error that may happen while a JS web worker is created and a
    /// message is sent to it.
    fn spawn(&self) -> Result<Worker, String> {
        log::debug!("spawning new worker");

        let worker = Worker {
            available: 1,
            work_item: None,
        };

        unsafe {
            let stack_layout =
                core::alloc::Layout::from_size_align(self.stack_size as usize, STACK_ALIGN)
                    .unwrap();
            let stack_top = std::alloc::alloc(stack_layout);
            let tls_layout =
                core::alloc::Layout::from_size_align(self.tls_size as usize, 8).unwrap();
            let tls = std::alloc::alloc(tls_layout);
            spawn_pool_worker(
                self.state.workers.borrow().len() as u32,
                stack_top as u32,
                &worker as *const Worker as u32,
                tls as u32,
            );
        }
        // With a worker spun up send it the module/memory so it can start
        // instantiating the wasm module. Later it might receive further
        // messages about code to run on the wasm module.
        Ok(worker)
    }

    /// Fetches a worker from this pool, spawning one if necessary.
    ///
    /// This will attempt to pull an already-spawned web worker from our cache
    /// if one is available, otherwise it will spawn a new worker and return the
    /// newly spawned worker.
    ///
    /// # Safety
    /// This function is not thread safe!
    /// Never attempt to spawn more than one thread at a time!
    ///
    fn worker(&self) -> Result<usize, String> {
        let workers = self.state.workers.borrow();
        for (id, worker) in workers.iter().enumerate() {
            if worker.available == 1 {
                return Ok(id);
            }
        }

        self.state.workers.borrow_mut().push(self.spawn()?);
        Ok(self.state.workers.borrow().len() - 1)
    }

    /// Executes the work `f` in a web worker, spawning a web worker if
    /// necessary.
    ///
    /// This will acquire a web worker and then send the closure `f` to the
    /// worker to execute. The worker won't be usable for anything else while
    /// `f` is executing, and no callbacks are registered for when the worker
    /// finishes.
    ///
    /// # Errors
    ///
    /// Returns any error that may happen while a JS web worker is created and a
    /// message is sent to it.
    fn execute(&self, f: impl FnOnce() + Send + 'static) -> Result<(), String> {
        log::debug!("execute f");
        let worker = self.worker()?;
        let mut workers = self.state.workers.borrow_mut();
        assert_eq!(workers[worker].available, 1);
        log::debug!("got worker");
        let work = Work { func: Box::new(f) };
        workers[worker].available = 0;
        workers[worker].work_item = Some(work);
        log::debug!("init worker");
        unsafe {
            atomics::memory_atomic_notify(workers[worker].available as *mut i32, 1);
        }
        log::debug!("notified worker");
        Ok(())
    }
}

impl WorkerPool {
    /// Executes `f` in a web worker.
    ///
    /// This pool manages a set of web workers to draw from, and `f` will be
    /// spawned quickly into one if the worker is idle. If no idle workers are
    /// available then a new web worker will be spawned.
    ///
    /// Once `f` returns the worker assigned to `f` is automatically reclaimed
    /// by this `WorkerPool`. This method provides no method of learning when
    /// `f` completes, and for that you'll need to use `run_notify`.
    ///
    /// # Errors
    ///
    /// If an error happens while spawning a web worker or sending a message to
    /// a web worker, that error is returned.
    pub fn run(&self, f: impl FnOnce() + Send + 'static) -> Result<(), String> {
        self.execute(f)
    }
}

mod atomics {
    #[cfg(feature = "std_atomics")]
    pub use core::arch::wasm32::{memory_atomic_notify, memory_atomic_wait32};

    #[cfg(not(feature = "std_atomics"))]
    pub use llvm_intrinsic::*;

    #[cfg(not(feature = "std_atomics"))]
    mod llvm_intrinsic {
        extern "C" {
            #[link_name = "llvm.wasm.atomic.wait.i32"]
            fn llvm_atomic_wait_i32(ptr: *mut i32, exp: i32, timeout: i64) -> i32;
            #[link_name = "llvm.wasm.atomic.notify"]
            fn llvm_atomic_notify(ptr: *mut i32, cnt: i32) -> i32;
        }

        #[inline]
        pub unsafe fn memory_atomic_wait32(ptr: *mut i32, expression: i32, timeout_ns: i64) -> i32 {
            llvm_atomic_wait_i32(ptr, expression, timeout_ns)
        }
        #[inline]
        pub unsafe fn memory_atomic_notify(ptr: *mut i32, waiters: u32) -> u32 {
            llvm_atomic_notify(ptr, waiters as i32) as u32
        }
    }
}

/// Entry point invoked by `worker.js`
/// The worker.available has to be set prior to its invokation
/// # Safety
/// this function should be safe, as long as it is called with valid arguments
pub unsafe fn child_entry_point(ptr: u32) {
    log::debug!("entry reached");
    let mut worker = ptr as *mut Worker;

    loop {
        if (*worker).work_item.is_some() {
            log::debug!("got work");
            let work = Box::from_raw((*worker).work_item.as_mut().unwrap());
            log::debug!("got boxed work");
            (work.func)();
            log::debug!("finished work");
            (*worker).work_item = None;
            log::debug!("reset work");
        }
        (*worker).available = 1;
        atomics::memory_atomic_wait32(&mut (*worker).available as *mut i32, 1, -1);
    }
}
