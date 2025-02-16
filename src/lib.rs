use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc};
use std::thread::{self, JoinHandle};

pub enum LoopState {
    Continue,
    Break,
}

pub trait Cancellable {
    type Error;

    fn for_each(&mut self) -> Result<LoopState, Self::Error>;

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
            keep_running,
            executor: j,
        }
    }
}

pub struct Handle<E> {
    keep_running: Arc<AtomicBool>,
    executor: JoinHandle<Result<(), E>>,
}

#[derive(Clone)]
pub struct Canceller {
    keep_running: Arc<AtomicBool>,
}

impl Canceller {
    pub fn cancel(&self) {
        self.keep_running.store(false, Ordering::SeqCst);
    }
}

impl<E> Handle<E> {
    pub fn canceller(&self) -> Canceller {
        Canceller {
            keep_running: self.keep_running.clone(),
        }
    }

    pub fn wait(self) -> Result<(), E> {
        match self.executor.join() {
            Ok(r) => r,
            Err(e) => {
                panic!("{:?}", e)
            }
        }
    }

    pub fn cancel(&self) {
        self.keep_running.store(false, Ordering::SeqCst);
    }
}
