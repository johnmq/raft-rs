use std::collections::HashMap;
use std::io::timer::sleep;
use std::time::duration::Duration;

pub trait Intercommunication {
    fn new() -> Self;
    fn send_ack_to(&self, String, String);
    fn try_listen(&mut self) -> Option < DumbPackage >;
    fn listen_block_with_timeout(&mut self) -> Option < DumbPackage >;
}

pub struct DefaultIntercommunication {
    sender: Option < Sender < DumbPackage > >,
    receiver: Option < Receiver < DumbPackage > >,
}

impl Intercommunication for DefaultIntercommunication {
    fn new() -> DefaultIntercommunication {
        DefaultIntercommunication { sender: None, receiver: None }
    }

    fn send_ack_to(&self, from: String, to: String) {
        match self.sender {
            Some(ref tx) => tx.send(Ack(from, to)),
            _ => ()
        }
    }

    fn try_listen(&mut self) -> Option < DumbPackage > {
        match self.receiver {
            Some(ref rx) => match rx.try_recv() {
                Ok(message) => Some(message),
                _ => None
            },
            _ => None
        }
    }

    fn listen_block_with_timeout(&mut self) -> Option < DumbPackage > {
        for i in range(0u, 10u) {
            match self.listen() {
                Some(result) => return Some(result),
                _ => (),
            }

            sleep(Duration::milliseconds(10));
        }

        None
    }
}

impl DefaultIntercommunication {
    pub fn listens_to(&mut self, as_host: String, network: &mut DumbNetwork) {
        self.receiver = Some(network.register(as_host));
    }

    pub fn sends_to(&mut self, network: &DumbNetwork) {
        self.sender = network.sender.clone();
    }
}

pub enum DumbPackage {
    // Ack("sender host", "recipient host")
    Ack(String, String),
}

pub struct DumbNetwork {
    sender: Option < Sender < DumbPackage > >,
    senders: HashMap < String, Sender < DumbPackage > >,
}

struct DumbNetworkHandler {
    receiver: Receiver < DumbPackage >,
    senders: HashMap < String, Sender < DumbPackage > >,
}

impl DumbNetwork {
    pub fn new() -> DumbNetwork {

        DumbNetwork { sender: None, senders: HashMap::new() }
    }

    fn register(&mut self, as_host: String) -> Receiver < DumbPackage > {
        let (tx, rx) = channel();

        self.senders.insert(as_host, tx);
        rx
    }

    pub fn start(&mut self) {
        let (tx, rx) = channel();

        self.sender = Some(tx);

        let me = DumbNetworkHandler {
            receiver: rx,
            senders: self.senders.clone()
        };

        spawn(proc() {
            loop {
                match me.receiver.recv() {
                    Ack(from, to) => match me.senders.find(&to) {
                        Some(tx) => tx.send(Ack(from, to)),
                        _ => ()
                    },
                }
            }
        })
    }
}
