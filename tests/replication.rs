extern crate raft_rs;

use raft_rs::replication::{DefaultCommandContainer, DefaultPersistence, DefaultReplicationLog, ReplicationLog, TestAdd, TestSet, Committable};

#[test]
fn default_command_container_implements_commit() {
    let (tx, value_rx) = DefaultPersistence::start();

    DefaultCommandContainer { command: TestSet(3), tx: tx.clone() }.commit();
    assert_eq!(3, value_rx.recv());

    DefaultCommandContainer { command: TestAdd(5), tx: tx.clone() }.commit();
    assert_eq!(8, value_rx.recv());

    DefaultCommandContainer { command: TestSet(13), tx: tx.clone() }.commit();
    assert_eq!(13, value_rx.recv());
}

#[test]
fn enqueue_and_commit_command() {
    let (tx, value_rx) = DefaultPersistence::start();
    let mut log: DefaultReplicationLog = ReplicationLog::new();

    log.enqueue(DefaultCommandContainer { command: TestSet(3), tx: tx.clone() });
    log.enqueue(DefaultCommandContainer { command: TestAdd(5), tx: tx.clone() });
    log.enqueue(DefaultCommandContainer { command: TestSet(21), tx: tx.clone() });

    assert_eq!(3, log.len());
    assert_eq!(0, log.committed_offset());

    log.commit_upto(2);

    assert_eq!(2, log.committed_offset());
    assert_eq!(3, value_rx.recv());
    assert_eq!(8, value_rx.recv());
}

#[test]
fn enqueue_and_discard_commands() {
    let (tx, value_rx) = DefaultPersistence::start();
    let mut log: DefaultReplicationLog = ReplicationLog::new();

    log.enqueue(DefaultCommandContainer { command: TestSet(3), tx: tx.clone() });
    log.enqueue(DefaultCommandContainer { command: TestAdd(5), tx: tx.clone() });
    log.enqueue(DefaultCommandContainer { command: TestSet(21), tx: tx.clone() });

    assert_eq!(3, log.len());
    assert_eq!(0, log.committed_offset());

    log.discard_downto(1);

    assert_eq!(1, log.len());
    assert_eq!(0, log.committed_offset());

    log.commit_upto(1);

    assert_eq!(1, log.committed_offset());
    assert_eq!(3, value_rx.recv());
}
