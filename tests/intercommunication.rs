extern crate raft_rs;


mod using_dumb_network {

    use raft_rs::intercommunication::{Intercommunication, DefaultIntercommunication, Ack, Pack, start, AppendLog, AppendQuery, AppendLogEntry};
    use raft_rs::replication::{DefaultCommandContainer, TestSet};

    #[test]
    fn sending_simple_ack() {
        let mut comm: DefaultIntercommunication < DefaultCommandContainer > = Intercommunication::new();

        let comm_1 = comm.register("host_1".to_string());
        let comm_2 = comm.register("host_2".to_string());

        let stop_comm = start(comm);

        comm_1.send("host_2".to_string(), Ack);

        match comm_2.listen_block_with_timeout() {
            Some(Pack(from, to, Ack)) => {
                assert_eq!(from, "host_1".to_string());
                assert_eq!(to, "host_2".to_string());
            },
            _ => panic!("No ack"),
        }

        stop_comm.send(0);
    }

    #[test]
    fn sending_append_query_with_command() {
        let mut comm: DefaultIntercommunication < DefaultCommandContainer > = Intercommunication::new();

        let comm_1 = comm.register("host_1".to_string());
        let comm_2 = comm.register("host_2".to_string());

        let stop_comm = start(comm);

        comm_1.send("host_2".to_string(), AppendQuery(AppendLog {
            committed_offset: 0,
            node_list: vec![],
            enqueue: Some(AppendLogEntry {
                offset: 1,
                entry: DefaultCommandContainer { command: TestSet(2) },
            }),
        }));

        match comm_2.listen_block_with_timeout() {
            Some(Pack(from, to, AppendQuery(AppendLog { committed_offset, node_list, enqueue }))) => {
                assert_eq!(from, "host_1".to_string());
                assert_eq!(to, "host_2".to_string());
                assert_eq!(node_list, vec![]);
                assert_eq!(committed_offset, 0);
                match enqueue {
                    Some(AppendLogEntry { offset, entry }) => {
                        assert_eq!(offset, 1);
                        assert_eq!(entry, DefaultCommandContainer { command: TestSet(2) });
                    },
                    _ => panic!("No enqueue"),
                }
            },
            _ => panic!("No append query"),
        }

        stop_comm.send(0);
    }
}
