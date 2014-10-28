use std::collections::HashMap;
use std::io::timer::sleep;
use std::time::duration::Duration;

use std::sync::{Arc, Mutex};

pub trait Intercommunication {
    fn new() -> Self;
    fn register(&mut self, host: String) -> Endpoint;
    fn receive(&mut self) -> Option < DumbPackage >;
    fn send(&mut self, recipient: String, package: DumbPackage);
}

pub struct DefaultIntercommunication {
    receiver: Receiver < DumbPackage >,
    sender: Sender < DumbPackage >,
    senders: HashMap < String, Sender < DumbPackage > >,
}

pub struct Endpoint {
    host: String,
    tx: Sender < DumbPackage >,
    rx: Receiver < DumbPackage >,
}

impl Intercommunication for DefaultIntercommunication {
    fn new() -> DefaultIntercommunication {
        let (tx, rx) = channel();

        DefaultIntercommunication {
            senders: HashMap::new(),
            sender: tx,
            receiver: rx,
        }
    }

    fn register(&mut self, host: String) -> Endpoint {
        let (tx, rx) = channel();

        self.senders.insert(host.clone(), tx);

        Endpoint {
            host: host,
            rx: rx,
            tx: self.sender.clone(),
        }
    }

    fn receive(&mut self) -> Option < DumbPackage > {
        match self.receiver.try_recv() {
            Ok(package) => Some(package),
            _ => None,
        }
    }

    fn send(&mut self, recipient: String, package: DumbPackage) {
        match self.senders.find(&recipient) {
            Some(tx) => {
                println!("Sent package to {}", recipient);
                tx.send(package);
            },
            None => println!("Unable to find recipient :("),
        }
    }
}

impl Endpoint {
    pub fn send_ack_to(&self, host: String) {
        self.tx.send(Ack(self.host.clone(), host));
    }

    pub fn listen_block_with_timeout(&self) -> Option < DumbPackage > {
        for _ in range(0, 10u) {
            match self.rx.try_recv() {
                Ok(result) => return Some(result),
                _ => {},
            }

            sleep(Duration::milliseconds(10));
        }

        None
    }
}

pub enum DumbPackage {
    // Ack("sender host", "recipient host")
    Ack(String, String),
}

pub fn start < T: Intercommunication + Send >(intercommunication: T) -> Sender < int > {
    let mutex = Arc::new(Mutex::new(intercommunication));
    let (exit_tx, exit_rx) = channel();

    spawn(proc() {
        loop {
            let mut intercommunication = mutex.lock();

            println!("waiting...");
            match intercommunication.receive() {
                Some(Ack(sender, recipient)) => {
                    println!("got something!");
                    intercommunication.send(recipient.clone(), Ack(sender.clone(), recipient))
                },
                None => ()
            }

            match exit_rx.try_recv() {
                Ok(_) => break,
                _ => (),
            }

            intercommunication.cond.signal();

            sleep(Duration::milliseconds(10));
        }
    });

    exit_tx
}
