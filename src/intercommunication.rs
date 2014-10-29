use std::collections::HashMap;
use std::io::timer::sleep;
use std::time::duration::Duration;
use std::comm::{Disconnected, TryRecvError};

use std::sync::{Arc, Mutex};

use std::task::TaskBuilder;

pub trait Intercommunication {
    fn new() -> Self;
    fn register(&mut self, host: String) -> Endpoint;
    fn receive(&mut self) -> Option < Package >;
    fn send(&mut self, recipient: String, package: Package);
}

pub struct DefaultIntercommunication {
    receiver: Receiver < Package >,
    sender: Sender < Package >,
    senders: HashMap < String, Sender < Package > >,
}

pub struct Endpoint {
    host: String,
    tx: Sender < Package >,
    rx: Receiver < Package >,
}

#[deriving(Clone)]
pub struct AppendLog {
    pub node_list: Vec < String >,
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

    fn receive(&mut self) -> Option < Package > {
        match self.receiver.try_recv() {
            Ok(package) => Some(package),
            _ => None,
        }
    }

    fn send(&mut self, recipient: String, package: Package) {
        match self.senders.find(&recipient) {
            Some(tx) => {
                tx.send(package);
            },
            None => (),
        }
    }
}

impl Endpoint {
    pub fn send(&self, host: String, package: PackageDetails) {
        self.tx.send(Pack(self.host.clone(), host, package));
    }

    pub fn listen_block_with_timeout(&self) -> Option < Package > {
        for _ in range(0, 10u) {
            match self.listen() {
                Some(x) => return Some(x),
                _ => ()
            }

            sleep(Duration::milliseconds(10));
        }

        None
    }

    pub fn listen(&self) -> Option < Package > {
        match self.rx.try_recv() {
            Ok(result) => Some(result),
            _ => None,
        }
    }
}

pub enum PackageDetails {
    // Ack
    Ack,

    // LeaderQuery
    LeaderQuery,

    // LeaderQueryResponse(leader_host)
    LeaderQueryResponse(Option < String >),

    // AppendQuery(log)
    AppendQuery(AppendLog),
}

pub enum Package {
    // Pack(from, to, package)
    Pack(String, String, PackageDetails),
}

pub fn start < T: Intercommunication + Send >(intercommunication: T) -> Sender < int > {
    let mutex = Arc::new(Mutex::new(intercommunication));
    let (exit_tx, exit_rx) = channel();

    TaskBuilder::new().named("intercommunication").spawn(proc() {
        loop {
            let mut intercommunication = mutex.lock();

            match intercommunication.receive() {
                Some(Pack(from, to, package)) => {
                    intercommunication.send(to.clone(), Pack(from, to, package));
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
