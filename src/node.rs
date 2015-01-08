extern crate time;
extern crate core;

use std::io::timer::sleep;
use std::time::duration::Duration;
use std::sync::mpsc::*;

use std::thread;

use std::{rand, num};

use std::fmt::Show;

use super::intercommunication::{Intercommunication, PackageDetails, Package, Endpoint, AppendLog, AppendLogEntry};
use super::replication::{ReplicationLog, Committable, Receivable, Queriable};

#[derive(Clone,Show,PartialEq)]
pub enum State {
    Follower,
    Candidate,
    Leader,
}

pub struct Node < T: Committable + Send, Q: Queriable + Send, R: Receivable + Send > {
    contact: Option < NodeContact < T, Q, R > >,
}

#[derive(Clone,Show,PartialEq)]
pub struct NodeHost {
    pub host: String,
}

struct NodeService < T: Committable + Send, R: ReplicationLog < T, Q, Rcv > + Send, Q: Queriable + Send, Rcv: Receivable + Send > {
    state: State,
    my_host: NodeHost,
    leader_host: Option < NodeHost >,

    contact: NodeServiceContact < T, Q, Rcv >,
    nodes: Vec < NodeHost >,

    comm: Endpoint < T >,

    last_append_log_seen_at: time::Timespec,
    last_sent_heartbeat: time::Timespec,
    term: uint,
    votes: uint,
    already_requested: bool,

    log: R,

    election_timeout: Duration,
}

struct NodeContact < T: Committable + Send, Q: Queriable + Send, R: Receivable + Send > {
    tx: Sender < Command < T, Q, R > >,
    rx: Receiver < CommandResponse >,
}

struct NodeServiceContact < T: Committable + Send, Q: Queriable + Send, R: Receivable + Send >  {
    tx: Sender < CommandResponse >,
    rx: Receiver < Command < T, Q, R > >,
}


enum Command < T: Committable + Send, Q: Queriable + Send, R: Receivable + Send > {
    Introduce(String),

    FetchNodes,

    AssignLeader(Option < NodeHost >),
    FetchLeader,

    AssignState(State),
    FetchState,

    Enqueue(T),
    Query(Q, Sender < R >),

    ExitCommand,
}

enum CommandResponse {
    FetchedLeader(Option < NodeHost >),

    FetchedState(State),

    FetchedNodes(Vec < NodeHost >),
}

impl < T: Committable + Send + Clone + Show, Q: Queriable + Send, R: Receivable + Send > Node < T, Q, R > {
    pub fn new() -> Node < T, Q, R > {
        Node { contact: None }
    }

    pub fn state(&self) -> State {
        self.contact().tx.send(Command::FetchState);
        match self.contact().rx.recv() {
            Ok(CommandResponse::FetchedState(state)) => state,
            _ => unreachable!(),
        }
    }

    pub fn forced_state(&self, state: State) -> State {
        self.contact().tx.send(Command::AssignState(state));
        match self.contact().rx.recv() {
            Ok(CommandResponse::FetchedState(state)) => state,
            _ => unreachable!(),
        }
    }

    pub fn fetch_leader(&self) -> Option < NodeHost > {
        self.contact().tx.send(Command::FetchLeader);
        match self.contact().rx.recv() {
            Ok(CommandResponse::FetchedLeader(leader)) => leader,
            _ => unreachable!(),
        }
    }

    pub fn force_follow(&self, host: &str) -> Option < NodeHost > {
        self.forced_state(State::Follower);

        self.contact().tx.send(Command::AssignLeader(Some(NodeHost { host: host.to_string() })));
        match self.contact().rx.recv() {
            Ok(CommandResponse::FetchedLeader(leader)) => leader,
            _ => unreachable!(),
        }
    }

    pub fn introduce(&self, host: &str) {
        self.forced_state(State::Follower);
        self.contact().tx.send(Command::Introduce(host.to_string()));
    }

    pub fn fetch_nodes(&self) -> Vec < NodeHost > {
        self.contact().tx.send(Command::FetchNodes);
        match self.contact().rx.recv() {
            Ok(CommandResponse::FetchedNodes(nodes)) => nodes,
            _ => unreachable!(),
        }
    }

    pub fn enqueue(&self, command: T) {
        self.contact().tx.send(Command::Enqueue(command));
    }

    pub fn query(&self, query: Q, respond_to: &Sender < R >) {
        self.contact().tx.send(Command::Query(query, respond_to.clone()));
    }

    pub fn stop(&self) {
        self.contact().tx.send(Command::ExitCommand);
    }

    pub fn start < I: Intercommunication < T >, Y: ReplicationLog < T, Q, R > + 'static + Send >(&mut self, host: &str, intercommunication: &mut I, log: Y, election_timeout: Duration) {
        match self.contact {
            Some(_) => {},
            None => self.contact = Some(NodeService::start_service(
                    host.to_string(),
                    intercommunication,
                    log,
                    election_timeout,
                    )),
        }
    }

    // private

    fn contact(&self) -> &NodeContact < T, Q, R > {
        match self.contact {
            Some(ref x) => x,
            None => panic!("You forgot to start the node")
        }
    }

}

impl < T: Committable + Clone + Show, R: ReplicationLog < T, Q, Rcv >, Q: Queriable, Rcv: Receivable > NodeService < T, R, Q, Rcv > {
    fn new (host: String, service_contact: NodeServiceContact < T, Q, Rcv >, comm: Endpoint < T >, log: R, election_timeout: Duration) -> NodeService < T, R, Q, Rcv > {
        NodeService {
            state: State::Follower,
            my_host: NodeHost { host: host.clone() },
            leader_host: None,

            contact: service_contact,
            nodes: vec![NodeHost { host: host.clone() }],

            comm: comm,

            last_append_log_seen_at: time::now().to_timespec(),
            last_sent_heartbeat: time::now().to_timespec(),
            term: 0,
            votes: 0,
            already_requested: false,

            log: log,

            election_timeout: election_timeout,
        }
    }

    fn start_service < I: Intercommunication < T > >(host: String, intercommunication: &mut I, log: R, election_timeout: Duration) -> NodeContact < T, Q, Rcv > {
        let (contact, service_contact) = NodeService::channels();

        let comm = intercommunication.register(host.clone());

        thread::Builder::new().name(format!("{}-service", host)).spawn(move || {
            let mut me = NodeService::new(host, service_contact, comm, log, election_timeout);

            let mut dead = false;

            while !dead {
                dead = dead || me.react_to_commands();

                me.react_to_intercommunication();

                me.election_handler();

                me.autocommit();

                sleep(Duration::milliseconds(2));
            }
        });

        contact
    }

    fn channels() -> (NodeContact < T, Q, Rcv >, NodeServiceContact < T, Q, Rcv >) {
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
            Ok(Command::FetchState) => { self.contact.tx.send(CommandResponse::FetchedState(self.state)); },
            Ok(Command::AssignState(state)) => {
                self.state = state;
                self.contact.tx.send(CommandResponse::FetchedState(self.state));
            },

            Ok(Command::FetchLeader) => { self.contact.tx.send(CommandResponse::FetchedLeader(self.fetch_leader_host().clone())); },
            Ok(Command::AssignLeader(leader)) => {
                self.leader_host = leader.clone();
                match leader {
                    Some(leader) => self.comm.send(leader.host, PackageDetails::Ack),
                    None => (),
                }
                self.contact.tx.send(CommandResponse::FetchedLeader(self.fetch_leader_host().clone()));
            },

            Ok(Command::FetchNodes) => { self.contact.tx.send(CommandResponse::FetchedNodes(self.nodes.clone())); },

            Ok(Command::ExitCommand) => { dead = true; },

            Ok(Command::Introduce(host)) => {
                self.comm.send(host.clone(), PackageDetails::Ack);
                self.comm.send(host, PackageDetails::LeaderQuery);
            },

            Ok(Command::Enqueue(command)) => {
                if self.state == State::Leader {
                    match self.log.enqueue(command.clone()) {
                        Ok(entry_offset) => {
                            self.send_append_log(Some(AppendLogEntry {
                                offset: entry_offset,
                                entry: command.clone(),
                            }));
                            //self.log.commit_upto(entry_offset);

                            let me = self.my_host.host.clone();
                            self.log.persisted(entry_offset, me);
                        },
                        _ => ()
                    }
                }
            },

            Ok(Command::Query(query, respond_to)) => {
                self.log.query_persistance(query, respond_to);
            }

            Err(Disconnected) => { dead = true; },

            Err(_) => (),
        }

        dead
    }

    fn react_to_intercommunication(&mut self) {
        match self.comm.listen() {
            Some(Package::Pack(from, _, PackageDetails::Ack)) => {
                self.nodes.push(NodeHost { host: from });
            },

            Some(Package::Pack(from, _, PackageDetails::LeaderQuery)) => {
                let leader_host = match self.fetch_leader_host() {
                    Some(NodeHost { ref host }) => Some(host.clone()),
                    None => None,
                };

                self.comm.send(from, PackageDetails::LeaderQueryResponse(leader_host));
            },

            Some(Package::Pack(_, _, PackageDetails::LeaderQueryResponse(leader_host))) => {
                match leader_host {
                    Some(host) => self.comm.send(host, PackageDetails::Ack),
                    None => (),
                }
            },

            Some(Package::Pack(leader, _, PackageDetails::AppendQuery(log))) => {
                self.nodes = log.node_list.iter().map(|x| { NodeHost { host: x.clone() } }).collect();
                self.last_append_log_seen_at = time::now().to_timespec();
                self.leader_host = Some(NodeHost { host: leader.clone() });
                self.log.commit_upto(log.committed_offset);

                match log.enqueue {
                    Some(log_entry) => {
                        match self.log.enqueue(log_entry.entry.clone()) {
                            Ok(my_offset) => self.comm.send(leader, PackageDetails::Persisted(log_entry.offset)),
                            _ => (),
                        }
                    },
                    None => (),
                }
            },

            Some(Package::Pack(follower, _, PackageDetails::Persisted(offset))) => {
                self.log.persisted(offset, follower);
            }

            Some(Package::Pack(candidate, _, PackageDetails::RequestVote(term))) => {
                if term > self.term {
                    self.term = term;
                    self.votes = 0;
                    self.last_append_log_seen_at = time::now().to_timespec();
                    self.comm.send(candidate, PackageDetails::Vote(term));
                }
            },

            Some(Package::Pack(_, _, PackageDetails::Vote(term))) => {
                if term == self.term && self.state == State::Candidate {
                    self.votes += 1;
                    if self.votes > self.nodes.len() / 2 {
                        self.state = State::Leader;
                        self.send_append_log(None);
                    }
                }
            },

            None => (),
        }
    }

    fn send_append_log(&mut self, enqueue: Option < AppendLogEntry < T > >) {
        let node_list: Vec < String > = self.nodes.clone().iter().map(|x| { x.host.clone() }).collect();

        for node in self.nodes.iter() {
            if node.host != self.my_host.host {
                let committed_offset = self.log.committed_offset();
                self.comm.send(node.host.clone(), PackageDetails::AppendQuery(AppendLog {
                    committed_offset: committed_offset,
                    node_list: node_list.clone(),
                    enqueue: enqueue.clone(),
                }));
            }
        }
    }

    fn election_handler(&mut self) {
        let passed = time::now().to_timespec() - self.last_append_log_seen_at;
        let passed_since_heartbeat = time::now().to_timespec() - self.last_sent_heartbeat;
        let duration = self.election_timeout;
        let heartbeat_timeout = Duration::milliseconds(70);

        match self.state {
            State::Follower => {
                if passed > duration {
                    self.state = State::Candidate;
                    self.votes = 0;
                    self.already_requested = false;
                    self.last_append_log_seen_at = time::now().to_timespec();
                }
            },

            State::Candidate => {
                if passed > duration {
                    self.state = State::Follower;
                    self.votes = 0;
                    self.last_append_log_seen_at = time::now().to_timespec();
                }

                if !self.already_requested && self.state == State::Candidate {
                    self.already_requested = true;
                    self.term += 1;

                    self.comm.send(self.my_host.host.clone(), PackageDetails::Vote(self.term));

                    for node in self.nodes.iter() {
                        self.comm.send(node.host.clone(), PackageDetails::RequestVote(self.term));
                    }
                }
            },

            State::Leader => {
                self.leader_host = None;

                if passed_since_heartbeat > heartbeat_timeout {
                    self.send_append_log(None);
                    self.last_sent_heartbeat = time::now().to_timespec();
                }
            },
        }
    }

    fn fetch_leader_host(&self) -> Option < NodeHost > {
        match self.state {
            State::Leader => None,
            _ => self.leader_host.clone(),
        }
    }

    fn autocommit(&mut self) {
        let majority_size = (self.nodes.len() + 1) / 2;
        let committed_offset_was = self.log.committed_offset();

        match self.state {
            State::Leader => self.log.autocommit_if_safe(majority_size),
            _ => (),
        }

        if committed_offset_was < self.log.committed_offset() {
            self.send_append_log(None);
        }
    }
}
