extern crate raft_rs;

use raft_rs::replication::{DefaultCommandContainer, DefaultPersistence, DefaultReplicationLog, ReplicationLog, TestAdd, TestSet, Committable, LogPersistence};

#[test]
fn default_persistance_implements_commit() {
    let persistence = DefaultPersistence::start();

    persistence.commit(DefaultCommandContainer { command: TestSet(3) });
    assert_eq!(3, persistence.rx.recv());

    persistence.commit(DefaultCommandContainer { command: TestAdd(5) });
    assert_eq!(8, persistence.rx.recv());

    persistence.commit(DefaultCommandContainer { command: TestSet(13) });
    assert_eq!(13, persistence.rx.recv());
}

#[test]
fn enqueue_and_commit_command() {
    let mut log: DefaultReplicationLog = ReplicationLog::new();

    log.enqueue(DefaultCommandContainer { command: TestSet(3) });
    log.enqueue(DefaultCommandContainer { command: TestAdd(5) });
    log.enqueue(DefaultCommandContainer { command: TestSet(21) });

    assert_eq!(3, log.len());
    assert_eq!(0, log.committed_offset());

    log.commit_upto(2);

    assert_eq!(2, log.committed_offset());
    assert_eq!(3, log.persistence.rx.recv());
    assert_eq!(8, log.persistence.rx.recv());
}

#[test]
fn enqueue_and_discard_commands() {
    let mut log: DefaultReplicationLog = ReplicationLog::new();

    log.enqueue(DefaultCommandContainer { command: TestSet(3) });
    log.enqueue(DefaultCommandContainer { command: TestAdd(5) });
    log.enqueue(DefaultCommandContainer { command: TestSet(21) });

    assert_eq!(3, log.len());
    assert_eq!(0, log.committed_offset());

    log.discard_downto(1);

    assert_eq!(1, log.len());
    assert_eq!(0, log.committed_offset());

    log.commit_upto(1);

    assert_eq!(1, log.committed_offset());
    assert_eq!(3, log.persistence.rx.recv());
}
