use std::io;
use std::io::timer::sleep;
use std::time::duration::Duration;
use std::comm::Disconnected;

pub trait Committable {
    fn commit(&self) -> io::IoResult < () >;
}

pub trait ReplicationLog < T: Committable > {
    fn new() -> Self;

    fn len(&self) -> uint;
    fn committed_offset(&self) -> uint;
    fn commit_upto(&mut self, new_committed_offset: uint) -> io::IoResult < () >;
    fn discard_downto(&mut self, new_len: uint) -> io::IoResult < () >;

    fn enqueue (&mut self, entry: T) -> io::IoResult < () >;
}

pub enum DefaultCommand {
    TestSet(int),
    TestAdd(int),
}

pub struct DefaultCommandContainer {
    pub command: DefaultCommand,
    pub tx: Sender < DefaultCommand >,
}

pub struct DefaultPersistence;

impl DefaultPersistence {
    pub fn start() -> (Sender < DefaultCommand >, Receiver < int >) {
        let (tx, rx) = channel();
        let (value_tx, value_rx) = channel();

        spawn(proc() {
            let mut value = 0i;

            loop {
                match rx.try_recv() {
                    Ok(command) => {
                        match command {
                            TestSet(x) => value = x,
                            TestAdd(dx) => value += dx,
                        }

                        value_tx.send(value);
                    }
                    Err(Disconnected) => break,
                    _ => ()
                }

                sleep(Duration::milliseconds(10));
            }
        });

        (tx, value_rx)
    }
}

pub struct DefaultReplicationLog {
    log: Vec < DefaultCommandContainer >,
    offset: uint,
}

impl Committable for DefaultCommandContainer {
    fn commit(&self) -> io::IoResult < () > {
        self.tx.send(self.command);
        Ok(())
    }
}

impl ReplicationLog < DefaultCommandContainer > for DefaultReplicationLog {
    fn new() -> DefaultReplicationLog {
        DefaultReplicationLog {
            log: vec![],
            offset: 0,
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
            self.log[self.offset].commit();
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

    fn enqueue (&mut self, entry: DefaultCommandContainer) -> io::IoResult < () > {
        self.log.push(entry);
        Ok(())
    }
}
