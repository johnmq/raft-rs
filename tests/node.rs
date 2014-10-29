extern crate raft_rs;

mod helpers {
    use raft_rs::node::{Node};
    use raft_rs::intercommunication::{DefaultIntercommunication, Intercommunication, start};

    use std::io::timer::sleep;
    use std::time::duration::Duration;

    pub fn node() -> Node {
        Node::new()
    }

    pub fn comm() -> DefaultIntercommunication {
        Intercommunication::new()
    }

    pub fn start_comm < T: Intercommunication + Send >(comm: T) -> Sender < int > {
        start(comm)
    }

    pub fn stop_comm(stop_comm: Sender < int >) {
        stop_comm.send(0)
    }

    pub fn with_proper_comm(f: |DefaultIntercommunication| -> Sender < int >) {
        let sig = f(Intercommunication::new());
        sig.send(0);
    }

    pub fn sleep_ms(ms: i64) {
        sleep(Duration::milliseconds(ms));
    }
}

mod a_node_can_be_in_one_of_the_states {

    use raft_rs::node::{Node, Follower, Candidate, Leader};
    use helpers;

    #[test]
    fn follower_state_by_default() {
        let mut node = helpers::node();

        helpers::with_proper_comm(|mut comm| {
            node.start("john-follower", &mut comm);

            let sig = helpers::start_comm(comm);

            assert_eq!(Follower, node.state());

            node.stop();

            sig
        })
    }

    #[test]
    fn candidate_state() {
        let mut node = helpers::node();

        helpers::with_proper_comm(|mut comm| {
            node.start("john-candidate", &mut comm);

            let sig = helpers::start_comm(comm);

            node.forced_state(Candidate);

            assert_eq!(Candidate, node.state());

            node.stop();

            sig
        })
    }

    #[test]
    fn ledaer_state() {
        let mut node = helpers::node();

        helpers::with_proper_comm(|mut comm| {
            node.start("john-leader", &mut comm);

            let sig = helpers::start_comm(comm);

            node.forced_state(Leader);

            assert_eq!(Leader, node.state());

            node.stop();

            sig
        })
    }

}

mod who_is_the_leader {

    use helpers;

    #[test]
    fn nothing_if_no_leader() {
        let mut node = helpers::node();
        let mut other = helpers::node();

        helpers::with_proper_comm(|mut comm| {
            node.start("john", &mut comm);
            other.start("sarah", &mut comm);

            let sig = helpers::start_comm(comm);

            assert_eq!(None, other.fetch_leader());

            node.stop();
            other.stop();

            sig
        })
    }

    #[test]
    fn leader_host_if_there_is_a_leader() {
        let mut leader = helpers::node();
        let mut node = helpers::node();

        helpers::with_proper_comm(|mut comm| {
            leader.start("leader", &mut comm);
            node.start("john", &mut comm);

            let sig = helpers::start_comm(comm);

            node.force_follow("leader");

            assert_eq!("leader", node.fetch_leader().unwrap().host.as_slice());

            node.stop();
            leader.stop();

            sig
        })

    }

    #[test]
    fn leader_knows_all_nodes() {
        let mut leader = helpers::node();
        let mut follower_1 = helpers::node();
        let mut follower_2 = helpers::node();
        let mut follower_3 = helpers::node();

        helpers::with_proper_comm(|mut comm| {
            leader.start("leader", &mut comm);
            follower_1.start("john", &mut comm);
            follower_2.start("sarah", &mut comm);
            follower_3.start("james", &mut comm);

            let sig = helpers::start_comm(comm);

            follower_1.force_follow("leader");
            follower_2.force_follow("leader");
            follower_3.force_follow("leader");

            helpers::sleep_ms(100);

            let nodes = leader.fetch_nodes();
            let node_hosts: Vec < &str > = nodes.iter().map(|x| { x.host.as_slice() }).collect();
            assert!(node_hosts.contains(&"leader"));
            assert!(node_hosts.contains(&"john"));
            assert!(node_hosts.contains(&"sarah"));
            assert!(node_hosts.contains(&"james"));

            follower_1.stop();
            follower_2.stop();
            follower_3.stop();
            leader.stop();

            sig
        })
    }

    #[test]
    fn new_node_introduces_itself_to_leader() {
        let mut leader = helpers::node();
        let mut follower_1 = helpers::node();
        let mut node = helpers::node();

        helpers::with_proper_comm(|mut comm| {
            leader.start("leader", &mut comm);
            follower_1.start("john", &mut comm);
            node.start("sarah", &mut comm);

            let sig = helpers::start_comm(comm);

            follower_1.force_follow("leader");
            node.introduce("john");

            helpers::sleep_ms(100);

            let nodes = leader.fetch_nodes();
            let node_hosts: Vec < &str > = nodes.iter().map(|x| { x.host.as_slice() }).collect();
            assert!(node_hosts.contains(&"leader"));
            assert!(node_hosts.contains(&"john"));
            assert!(node_hosts.contains(&"sarah"));

            follower_1.stop();
            node.stop();
            leader.stop();

            sig
        })
    }

    #[test]
    fn leader_propagates_node_list_changes_to_its_followers() {
        let mut leader = helpers::node();
        let mut follower_1 = helpers::node();
        let mut node = helpers::node();

        helpers::with_proper_comm(|mut comm| {
            leader.start("leader", &mut comm);
            follower_1.start("john", &mut comm);
            node.start("sarah", &mut comm);

            let sig = helpers::start_comm(comm);

            follower_1.force_follow("leader");
            node.introduce("john");

            helpers::sleep_ms(100);

            let nodes = follower_1.fetch_nodes();
            let node_hosts: Vec < &str > = nodes.iter().map(|x| { x.host.as_slice() }).collect();
            assert!(node_hosts.contains(&"leader"));
            assert!(node_hosts.contains(&"john"));
            assert!(node_hosts.contains(&"sarah"));

            follower_1.stop();
            node.stop();
            leader.stop();

            sig
        })
    }

}
