use core::time;
use haltic::{self, Cancellable};
use std::io::Write;
use std::{io, net, thread};

struct Service(net::TcpListener);

impl haltic::Cancellable for Service {
    type Error = io::Error;

    fn for_each(&mut self) -> Result<haltic::LoopState, Self::Error> {
        let mut stream = self.0.accept()?.0;
        write!(stream, "hello!")?;
        Ok(haltic::LoopState::Continue)
    }
}

impl Service {
    fn new() -> Self {
        let listener = net::TcpListener::bind("127.0.0.1:6556").unwrap();
        Service(listener)
    }
}

fn main() {
    let mut s = Service::new();
    eprintln!("service started!");
    // s.run().unwrap();

    let h = s.spawn();

    let exit = h.canceller();

    thread::spawn(move || {
        thread::sleep(time::Duration::from_secs(10));
        eprintln!("service terminating!");

        exit.cancel();
    });
    h.wait().unwrap();
    eprintln!("service terminated!");
}
