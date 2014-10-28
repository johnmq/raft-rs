use std::collections::HashMap;
use std::io::timer::sleep;
use std::time::duration::Duration;
use std::comm::{Disconnected, TryRecvError};

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

    pub fn send_leader_query_to(&self, host: String) {
        self.tx.send(LeaderQuery(self.host.clone(), host));
    }

    pub fn send_leader_query_response_to(&self, host: String, leader_host: Option < String >) {
        self.tx.send(LeaderQueryResponse(self.host.clone(), host, leader_host));
    }

    pub fn listen_block_with_timeout(&self) -> Option < DumbPackage > {
        for _ in range(0, 10u) {
            match self.listen() {
                Some(x) => return Some(x),
                _ => ()
            }

            sleep(Duration::milliseconds(10));
        }

        None
    }

    pub fn listen(&self) -> Option < DumbPackage > {
        match self.rx.try_recv() {
            Ok(result) => Some(result),
            _ => None,
        }
    }
}

pub enum DumbPackage {
    // Ack("sender host", "recipient host")
    Ack(String, String),

    // LeaderQuery("requester", "responder")
    LeaderQuery(String, String),

    // LeaderQueryResponse("responder", "requester", "leader_host")
    LeaderQueryResponse(String, String, Option < String >),
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
                Some(LeaderQuery(requester, responder)) => {
                    intercommunication.send(responder.clone(), LeaderQuery(requester.clone(), responder))
                },
                Some(LeaderQueryResponse(responder, requester, leader_host)) => {
                    intercommunication.send(
                        requester.clone(),
                        LeaderQueryResponse(responder, requester, leader_host),
                        );
                },
                None => ()
            }

            match exit_rx.try_recv() {
                Ok(_) | Err(Disconnected) => break,
                _ => (),
            }

            intercommunication.cond.signal();

            sleep(Duration::milliseconds(10));
        }
    });

    exit_tx
}
