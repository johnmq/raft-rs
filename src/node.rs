extern crate time;

use std::io::timer::sleep;
use std::time::duration::Duration;
use std::comm::{Disconnected, TryRecvError};

use std::sync::{Arc, Mutex};

use std::task::TaskBuilder;

use std::{rand, num};

use super::intercommunication::{Intercommunication, Ack, LeaderQuery, LeaderQueryResponse, Pack, Endpoint, AppendQuery, AppendLog, RequestVote, Vote};

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

    comm: Endpoint,

    last_append_log_seen_at: time::Timespec,
    term: uint,
    votes: uint,
    already_requested: bool,
}

struct NodeContact {
    tx: Sender < Command >,
    rx: Receiver < CommandResponse >,
}

struct NodeServiceContact {
    tx: Sender < CommandResponse >,
    rx: Receiver < Command >,
}


enum Command {
    Introduce(String),

    FetchNodes,

    AssignLeader(Option < NodeHost >),
    FetchLeader,

    AssignState(State),
    FetchState,

    ExitCommand,
}

enum CommandResponse {
    FetchedLeader(Option < NodeHost >),

    FetchedState(State),

    FetchedNodes(Vec < NodeHost >),
}

impl Node {
    pub fn new() -> Node {
        Node { contact: None }
    }

    pub fn state(&self) -> State {
        self.contact().tx.send(FetchState);
        match self.contact().rx.recv() {
            FetchedState(state) => state,
            _ => unreachable!(),
        }
    }

    pub fn forced_state(&self, state: State) -> State {
        self.contact().tx.send(AssignState(state));
        match self.contact().rx.recv() {
            FetchedState(state) => state,
            _ => unreachable!(),
        }
    }

    pub fn fetch_leader(&self) -> Option < NodeHost > {
        self.contact().tx.send(FetchLeader);
        match self.contact().rx.recv() {
            FetchedLeader(leader) => leader,
            _ => unreachable!(),
        }
    }

    pub fn force_follow(&self, host: &str) -> Option < NodeHost > {
        self.forced_state(Follower);

        self.contact().tx.send(AssignLeader(Some(NodeHost { host: host.to_string() })));
        match self.contact().rx.recv() {
            FetchedLeader(leader) => leader,
            _ => unreachable!(),
        }
    }

    pub fn introduce(&self, host: &str) {
        self.contact().tx.send(Introduce(host.to_string()));
    }

    pub fn fetch_nodes(&self) -> Vec < NodeHost > {
        self.contact().tx.send(FetchNodes);
        match self.contact().rx.recv() {
            FetchedNodes(nodes) => nodes,
            _ => unreachable!(),
        }
    }

    pub fn stop(&self) {
        self.contact().tx.send(ExitCommand);
    }

    pub fn start(&mut self, host: &str, intercommunication: &mut Intercommunication) {
        match self.contact {
            Some(_) => {},
            None => self.contact = Some(NodeService::start_service(
                    host.to_string(),
                    intercommunication,
                    )),
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

impl NodeService {
    fn new(host: String, service_contact: NodeServiceContact, comm: Endpoint) -> NodeService {
        NodeService {
            state: Follower,
            my_host: NodeHost { host: host.clone() },
            leader_host: None,

            contact: service_contact,
            nodes: vec![NodeHost { host: host.clone() }],

            comm: comm,

            last_append_log_seen_at: time::now().to_timespec(),
            term: 0,
            votes: 0,
            already_requested: false,
        }
    }

    fn start_service(host: String, intercommunication: &mut Intercommunication) -> NodeContact {
        let (contact, service_contact) = NodeService::channels();

        let mut comm = intercommunication.register(host.clone());

        TaskBuilder::new().named(format!("{}-service", host)).spawn(proc() {
            let mut me = NodeService::new(host, service_contact, comm);

            let mut dead = false;

            while !dead {
                dead = dead || me.react_to_commands();

                me.react_to_intercommunication();

                me.election_handler();

                sleep(Duration::milliseconds(10));
            }
        });

        contact
    }

    fn channels() -> (NodeContact, NodeServiceContact) {
        let (tx, service_rx) = channel();
        let (service_tx, rx) = channel();

        let contact = NodeContact {
            tx: tx,
            rx: rx,
        };

        let service_contact = NodeServiceContact {
            tx: service_tx,
            rx: service_rx,
        };

        (contact, service_contact)
    }

    fn react_to_commands(&mut self) -> bool {
        let mut dead = false;

        match self.contact.rx.try_recv() {
            Ok(FetchState) => self.contact.tx.send(FetchedState(self.state)),
            Ok(AssignState(state)) => {
                self.state = state;
                self.contact.tx.send(FetchedState(self.state));
            },

            Ok(FetchLeader) => self.contact.tx.send(FetchedLeader(self.leader_host.clone())),
            Ok(AssignLeader(leader)) => {
                self.leader_host = leader.clone();
                match leader {
                    Some(leader) => self.comm.send(leader.host, Ack),
                    None => (),
                }
                self.contact.tx.send(FetchedLeader(self.leader_host.clone()));
            },

            Ok(FetchNodes) => self.contact.tx.send(FetchedNodes(self.nodes.clone())),

            Ok(ExitCommand) => dead = true,

            Ok(Introduce(host)) => {
                self.comm.send(host.clone(), Ack);
                self.comm.send(host, LeaderQuery);
            },

            Err(Disconnected) => dead = true,

            Err(_) => (),
        }

        dead
    }

    fn react_to_intercommunication(&mut self) {
        match self.comm.listen() {
            Some(Pack(from, _, Ack)) => {
                self.nodes.push(NodeHost { host: from });
            },
            Some(Pack(from, _, LeaderQuery)) => {
                let leader_host = match self.leader_host {
                    Some(NodeHost { ref host }) => Some(host.clone()),
                    None => None,
                };

                self.comm.send(from, LeaderQueryResponse(leader_host));
            },
            Some(Pack(_, _, LeaderQueryResponse(leader_host))) => {
                match leader_host {
                    Some(host) => self.comm.send(host, Ack),
                    None => (),
                }
            },
            Some(Pack(leader, _, AppendQuery(log))) => {
                self.nodes = log.node_list.iter().map(|x| { NodeHost { host: x.clone() } }).collect();
                self.last_append_log_seen_at = time::now().to_timespec();
                self.leader_host = Some(NodeHost { host: leader });
            },
            Some(Pack(candidate, _, RequestVote(term))) => {
                if term > self.term {
                    self.term = term;
                    self.votes = 0;
                    self.last_append_log_seen_at = time::now().to_timespec();
                    self.comm.send(candidate, Vote(term));
                }
            },
            Some(Pack(_, _, Vote(term))) => {
                if term == self.term && self.state == Candidate {
                    self.votes += 1;
                    if self.votes > self.nodes.len() / 2 {
                        self.state = Leader;
                    }
                }
            },
            None => (),
        }
    }

    fn send_append_log(&mut self) {
        let node_list = self.nodes.clone();
        let log = AppendLog {
            node_list: node_list.iter().map(|x| { x.host.clone() }).collect(),
        };

        for node in self.nodes.iter() {
            if node.host != self.my_host.host {
                self.comm.send(node.host.clone(), AppendQuery(log.clone()));
            }
        }
    }

    fn election_handler(&mut self) {
        let passed = time::now().to_timespec() - self.last_append_log_seen_at;
        let ms = 150 + num::abs(rand::random::< i64 >() % 150);
        let duration = Duration::milliseconds(ms);

        match self.state {
            Follower => {
                if passed > duration {
                    println!("Passed duration: {}", ms);
                    self.state = Candidate;
                    self.votes = 0;
                    self.already_requested = false;
                    self.last_append_log_seen_at = time::now().to_timespec();
                }
            },
            Candidate => {
                if passed > duration {
                    self.state = Follower;
                    self.votes = 0;
                    self.last_append_log_seen_at = time::now().to_timespec();
                }

                if !self.already_requested && self.state == Candidate {
                    self.already_requested = true;
                    self.term += 1;

                    for node in self.nodes.iter() {
                        if node.host != self.my_host.host {
                            self.comm.send(node.host.clone(), RequestVote(self.term));
                        }
                    }
                }
            },
            Leader => {
                self.send_append_log();
            },
        }
    }
}
