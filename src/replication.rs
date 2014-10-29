
use std::io;

trait  Committable {
    fn persist(&self) -> io::IoResult < () >;
    fn commit(&self) -> io::IoResult < () >;
    fn discard(&self) -> io::IoResult < () >;
}

trait ReplicationLog {
    fn len(&self) -> u64;
    fn committed_offset(&self) -> u64;
    fn commit_upto(&self, new_committed_offset: u64) -> io::IoResult < () >;
    fn discard_downto(&self, new_committed_offset: u64) -> io::IoResult < () >;
}

