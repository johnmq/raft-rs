use std::io;
use std::io::timer::sleep;
use std::time::duration::Duration;
use std::sync::mpsc::TryRecvError;

pub trait Committable {

}

pub trait Receivable {

}

pub trait Queriable {

}

pub trait LogPersistence < T: Committable > {
    fn commit(&self, entry: T) -> io::IoResult < () >;
}

#[derive(Show, PartialEq)]
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

    fn autocommit_if_safe(&mut self, majority_size: uint);

    fn enqueue(&mut self, entry: T) -> io::IoResult < uint >;
    fn persisted(&mut self, offset: uint, node: String) -> io::IoResult < uint >;
    fn query_persistance(&mut self, query: Q, respond_to: Sender < R >);
}

#[derive(Clone, Show, PartialEq)]
pub enum DefaultCommand {
    TestSet(int),
    TestAdd(int),
}

#[derive(Clone, Show, PartialEq)]
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

        spawn(move || {
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

                sleep(Duration::milliseconds(2));
            }
        });

        me
    }
}

pub struct DefaultReplicationLog {
    log: Vec < DefaultCommandContainer >,
    persisted_by: Vec < DefaultPersistedBy >,
    offset: uint,
    pub persistence: DefaultPersistence,
}

struct DefaultPersistedBy {
    node_list: Vec < String >,
}

impl Committable for DefaultCommandContainer {

}

impl LogPersistence < DefaultCommandContainer > for DefaultPersistence {
    fn commit(&self, entry: DefaultCommandContainer) -> io::IoResult < () > {
        self.tx.send(entry);
        Ok(())
    }
}

impl DefaultReplicationLog {
    fn safe_to_commit(&mut self, offset: uint, majority_size: uint) -> bool {
        self.persisted_by.len() > offset &&
            self.persisted_by[offset].node_list.len() >= majority_size
    }
}

impl ReplicationLog < DefaultCommandContainer, DefaultQuery, DefaultReceivable > for DefaultReplicationLog {
    fn new() -> DefaultReplicationLog {
        DefaultReplicationLog {
            log: vec![],
            persisted_by: vec![],
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

    fn autocommit_if_safe(&mut self, majority_size: uint) {
        let target_offset = self.committed_offset();
        let safe = self.safe_to_commit(target_offset, majority_size);

        if safe {
            self.commit_upto(target_offset + 1);
        }
    }

    fn enqueue(&mut self, entry: DefaultCommandContainer) -> io::IoResult < uint > {
        self.log.push(entry);
        self.persisted_by.push(DefaultPersistedBy { node_list: vec![] });
        Ok(self.log.len() - 1)
    }

    fn persisted(&mut self, offset: uint, host: String) -> io::IoResult < uint > {
        if !self.persisted_by[offset].node_list.contains(&host) {
            self.persisted_by[offset].node_list.push(host);
        }
        Ok(offset)
    }

    fn query_persistance(&mut self, query: DefaultQuery, respond_to: Sender < DefaultReceivable >) {
        match self.persistence.rx.try_recv() {
            Ok(value) => respond_to.send(ReceivableInt(value)),
            _ => (),
        }
    }
}
