extern crate raft_rs;

mod a_node_can_be_in_one_of_the_states {

    use raft_rs::node::{Node, Follower, Candidate, Leader};

    #[test]
    fn follower_state_by_default() {
        let mut node = Node::new();

        node.start("john-follower");

        assert_eq!(Follower, node.state());

        node.stop();
    }

    #[test]
    fn candidate_state() {
        let mut node = Node::new();

        node.start("john-candidate");

        node.forced_state(Candidate);

        assert_eq!(Candidate, node.state());

        node.stop();
    }

    #[test]
    fn ledaer_state() {
        let mut node = Node::new();

        node.start("john-leader");

        node.forced_state(Leader);

        assert_eq!(Leader, node.state());

        node.stop();
    }

}

mod who_is_the_leader {

    use raft_rs::node::{Node};

    #[test]
    fn nothing_if_no_leader() {
        let mut node = Node::new();
        let mut other = Node::new();

        node.start("john");
        other.start("sarah");

        assert_eq!(None, other.fetch_leader());

        node.stop();
    }

    #[test]
    fn leader_host_if_there_is_a_leader() {
        let mut leader = Node::new();
        let mut node = Node::new();

        leader.start("leader");
        node.start("john");

        node.force_follow("leader");

        assert_eq!("leader", node.fetch_leader().unwrap().host.as_slice());

        node.stop();
    }

    #[test]
    fn leader_knows_all_his_followers() {
        let mut leader = Node::new();
        let mut follower_1 = Node::new();
        let mut follower_2 = Node::new();
        let mut follower_3 = Node::new();

        leader.start("leader");
        follower_1.start("john");
        follower_2.start("sarah");
        follower_3.start("james");

        follower_1.force_follow("leader");
        follower_2.force_follow("leader");
        follower_3.force_follow("leader");

        //let followers = leader.fetch_followers();
        // PENDING
    }

}
