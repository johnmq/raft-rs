extern crate raft_rs;


mod using_dumb_network {

    use std::io::timer::sleep;
    use std::time::duration::Duration;

    use raft_rs::intercommunication::{Intercommunication, DefaultIntercommunication, DumbNetwork, DumbPackage, Ack};

    #[test]
    fn sending_simple_ack() {
        let mut network = DumbNetwork::new();

        let mut comm_1: DefaultIntercommunication = Intercommunication::new();
        let mut comm_2: DefaultIntercommunication = Intercommunication::new();

        comm_1.listens_to("host_1".to_string(), &mut network);
        comm_2.listens_to("host_2".to_string(), &mut network);

        network.start();

        comm_1.sends_to(&network);
        comm_2.sends_to(&network);

        comm_1.send_ack_to("host_1".to_string(), "host_2".to_string());


        match comm_2.listen_block_with_timeout() {
            Some(Ack(from, to)) => {
                assert_eq!(from, "host_1".to_string());
                assert_eq!(to, "host_2".to_string());
            },
            _ => fail!("No ack")
        }
    }
}
