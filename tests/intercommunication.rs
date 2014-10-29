extern crate raft_rs;


mod using_dumb_network {

    use std::io::timer::sleep;
    use std::time::duration::Duration;

    use raft_rs::intercommunication::{Intercommunication, DefaultIntercommunication, Package, Ack, Pack, start};

    #[test]
    fn sending_simple_ack() {
        let mut comm: DefaultIntercommunication = Intercommunication::new();

        let mut comm_1 = comm.register("host_1".to_string());
        let mut comm_2 = comm.register("host_2".to_string());

        let stop_comm = start(comm);

        comm_1.send("host_2".to_string(), Ack);

        match comm_2.listen_block_with_timeout() {
            Some(Pack(from, to, Ack)) => {
                assert_eq!(from, "host_1".to_string());
                assert_eq!(to, "host_2".to_string());
            },
            _ => fail!("No ack")
        }

        stop_comm.send(0);
    }
}
