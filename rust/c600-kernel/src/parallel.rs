//! Independent pieces of work side by side, for charter E3's startup (a draft): the model loads read, unpack and
//! check their files on the machine's threads, and a shell starts its loads before it needs them.
//!
//! - Each job reads only its own inputs and returns a value, so a job's result does not depend on which thread runs it
//!   or when.
//! - [`map`] returns the results in the jobs' order, each with the panic its job raised in its place. A caller takes
//!   them in that order and stops at its first error, and [`taken`] re-raises a panic where it is taken. So a load
//!   answers as it does with the jobs run one after another: the same value and the same first error. A job after the
//!   first error still runs, and its result is dropped.
//! - [`map`]'s threads are scoped: every job has ended when it returns. A [`Background`] job ends before its handle is
//!   dropped.
//! - A thread that does not start leaves its jobs to the caller's thread.

use std::num::NonZeroUsize;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};

/// `job` of every index in `0..count`, in index order, run on up to the machine's available parallelism of threads
/// (the caller's among them).
pub fn map<T: Send>(count: usize, job: impl Fn(usize) -> T + Sync) -> Vec<thread::Result<T>> {
    let workers = thread::available_parallelism().map_or(1, NonZeroUsize::get).min(count);
    let next = AtomicUsize::new(0);
    let results: Vec<Mutex<Option<thread::Result<T>>>> = (0..count).map(|_| Mutex::new(None)).collect();
    let work = &|| loop {
        let index = next.fetch_add(1, Ordering::Relaxed);
        if index >= count {
            break;
        }
        let result = catch_unwind(AssertUnwindSafe(|| job(index)));
        *results[index].lock().unwrap_or_else(PoisonError::into_inner) = Some(result);
    };
    thread::scope(|scope| {
        for _ in 1..workers {
            if thread::Builder::new().spawn_scoped(scope, work).is_err() {
                break;
            }
        }
        work();
    });
    let ran = |slot: Mutex<Option<thread::Result<T>>>| slot.into_inner().unwrap_or_else(PoisonError::into_inner);
    results.into_iter().map(|slot| ran(slot).expect("every index is taken once")).collect()
}

/// A job's value; a job that panicked panics here with its payload.
pub fn taken<T>(result: thread::Result<T>) -> T {
    result.unwrap_or_else(|payload| resume_unwind(payload))
}

type Job<T> = Box<dyn FnOnce() -> T + Send>;

/// A job on a thread of its own, joined where its value is needed; or, when its thread did not start or it was
/// [`Background::deferred`], still to run where it is joined.
pub struct Background<T: Send + 'static> {
    thread: Option<JoinHandle<Option<thread::Result<T>>>>,
    job: Arc<Mutex<Option<Job<T>>>>,
}

impl<T: Send + 'static> Background<T> {
    /// Starts `job` on a new thread named `name`.
    pub fn start(name: &str, job: impl FnOnce() -> T + Send + 'static) -> Background<T> {
        let job: Arc<Mutex<Option<Job<T>>>> = Arc::new(Mutex::new(Some(Box::new(job))));
        let held = Arc::clone(&job);
        let run = move || {
            let job = held.lock().unwrap_or_else(PoisonError::into_inner).take();
            job.map(|job| catch_unwind(AssertUnwindSafe(job)))
        };
        Background { thread: thread::Builder::new().name(name.into()).spawn(run).ok(), job }
    }

    /// `job`, to run on the thread that joins it.
    pub fn deferred(job: impl FnOnce() -> T + Send + 'static) -> Background<T> {
        Background { thread: None, job: Arc::new(Mutex::new(Some(Box::new(job)))) }
    }

    /// The job's value. A job that panicked on its thread panics here with the same payload.
    pub fn join(mut self) -> T {
        let ran = self.thread.take().and_then(|thread| thread.join().unwrap_or_else(|payload| resume_unwind(payload)));
        if let Some(result) = ran {
            return taken(result);
        }
        let job = self.job.lock().unwrap_or_else(PoisonError::into_inner).take();
        job.expect("a job runs once")()
    }
}

impl<T: Send + 'static> Drop for Background<T> {
    /// Waits for a job still on its thread and drops its result.
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn results_come_back_in_job_order_with_each_panic_in_its_place() {
        let squares = map(100, |i| {
            if i == 41 {
                panic!("job 41");
            }
            i * i
        });
        assert_eq!(squares.len(), 100);
        for (i, result) in squares.into_iter().enumerate() {
            match result {
                Ok(value) => assert_eq!(value, i * i),
                Err(payload) => {
                    assert_eq!(i, 41);
                    assert_eq!(payload.downcast_ref::<&str>(), Some(&"job 41"));
                }
            }
        }
        assert!(map(0, |i| i).is_empty());
    }

    #[test]
    fn the_first_error_in_job_order_is_the_answer() {
        let first_error = |failing: &[usize]| -> Result<Vec<usize>, usize> {
            map(64, |i| if failing.contains(&i) { Err(i) } else { Ok(i) }).into_iter().map(taken).collect()
        };
        assert_eq!(first_error(&[]), Ok((0..64).collect()));
        assert_eq!(first_error(&[63, 7, 30]), Err(7));
    }

    #[test]
    fn a_background_job_gives_its_value_or_its_panic_where_it_is_joined() {
        let started = Background::start("c600-parallel-test", || thread::current().name().map(str::to_owned));
        assert_eq!(started.join().as_deref(), Some("c600-parallel-test"));
        let deferred = Background::deferred(|| thread::current().id());
        assert_eq!(deferred.join(), thread::current().id());
        let panicking = Background::start("c600-parallel-panic", || -> u8 { panic!("background") });
        let raised = catch_unwind(AssertUnwindSafe(|| panicking.join())).unwrap_err();
        assert_eq!(raised.downcast_ref::<&str>(), Some(&"background"));
        // A dropped job has ended: its side effect is visible after the drop.
        let done = Arc::new(AtomicUsize::new(0));
        let seen = Arc::clone(&done);
        drop(Background::start("c600-parallel-drop", move || {
            thread::sleep(std::time::Duration::from_millis(50));
            seen.store(1, Ordering::SeqCst);
        }));
        assert_eq!(done.load(Ordering::SeqCst), 1);
    }
}
