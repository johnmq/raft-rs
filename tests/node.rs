extern crate raft_rs;

mod helpers {
    use raft_rs::node::{Node};
    use raft_rs::intercommunication::{DefaultIntercommunication, Intercommunication, start};
    use raft_rs::replication::{DefaultReplicationLog, ReplicationLog, DefaultCommandContainer, DefaultReceivable, DefaultQuery};

    use std::{rand, num};
    use std::io::timer::sleep;
    use std::time::duration::Duration;

    pub fn node() -> Node < DefaultCommandContainer, DefaultQuery, DefaultReceivable > {
        Node::new()
    }

    pub fn comm() -> DefaultIntercommunication < DefaultCommandContainer > {
        Intercommunication::new()
    }

    pub fn start_comm < T: Intercommunication < DefaultCommandContainer > + Send >(comm: T) -> Sender < int > {
        start(comm)
    }

    pub fn stop_comm(stop_comm: Sender < int >) {
        stop_comm.send(0)
    }

    pub fn with_proper_comm(f: |DefaultIntercommunication < DefaultCommandContainer >| -> Sender < int >) {
        let sig = f(Intercommunication::new());
        sig.send(0);
    }

    pub fn sleep_ms(ms: i64) {
        sleep(Duration::milliseconds(ms));
    }

    pub fn node_start(node: &mut Node < DefaultCommandContainer, DefaultQuery, DefaultReceivable >, host: &str, comm: &mut DefaultIntercommunication < DefaultCommandContainer >) {
        let mut log: DefaultReplicationLog = ReplicationLog::new();
        let election_timeout = 150 + num::abs(rand::random::< i64 >() % 150);
        node.start(host, comm, log, Duration::milliseconds(election_timeout));
    }
}

mod a_node_can_be_in_one_of_the_states {

    use raft_rs::node::{Node, Follower, Candidate, Leader};
    use helpers;

    #[test]
    fn follower_state_by_default() {
        let mut node = helpers::node();

        helpers::with_proper_comm(|mut comm| {
            helpers::node_start(&mut node, "john-follower", &mut comm);

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
            helpers::node_start(&mut node, "john-candidate", &mut comm);

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
            helpers::node_start(&mut node, "john-leader", &mut comm);

            let sig = helpers::start_comm(comm);

            node.forced_state(Leader);

            assert_eq!(Leader, node.state());

            node.stop();

            sig
        })
    }

}

mod discovery {

    use helpers;
    use raft_rs::node::{Leader};

    #[test]
    fn nothing_if_no_leader() {
        let mut node = helpers::node();
        let mut other = helpers::node();

        helpers::with_proper_comm(|mut comm| {
            helpers::node_start(&mut node, "john", &mut comm);
            helpers::node_start(&mut other, "sarah", &mut comm);

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
            helpers::node_start(&mut leader, "leader", &mut comm);
            helpers::node_start(&mut node, "john", &mut comm);

            let sig = helpers::start_comm(comm);

            node.force_follow("leader");

            assert_eq!("leader", node.fetch_leader().unwrap().host.as_slice());

            leader.stop();
            node.stop();

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
            helpers::node_start(&mut leader, "leader", &mut comm);
            helpers::node_start(&mut follower_1, "john", &mut comm);
            helpers::node_start(&mut follower_2, "sarah", &mut comm);
            helpers::node_start(&mut follower_3, "james", &mut comm);

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

            leader.stop();
            follower_1.stop();
            follower_2.stop();
            follower_3.stop();

            sig
        })
    }

    #[test]
    fn new_node_introduces_itself_to_leader() {
        let mut leader = helpers::node();
        let mut follower_1 = helpers::node();
        let mut node = helpers::node();

        helpers::with_proper_comm(|mut comm| {
            helpers::node_start(&mut leader, "leader", &mut comm);
            helpers::node_start(&mut follower_1, "john", &mut comm);
            helpers::node_start(&mut node, "sarah", &mut comm);

            let sig = helpers::start_comm(comm);

            follower_1.force_follow("leader");
            node.introduce("john");

            helpers::sleep_ms(100);

            let nodes = leader.fetch_nodes();
            let node_hosts: Vec < &str > = nodes.iter().map(|x| { x.host.as_slice() }).collect();
            assert!(node_hosts.contains(&"leader"));
            assert!(node_hosts.contains(&"john"));
            assert!(node_hosts.contains(&"sarah"));

            leader.stop();
            follower_1.stop();
            node.stop();

            sig
        })
    }

    #[test]
    fn leader_propagates_node_list_changes_to_its_followers() {
        let mut leader = helpers::node();
        let mut follower_1 = helpers::node();
        let mut node = helpers::node();

        helpers::with_proper_comm(|mut comm| {
            helpers::node_start(&mut leader, "leader", &mut comm);
            helpers::node_start(&mut follower_1, "john", &mut comm);
            helpers::node_start(&mut node, "sarah", &mut comm);

            let sig = helpers::start_comm(comm);

            leader.forced_state(Leader);
            follower_1.force_follow("leader");
            node.introduce("john");

            helpers::sleep_ms(200);

            let nodes = follower_1.fetch_nodes();
            let node_hosts: Vec < &str > = nodes.iter().map(|x| { x.host.as_slice() }).collect();
            assert!(node_hosts.contains(&"leader"));
            assert!(node_hosts.contains(&"john"));
            assert!(node_hosts.contains(&"sarah"));

            leader.stop();
            follower_1.stop();
            node.stop();

            sig
        })
    }

}

mod election {

    use helpers;
    use raft_rs::node::{Candidate, Leader, Follower, State};

    #[test]
    fn follower_not_getting_append_logs_becomes_candidate() {
        let mut node = helpers::node();

        helpers::with_proper_comm(|mut comm| {
            helpers::node_start(&mut node, "john", &mut comm);

            let sig = helpers::start_comm(comm);

            helpers::sleep_ms(350);

            let state = node.state();
            assert!(Candidate == state || Leader == state);

            node.stop();

            sig
        })
    }

    #[test]
    fn follower_getting_append_logs_stays_being_follower() {
        let mut leader = helpers::node();
        let mut node = helpers::node();

        helpers::with_proper_comm(|mut comm| {
            helpers::node_start(&mut leader, "leader", &mut comm);
            helpers::node_start(&mut node, "john", &mut comm);

            let sig = helpers::start_comm(comm);

            leader.forced_state(Leader);
            node.introduce("leader");
            node.force_follow("leader");

            helpers::sleep_ms(350);

            let state = node.state();
            assert_eq!(Follower, state);

            leader.stop();
            node.stop();

            sig
        })
    }

    #[test]
    fn candidate_votes_for_himself_and_asks_other_nodes_for_votes() {
        let mut node_1 = helpers::node();
        let mut node_2 = helpers::node();
        let mut node_3 = helpers::node();

        helpers::with_proper_comm(|mut comm| {
            helpers::node_start(&mut node_1, "john", &mut comm);
            helpers::node_start(&mut node_2, "duck", &mut comm);
            helpers::node_start(&mut node_3, "sarah", &mut comm);

            let sig = helpers::start_comm(comm);

            node_1.introduce("duck");
            node_3.introduce("duck");

            helpers::sleep_ms(100);

            node_1.forced_state(Follower);
            node_2.forced_state(Candidate);
            node_3.forced_state(Follower);

            helpers::sleep_ms(250);

            let state = node_2.state();
            assert_eq!(Leader, state);

            let leader = node_1.fetch_leader();
            assert_eq!("duck".to_string(), leader.unwrap().host);

            node_1.stop();
            node_2.stop();
            node_3.stop();

            sig
        })
    }

    #[test]
    fn concurrent_candidates_decide_who_leader_is() {
        let mut node_1 = helpers::node();
        let mut node_2 = helpers::node();
        let mut node_3 = helpers::node();

        helpers::with_proper_comm(|mut comm| {
            helpers::node_start(&mut node_1, "john", &mut comm);
            helpers::node_start(&mut node_2, "duck", &mut comm);
            helpers::node_start(&mut node_3, "sarah", &mut comm);

            let sig = helpers::start_comm(comm);

            node_1.introduce("duck");
            node_3.introduce("duck");

            node_1.introduce("sarah");
            node_2.introduce("sarah");

            node_2.introduce("john");
            node_3.introduce("john");

            helpers::sleep_ms(100);

            node_1.forced_state(Follower);
            node_2.forced_state(Candidate);
            node_3.forced_state(Candidate);

            helpers::sleep_ms(700);

            let states = vec![node_1.state(), node_2.state(), node_3.state()];
            assert!(states.contains(&Leader));

            node_1.stop();
            node_2.stop();
            node_3.stop();

            sig
        })
    }

    #[test]
    fn all_nodes_become_candidates_at_once() {
        let mut node_1 = helpers::node();
        let mut node_2 = helpers::node();
        let mut node_3 = helpers::node();

        helpers::with_proper_comm(|mut comm| {
            helpers::node_start(&mut node_1, "john", &mut comm);
            helpers::node_start(&mut node_2, "duck", &mut comm);
            helpers::node_start(&mut node_3, "sarah", &mut comm);

            let sig = helpers::start_comm(comm);

            node_1.introduce("duck");
            node_3.introduce("duck");

            node_1.introduce("sarah");
            node_2.introduce("sarah");

            node_2.introduce("john");
            node_3.introduce("john");

            helpers::sleep_ms(100);

            node_1.forced_state(Candidate);
            node_2.forced_state(Candidate);
            node_3.forced_state(Candidate);

            helpers::sleep_ms(1150);

            let states = vec![node_1.state(), node_2.state(), node_3.state()];
            assert!(states.contains(&Leader));

            node_1.stop();
            node_2.stop();
            node_3.stop();

            sig
        })
    }

    #[test]
    fn one_node_in_a_cluster_becomes_leader() {
        let mut node = helpers::node();

        helpers::with_proper_comm(|mut comm| {
            helpers::node_start(&mut node, "john", &mut comm);

            let sig = helpers::start_comm(comm);

            helpers::sleep_ms(350);

            let state = node.state();
            assert_eq!(Leader, state);

            node.stop();

            sig
        })
    }

    #[test]
    fn majority_is_enough_to_decide_on_election() {
        let mut node_1 = helpers::node();
        let mut node_2 = helpers::node();
        let mut node_3 = helpers::node();

        helpers::with_proper_comm(|mut comm| {
            helpers::node_start(&mut node_1, "john", &mut comm);
            helpers::node_start(&mut node_2, "sarah", &mut comm);
            helpers::node_start(&mut node_3, "james", &mut comm);

            let sig = helpers::start_comm(comm);

            node_1.forced_state(Leader);

            node_2.introduce("john");
            node_3.introduce("john");

            helpers::sleep_ms(350);

            let state = node_1.state();
            assert_eq!(Leader, state);

            node_1.stop();

            helpers::sleep_ms(500);

            let states = vec![node_2.state(), node_3.state()];
            assert!(states.contains(&Leader));

            node_2.stop();
            node_3.stop();

            sig
        })
    }


    #[test]
    fn without_majority_no_election_could_succeed() {
        let mut node_1 = helpers::node();
        let mut node_2 = helpers::node();
        let mut node_3 = helpers::node();

        helpers::with_proper_comm(|mut comm| {
            helpers::node_start(&mut node_1, "john", &mut comm);
            helpers::node_start(&mut node_2, "sarah", &mut comm);
            helpers::node_start(&mut node_3, "james", &mut comm);

            let sig = helpers::start_comm(comm);

            node_1.forced_state(Leader);

            node_2.introduce("john");
            node_3.introduce("john");

            helpers::sleep_ms(350);

            let state = node_1.state();
            assert_eq!(Leader, state);

            node_1.stop();
            node_2.stop();

            helpers::sleep_ms(700);

            let state = node_3.state();
            assert!(vec![Candidate, Follower].contains(&state));

            helpers::sleep_ms(1350);

            let state = node_3.state();
            assert!(vec![Candidate, Follower].contains(&state));

            node_3.stop();

            sig
        })
    }
}

mod replication {

    use helpers;
    use raft_rs::node::{Leader, Follower};
    use raft_rs::replication::{DefaultReplicationLog, ReplicationLog, DefaultPersistence, DefaultCommandContainer, DefaultCommand, TestSet, TestAdd, DefaultReceivable, ReceivableInt, DefaultQuery};

    #[test]
    fn three_nodes_in_a_cluster_come_to_consensus_about_one_command() {
        let mut node = helpers::node();
        let mut follower_1 = helpers::node();
        let mut follower_2 = helpers::node();

        helpers::with_proper_comm(|mut comm| {
            helpers::node_start(&mut node, "leader", &mut comm);
            helpers::node_start(&mut follower_1, "sarah", &mut comm);
            helpers::node_start(&mut follower_2, "john", &mut comm);

            node.forced_state(Leader);

            follower_1.introduce("leader");
            follower_2.introduce("leader");

            let sig = helpers::start_comm(comm);

            helpers::sleep_ms(350);

            let states = vec![node.state(), follower_1.state(), follower_2.state()];
            assert_eq!(vec![Leader, Follower, Follower], states);

            node.enqueue(DefaultCommandContainer { command: TestSet(2) });
            node.enqueue(DefaultCommandContainer { command: TestAdd(3) });
            node.enqueue(DefaultCommandContainer { command: TestSet(9) });

            helpers::sleep_ms(40);

            let (tx, rx): (Sender < DefaultReceivable >, Receiver < DefaultReceivable >) = channel();

            node.query(DefaultQuery, &tx);
            helpers::sleep_ms(30);
            assert_eq!(ReceivableInt(2), rx.try_recv().unwrap());

            node.query(DefaultQuery, &tx);
            helpers::sleep_ms(30);
            assert_eq!(ReceivableInt(5), rx.try_recv().unwrap());

            node.query(DefaultQuery, &tx);
            helpers::sleep_ms(30);
            assert_eq!(ReceivableInt(9), rx.try_recv().unwrap());

            follower_1.query(DefaultQuery, &tx);
            helpers::sleep_ms(30);
            assert_eq!(ReceivableInt(2), rx.try_recv().unwrap());

            follower_1.query(DefaultQuery, &tx);
            helpers::sleep_ms(30);
            assert_eq!(ReceivableInt(5), rx.try_recv().unwrap());

            follower_1.query(DefaultQuery, &tx);
            helpers::sleep_ms(30);
            assert_eq!(ReceivableInt(9), rx.try_recv().unwrap());

            follower_2.query(DefaultQuery, &tx);
            helpers::sleep_ms(30);
            assert_eq!(ReceivableInt(2), rx.try_recv().unwrap());

            follower_2.query(DefaultQuery, &tx);
            helpers::sleep_ms(30);
            assert_eq!(ReceivableInt(5), rx.try_recv().unwrap());

            follower_2.query(DefaultQuery, &tx);
            helpers::sleep_ms(30);
            assert_eq!(ReceivableInt(9), rx.try_recv().unwrap());

            node.stop();
            follower_1.stop();
            follower_2.stop();

            sig
        })
    }

    #[test]
    fn two_of_three_nodes_in_a_cluster_come_to_consensus_about_one_command() {
        let mut node = helpers::node();
        let mut follower_1 = helpers::node();
        let mut follower_2 = helpers::node();

        helpers::with_proper_comm(|mut comm| {
            helpers::node_start(&mut node, "leader", &mut comm);
            helpers::node_start(&mut follower_1, "sarah", &mut comm);
            helpers::node_start(&mut follower_2, "john", &mut comm);

            node.forced_state(Leader);

            follower_1.introduce("leader");
            follower_2.introduce("leader");

            let sig = helpers::start_comm(comm);

            helpers::sleep_ms(350);

            let state = node.state();
            assert_eq!(Leader, state);

            follower_1.stop();

            helpers::sleep_ms(30);

            node.enqueue(DefaultCommandContainer { command: TestSet(2) });
            node.enqueue(DefaultCommandContainer { command: TestAdd(3) });
            node.enqueue(DefaultCommandContainer { command: TestSet(9) });

            helpers::sleep_ms(40);

            let (tx, rx): (Sender < DefaultReceivable >, Receiver < DefaultReceivable >) = channel();

            node.query(DefaultQuery, &tx);
            helpers::sleep_ms(30);
            assert_eq!(ReceivableInt(2), rx.try_recv().unwrap());

            node.query(DefaultQuery, &tx);
            helpers::sleep_ms(30);
            assert_eq!(ReceivableInt(5), rx.try_recv().unwrap());

            node.query(DefaultQuery, &tx);
            helpers::sleep_ms(30);
            assert_eq!(ReceivableInt(9), rx.try_recv().unwrap());

            follower_2.query(DefaultQuery, &tx);
            helpers::sleep_ms(30);
            assert_eq!(ReceivableInt(2), rx.try_recv().unwrap());

            follower_2.query(DefaultQuery, &tx);
            helpers::sleep_ms(30);
            assert_eq!(ReceivableInt(5), rx.try_recv().unwrap());

            follower_2.query(DefaultQuery, &tx);
            helpers::sleep_ms(30);
            assert_eq!(ReceivableInt(9), rx.try_recv().unwrap());

            node.stop();
            follower_2.stop();

            sig
        })
    }

    #[test]
    fn one_of_three_nodes_in_a_cluster_doesnt_come_to_consensus_about_one_command() {
        let mut node = helpers::node();
        let mut follower_1 = helpers::node();
        let mut follower_2 = helpers::node();

        helpers::with_proper_comm(|mut comm| {
            helpers::node_start(&mut node, "leader", &mut comm);
            helpers::node_start(&mut follower_1, "sarah", &mut comm);
            helpers::node_start(&mut follower_2, "john", &mut comm);

            node.forced_state(Leader);

            follower_1.introduce("leader");
            follower_2.introduce("leader");

            let sig = helpers::start_comm(comm);

            helpers::sleep_ms(350);

            let state = node.state();
            assert_eq!(Leader, state);

            follower_1.stop();
            follower_2.stop();

            helpers::sleep_ms(30);

            node.enqueue(DefaultCommandContainer { command: TestSet(2) });
            node.enqueue(DefaultCommandContainer { command: TestAdd(3) });
            node.enqueue(DefaultCommandContainer { command: TestSet(9) });

            helpers::sleep_ms(40);

            let (tx, rx): (Sender < DefaultReceivable >, Receiver < DefaultReceivable >) = channel();

            node.query(DefaultQuery, &tx);
            helpers::sleep_ms(30);
            match rx.try_recv() {
                Ok(_) => panic!("Should have not been committed"),
                _ => (),
            }

            node.stop();

            sig
        })
    }
}
