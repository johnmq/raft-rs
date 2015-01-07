use std::collections::HashMap;
use std::io::timer::sleep;
use std::time::duration::Duration;
use std::comm::Disconnected;

use std::fmt::Show;

use std::sync::{Arc, Mutex};

use std::task::TaskBuilder;

use serialize::json;

use super::replication::Committable;

pub trait Intercommunication < T: Committable + Send + Show > {
    fn new() -> Self;
    fn register(&mut self, host: String) -> Endpoint < T >;
    fn receive(&mut self) -> Option < Package < T > >;
    fn send(&mut self, recipient: String, package: Package < T >);
    fn is_debug(&self) -> bool;
}

pub struct DefaultIntercommunication < T: Committable + Send > {
    receiver: Receiver < Package < T > >,
    sender: Sender < Package < T > >,
    senders: HashMap < String, Sender < Package < T > > >,

    pub is_debug: bool,
}

pub struct Endpoint < T: Committable + Send > {
    pub host: String,
    pub tx: Sender < Package < T > >,
    pub rx: Receiver < Package < T > >,
}

#[derive(Encodable, Decodable, Show, Clone)]
pub struct AppendLog < T: Committable > {
    pub committed_offset: uint,
    pub node_list: Vec < String >,
    pub enqueue: Option < AppendLogEntry < T > >,
}

unsafe impl Send for AppendLog { }

#[derive(Encodable, Decodable, Show, Clone)]
pub struct AppendLogEntry < T: Committable > {
    pub offset: uint,
    pub entry: T,
}

unsafe impl Send for AppendLogEntry { }

impl < T: Committable + Send + Show > Intercommunication < T > for DefaultIntercommunication < T > {
    fn new() -> DefaultIntercommunication < T > {
        let (tx, rx) = channel();

        DefaultIntercommunication {
            senders: HashMap::new(),
            sender: tx,
            receiver: rx,
            is_debug: false,
        }
    }

    fn register(&mut self, host: String) -> Endpoint < T > {
        let (tx, rx) = channel();

        self.senders.insert(host.clone(), tx);

        Endpoint {
            host: host,
            rx: rx,
            tx: self.sender.clone(),
        }
    }

    fn receive(&mut self) -> Option < Package < T > > {
        match self.receiver.try_recv() {
            Ok(package) => Some(package),
            _ => None,
        }
    }

    fn send(&mut self, recipient: String, package: Package < T >) {
        match self.senders.find(&recipient) {
            Some(tx) => {
                // be more careful at sending
                match tx.send_opt(package) {
                    Err(_) => (),
                    _ => (),
                }
            },
            None => (),
        }
    }

    fn is_debug(&self) -> bool {
        self.is_debug
    }
}

impl < T: Committable + Send > Endpoint < T > {
    pub fn send (&self, host: String, package: PackageDetails < T >) {
        self.tx.send(Pack(self.host.clone(), host, package));
    }

    pub fn listen_block_with_timeout(&self) -> Option < Package < T > > {
        for _ in range(0, 10u) {
            match self.listen() {
                Some(x) => return Some(x),
                _ => ()
            }

            sleep(Duration::milliseconds(2));
        }

        None
    }

    pub fn listen(&self) -> Option < Package < T > > {
        match self.rx.try_recv() {
            Ok(result) => Some(result),
            _ => None,
        }
    }
}

#[derive(Encodable, Decodable, Show, Clone)]
pub enum PackageDetails < T: Committable + Send > {
    Ack,

    LeaderQuery,

    // LeaderQueryResponse(leader_host)
    LeaderQueryResponse(Option < String >),

    // AppendQuery(log)
    AppendQuery(AppendLog < T >),

    // Persisted(entry_offset)
    Persisted(uint),

    // RequestVote(term
    RequestVote(uint),

    // Vote(term)
    Vote(uint),
}

#[derive(Encodable, Decodable, Show, Clone)]
pub enum Package < T: Committable + Send > {
    // Pack(from, to, package)
    Pack(String, String, PackageDetails < T >),
}

pub fn start < T: Committable + Send + Clone + Show, I: Intercommunication < T > + Send >(intercommunication: I) -> Sender < int > {
    let mutex = Arc::new(Mutex::new(intercommunication));
    let (exit_tx, exit_rx) = channel();

    TaskBuilder::new().named("intercommunication").spawn(move || {
        loop {
            let mut intercommunication = mutex.lock();

            match intercommunication.receive() {
                Some(Pack(from, to, package)) => {
                    if intercommunication.is_debug() {
                        println!("sent package {} to host {} from {}", package, to, from);
                    }

                    intercommunication.send(to.clone(), Pack(from, to, package));
                },
                None => ()
            }

            match exit_rx.try_recv() {
                Ok(_) => break,
                _ => (),
            }

            intercommunication.cond.signal();

            sleep(Duration::milliseconds(2));
        }
    });

    exit_tx
}
