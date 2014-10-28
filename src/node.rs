use std::io::timer::sleep;
use std::time::duration::Duration;
use std::comm::{Disconnected, TryRecvError};

use super::intercommunication::{Intercommunication, Ack};

#[deriving(Clone,Show,PartialEq)]
pub enum State {
    Follower,
    Candidate,
    Leader,
}

pub struct Node {
    contact: Option < NodeContact >,
}

#[deriving(Clone,Show,PartialEq)]
pub struct NodeHost {
    pub host: String,
}

struct NodeService {
    state: State,
    my_host: NodeHost,
    leader_host: Option < NodeHost >,

    contact: NodeServiceContact,
    nodes: Vec < NodeHost >,
}

struct NodeContact {
    state_tx: Sender < Option < State > >,
    state_rx: Receiver < State >,

    leader_tx: Sender < Option < String > >,
    leader_rx: Receiver < Option < NodeHost > >,

    exit_tx: Sender < int >,

    nodes_tx: Sender < int >,
    nodes_rx: Receiver < Vec < NodeHost > >,
}

struct NodeServiceContact {
    state_rx: Receiver < Option < State > >,
    state_tx: Sender < State >,

    leader_rx: Receiver < Option < String > >,
    leader_tx: Sender < Option < NodeHost > >,

    exit_rx: Receiver < int >,

    nodes_tx: Sender < Vec < NodeHost > >,
    nodes_rx: Receiver < int >,
}

impl < T: Intercommunication > Node {
    pub fn new() -> Node {
        Node { contact: None }
    }

    pub fn state(&self) -> State {
        self.contact().state_tx.send(None);
        self.contact().state_rx.recv()
    }

    pub fn forced_state(&self, state: State) -> State {
        self.contact().state_tx.send(Some(state));
        self.contact().state_rx.recv()
    }

    pub fn fetch_leader(&self) -> Option < NodeHost > {
        self.contact().leader_tx.send(None);
        self.contact().leader_rx.recv()
    }

    pub fn force_follow(&self, host: &str) -> Option < NodeHost > {
        self.forced_state(Follower);

        self.contact().leader_tx.send(Some(host.to_string()));
        self.contact().leader_rx.recv()
    }

    pub fn fetch_nodes(&self) -> Vec < NodeHost > {
        self.contact().nodes_tx.send(0);
        self.contact().nodes_rx.recv()
    }

    pub fn stop(&self) {
        self.contact().exit_tx.send(0);
    }

    pub fn start(&mut self, host: &str, intercommunication: T) {
        match self.contact {
            Some(_) => {},
            None => self.contact = Some(NodeService::start_service(host.to_string(), intercommunication)),
        }
    }

    // private

    fn contact(&self) -> &NodeContact {
        match self.contact {
            Some(ref x) => x,
            None => fail!("You forgot to start the node")
        }
    }

}

impl < T: Intercommunication > NodeService {
    fn start_service(host: String, intercommunication: T) -> NodeContact {
        let (state_tx, service_state_rx) = channel();
        let (service_state_tx, state_rx) = channel();

        let (leader_tx, service_leader_rx) = channel();
        let (service_leader_tx, leader_rx) = channel();

        let (exit_tx, service_exit_rx) = channel();

        let (nodes_tx, service_nodes_rx) = channel();
        let (service_nodes_tx, nodes_rx) = channel();

        let contact = NodeContact {
            state_tx: state_tx,
            state_rx: state_rx,

            leader_tx: leader_tx,
            leader_rx: leader_rx,

            exit_tx: exit_tx,

            nodes_tx: nodes_tx,
            nodes_rx: nodes_rx,
        };

        let service_contact = NodeServiceContact {
            state_rx: service_state_rx,
            state_tx: service_state_tx,

            leader_rx: service_leader_rx,
            leader_tx: service_leader_tx,

            exit_rx: service_exit_rx,

            nodes_tx: service_nodes_tx,
            nodes_rx: service_nodes_rx,
        };

        spawn(proc() {
            let mut me = NodeService {
                state: Follower,
                my_host: NodeHost { host: host.clone() },
                leader_host: None,

                contact: service_contact,
                nodes: vec![NodeHost { host: host.clone() }],
            };

            let mut dead = false;

            while !dead {
                dead = dead || me.try_serve_state();
                dead = dead || me.try_serve_leader();
                dead = dead || me.try_serve_nodes();
                dead = dead || me.exit_if_asked();

                match intercommunication.try_listen() {
                    Some(Ack(from_host, _)) => {
                        let node_hosts: Vec < String > = me.nodes.iter().map(|x| { x.host }).collect();

                        match node_hosts.contains(&from_host) {
                            true => (),
                            false => {
                                me.nodes.push(NodeHost { host: from_host.clone() });
                            }
                        }
                    },
                    _ => ()
                }

                sleep(Duration::milliseconds(10));
            }
        });

        contact
    }

    fn try_serve_nodes(&mut self) -> bool {
        match self.contact.nodes_rx.try_recv() {
            Ok(_) => {
                self.contact.nodes_tx.send(self.nodes.clone());
                false
            },
            Err(err) if err == Disconnected => true,
            _ => false
        }
    }

    fn try_serve_state(&mut self) -> bool {
        let received = self.contact.state_rx.try_recv();

        self.try_serve(
            received,
            |ref mut me, value| { me.state = value; },
            |ref me| { me.contact.state_tx.send(me.state.clone()); }
            )
    }

    fn try_serve_leader(&mut self) -> bool {
        let received = self.contact.leader_rx.try_recv();

        self.try_serve(
            received,
            |ref mut me, value| { me.leader_host = Some(NodeHost { host: value }); },
            |ref me| { me.contact.leader_tx.send(me.leader_host.clone()); }
            )
    }

    fn try_serve < T > (&mut self, received: Result < Option < T >, TryRecvError >, change: |&mut NodeService, T| -> (), respond: |&NodeService| -> ()) -> bool {
        match received {
            Ok(value) => {
                match value {
                    Some(value) => change(self, value),
                    _ => {},
                }

                respond(self);
                false
            },
            Err(err) if err == Disconnected => true,
            _ => false
        }
    }

    fn exit_if_asked(&self) -> bool {
        match self.contact.exit_rx.try_recv() {
            Ok(_) => true,
            _ => false
        }
    }
}
