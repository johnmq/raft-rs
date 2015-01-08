#![crate_name = "raft_rs"]
#![comment = "Implementation of Raft distributed consensus protocol in Rust"]
#![license = "MIT"]

extern crate "rustc-serialize" as rustc_serialize;

pub mod node;
pub mod intercommunication;
pub mod replication;
