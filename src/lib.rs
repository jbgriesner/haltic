//! This crate provides a wrapper type for making a long-running service loops cancellable
#![deny(missing_docs)]

use std::ops::Deref;
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc};
use std::thread::{self, JoinHandle};

/// Tells if main service should continue or not
pub enum LoopState {
    /// Accept more
    Continue,
    /// Break
    Break,
}

/// Main trait that should be implemented for a service to be cancellable
pub trait Cancellable {
    /// Error type for the value returned by inner loop
    type Error;

    /// Method that will be called successively
    fn for_each(&mut self) -> Result<LoopState, Self::Error>;

    /// Run in same thread
    fn run(&mut self) -> Result<(), Self::Error> {
        loop {
            match self.for_each() {
                Ok(LoopState::Continue) => {}
                Ok(LoopState::Break) => break,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Run in dedicated thread
    fn spawn(mut self) -> Handle<Self::Error>
    where
        Self: Send + Sized + 'static,
        Self::Error: Send + 'static,
    {
        let keep_running = Arc::new(AtomicBool::new(true));
        let j = {
            let keep_running = keep_running.clone();
            thread::spawn(move || {
                while keep_running.load(Ordering::SeqCst) {
                    match self.for_each() {
                        Ok(LoopState::Continue) => {}
                        Ok(LoopState::Break) => break,
                        Err(e) => return Err(e),
                    }
                }
                Ok(())
            })
        };
        Handle {
            canceller: Canceller { keep_running },
            executor: j,
        }
    }
}

/// Handle to manage service loop
///
/// You can use it to cancel the running loop at the next opportunity
/// or to wait for the loop to terminate
pub struct Handle<E> {
    canceller: Canceller,
    executor: JoinHandle<Result<(), E>>,
}

impl<E> Deref for Handle<E> {
    type Target = Canceller;
    fn deref(&self) -> &Self::Target {
        &self.canceller
    }
}

/// Get a thread safe access to the atomic bool
#[derive(Clone)]
pub struct Canceller {
    keep_running: Arc<AtomicBool>,
}

impl Canceller {
    /// Tells the service to stop ASAP
    /// This will *not* interrupt a currently executing service
    pub fn cancel(&self) {
        self.keep_running.store(false, Ordering::SeqCst);
    }
}

impl<E> Handle<E> {
    /// Get a canceller: another handle for cancelling the service loop
    pub fn canceller(&self) -> Canceller {
        self.canceller.clone()
    }

    /// Wait for the service loop to exit and return its result
    pub fn wait(self) -> Result<(), E> {
        match self.executor.join() {
            Ok(r) => r,
            Err(e) => {
                panic!("{:?}", e)
            }
        }
    }
}
