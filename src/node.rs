enum State {
    Follower(Option < Box < Node > >),
    Candidate,
    Leader
}

struct Node {
    state: State
}

impl Node {
    fn new() -> Node {
        Node {
            state: Follower(None)
        }
    }

    fn lead(&mut self) {
        self.state = Leader;
    }

    fn follow(&mut self, leader: Box < Node >) {
        match leader.state {
            Leader => self.state = Follower(Some(leader)),
            _ => self.unfollow()
        }
    }

    fn become_a_candidate(&mut self) {
        self.state = Candidate;
    }

    fn unfollow(&mut self) {
        self.state = Follower(None);
    }
}

#[cfg(test)]
mod test {

    use node::Node;

    #[test]
    fn creating_node() {
        Node::new();
    }

    mod a_node_can_be_in_1_of_3_states {

        use node::{Node, Follower, Leader, Candidate};

        #[test]
        fn follower_state_by_default() {
            let node = Node::new();

            match node.state {
                Follower(None) => {},
                _ => fail!("Default state should be follower with absent leader")
            }
        }

        #[test]
        fn follower_state_with_leader() {
            let mut leader = box Node::new();
            let mut follower = Node::new();

            leader.lead();
            follower.follow(leader);

            match follower.state {
                Follower(Some(leader)) => {},
                _ => fail!("Should have been following after the leader")
            }
        }

        #[test]
        fn unable_to_follow_non_leader_node() {
            let mut somenode = box Node::new();
            let mut follower = Node::new();

            follower.follow(somenode);

            match follower.state {
                Follower(None) => {},
                _ => fail!("Should have been following noone")
            }
        }

        #[test]
        fn candidate_state() {
            let mut node = Node::new();

            node.become_a_candidate();

            match node.state {
                Candidate => {},
                _ => fail!("Should have been a candidate")
            }
        }

        #[test]
        fn leader_state() {
            let mut node = Node::new();

            node.lead();

            match node.state {
                Leader => {},
                _ => fail!("Should have been a leader")
            }
        }

        #[test]
        fn can_unfollow() {
            let mut leader = box Node::new();
            let mut node = Node::new();

            leader.lead();
            node.follow(leader);
            node.unfollow();

            match node.state {
                Follower(None) => {},
                _ => fail!("Should have been following noone")
            }
        }

    }

}
