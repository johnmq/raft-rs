# Raft-rs [![Build Status](https://travis-ci.org/johnmq/raft-rs.svg?branch=master)](https://travis-ci.org/johnmq/raft-rs)

> Raft consensus algorithm implementation in Rust language as a reusable library.

You can read about Raft consensus algorithm here:
[raftconsensus.github.io](http://raftconsensus.github.io/). It is very simple
to understand and is a very powerful tool to use.

Part of the [JohnMQ](https://github.com/johnmq) Project - High-performance,
persistent, reliable and dumb-simple messaging queue in Rust.

## Disclaimer

This project is still under heavy development and lack key features and
therefore is not recommended for production usage. Missing features:

- It is not working, yet. But first iteration is almost over, and after that it
  may be used with caution.

Contributions are highly welcome. And by the way, I'm looking for collaborators
to make this project production ready faster. Email me if you feel like:
[waterlink000@gmail.com](mailto:waterlink000+johnmq@gmail.com)

## Usage

*NOTE: Work in progress.*

This project is powered by [cargo](http://doc.crates.io).

Clone & build the project:

```
git clone https://github.com/johnmq/raft-rs.git
cd raft-rs
cargo build
```

Optionally check that tests are passing (since your version of `rustc` could be
incompatible with current version of `raft-rs`):

```
cargo test               # to run the same suite that is run on travis
cargo test -- --bench    # if you want to run benchmarks
```

## Usage as library

*NOTE: Work in progress.*

`raft-rs` is build as library. So you can install raft-rs in your regular Cargo
project by adding this to your `Cargo.toml`:

```toml
[dependencies.raft_rs]

git = "https://github.com/johnmq/raft-rs.git"
```

### Exposed abstractions (traits)

It exposes a bunch of traits that can be implemented and injected back into
`raft_rs` to make it work in your domain:

```rust
// use raft_rs::intercommunication::Intercommunication;
pub trait Intercommunication < T: Committable + Send + Show > {
    fn new() -> Self;
    fn register(&mut self, host: String) -> Endpoint < T >;
    fn receive(&mut self) -> Option < Package < T > >;
    fn send(&mut self, recipient: String, package: Package < T >);
    fn is_debug(&self) -> bool;
}

// use raft_rs::replication::{Committable, Receivable, Queriable, LogPersistence, ReplicationLog};
pub trait Committable { }
pub trait Receivable { }
pub trait Queriable { }
pub trait LogPersistence < T: Committable > {
    fn commit(&self, entry: T) -> io::IoResult < () >;
}
pub trait ReplicationLog < T: Committable, Q: Queriable, R: Receivable > {
    fn new() -> Self;

    fn len(&self) -> uint;
    fn committed_offset(&self) -> uint;
    fn commit_upto(&mut self, new_committed_offset: uint) -> io::IoResult < () >;
    fn discard_downto(&mut self, new_len: uint) -> io::IoResult < () >;

    fn autocommit_if_safe(&mut self, majority_size: uint);

    fn enqueue(&mut self, entry: T) -> io::IoResult < uint >;
    fn persisted(&mut self, offset: uint, node: String) -> io::IoResult < uint >;
    fn query_persistance(&mut self, query: Q, respond_to: Sender < R >);
}
```

*TODO: example how to implement these traits and how to inject them back into raft_rs.*

### Further examples

*TODO: put simple implementation examples in `examples/` folder and link to it from here.*

## Contributing

1. Fork it https://github.com/johnmq/raft-rs/fork
2. Create your feature branch (git checkout -b my-new-feature)
3. Commit your changes (git commit -am "Add some feature")
4. Push to the branch (git push origin my-new-feature)
5. Create a new Pull Request
