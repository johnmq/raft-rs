use std::io;
use std::io::timer::sleep;
use std::time::duration::Duration;
use std::comm::Disconnected;

pub trait Committable {

}

pub trait Receivable {

}

pub trait Queriable {

}

pub trait LogPersistence < T: Committable > {
    fn commit(&self, entry: T) -> io::IoResult < () >;
}

#[deriving(Show, PartialEq)]
pub enum DefaultReceivable {
    ReceivableInt(int),
}

impl Receivable for DefaultReceivable {

}

pub struct DefaultQuery;

impl Queriable for DefaultQuery {

}

pub trait ReplicationLog < T: Committable, Q: Queriable, R: Receivable > {
    fn new() -> Self;

    fn len(&self) -> uint;
    fn committed_offset(&self) -> uint;
    fn commit_upto(&mut self, new_committed_offset: uint) -> io::IoResult < () >;
    fn discard_downto(&mut self, new_len: uint) -> io::IoResult < () >;

    fn enqueue(&mut self, entry: T) -> io::IoResult < uint >;
    fn query_persistance(&mut self, query: Q, respond_to: Sender < R >);
}

#[deriving(Clone, Show)]
pub enum DefaultCommand {
    TestSet(int),
    TestAdd(int),
}

#[deriving(Clone, Show)]
pub struct DefaultCommandContainer {
    pub command: DefaultCommand,
}

pub struct DefaultPersistence {
    tx: Sender < DefaultCommandContainer >,
    pub rx: Receiver < int >,
}

impl DefaultPersistence {
    pub fn start() -> DefaultPersistence {
        let (tx, rx) = channel();
        let (value_tx, value_rx) = channel();

        let me = DefaultPersistence {
            tx: tx,
            rx: value_rx,
        };

        spawn(proc() {
            let mut value = 0i;

            loop {
                match rx.try_recv() {
                    Ok(command) => {
                        match command {
                            DefaultCommandContainer{ command: TestSet(x) } => value = x,
                            DefaultCommandContainer{ command: TestAdd(dx) } => value += dx,
                        }

                        value_tx.send(value);
                    }
                    Err(Disconnected) => break,
                    _ => ()
                }

                sleep(Duration::milliseconds(10));
            }
        });

        me
    }
}

pub struct DefaultReplicationLog {
    log: Vec < DefaultCommandContainer >,
    offset: uint,
    pub persistence: DefaultPersistence,
}

impl Committable for DefaultCommandContainer {

}

impl LogPersistence < DefaultCommandContainer > for DefaultPersistence {
    fn commit(&self, entry: DefaultCommandContainer) -> io::IoResult < () > {
        self.tx.send(entry);
        Ok(())
    }
}

impl ReplicationLog < DefaultCommandContainer, DefaultQuery, DefaultReceivable > for DefaultReplicationLog {
    fn new() -> DefaultReplicationLog {
        DefaultReplicationLog {
            log: vec![],
            offset: 0,
            persistence: DefaultPersistence::start(),
        }
    }

    fn len(&self) -> uint {
        self.log.len()
    }

    fn committed_offset(&self) -> uint {
        self.offset
    }

    fn commit_upto(&mut self, new_committed_offset: uint) -> io::IoResult < () > {
        while self.offset < new_committed_offset && self.offset < self.len() {
            self.persistence.commit(self.log[self.offset]);
            self.offset += 1;
        }

        Ok(())
    }

    fn discard_downto(&mut self, new_len: uint) -> io::IoResult < () > {
        while self.len() > new_len && self.len() > self.offset {
            self.log.pop();
        }
        Ok(())
    }

    fn enqueue(&mut self, entry: DefaultCommandContainer) -> io::IoResult < uint > {
        self.log.push(entry);
        Ok(self.log.len())
    }

    fn query_persistance(&mut self, query: DefaultQuery, respond_to: Sender < DefaultReceivable >) {
        match self.persistence.rx.try_recv() {
            Ok(value) => respond_to.send(ReceivableInt(value)),
            _ => (),
        }
    }
}
