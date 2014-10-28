extern crate raft_rs;


mod using_dumb_network {

    use std::io::timer::sleep;
    use std::time::duration::Duration;

    use raft_rs::intercommunication::{Intercommunication, DefaultIntercommunication, DumbPackage, Ack, start};

    #[test]
    fn sending_simple_ack() {
        let mut comm: DefaultIntercommunication = Intercommunication::new();

        let mut comm_1 = comm.register("host_1".to_string());
        let mut comm_2 = comm.register("host_2".to_string());

        let stop_comm = start(comm);

        comm_1.send_ack_to("host_2".to_string());

        match comm_2.listen_block_with_timeout() {
            Some(Ack(from, to)) => {
                assert_eq!(from, "host_1".to_string());
                assert_eq!(to, "host_2".to_string());
            },
            _ => fail!("No ack")
        }

        stop_comm.send(0);
    }
}
