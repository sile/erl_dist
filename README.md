erl_dist
========

[![erl_dist](https://img.shields.io/crates/v/erl_dist.svg)](https://crates.io/crates/erl_dist)
[![Documentation](https://docs.rs/erl_dist/badge.svg)](https://docs.rs/erl_dist)
[![Actions Status](https://github.com/sile/erl_dist/workflows/CI/badge.svg)](https://github.com/sile/erl_dist/actions)
[![Coverage Status](https://coveralls.io/repos/github/sile/erl_dist/badge.svg?branch=master)](https://coveralls.io/github/sile/erl_dist?branch=master)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Rust Implementation of [Erlang Distribution Protocol](http://erlang.org/doc/apps/erts/erl_dist_protocol.html).

The distribution protocol is used to communicate with distributed erlang nodes.

Examples
---------

Gets a node entry from EPMD:
```rust
use erl_dist::epmd::{DEFAULT_EPMD_PORT, EpmdClient};

// Connect to the local EPMD.
let connection = TcpStream::connect(("localhost", DEFAULT_EPMD_PORT)).await?;
let client = EpmdClient::new(connection);

// Get the information of a node.
let node_name = "foo";
if let Some(node) = client.get_node(node_name).await? {
    println!("Found: {:?}", node);
} else {
    println!("Not found");
}
```

Sends a message to an Erlang node:
```rust
use erl_dist::LOWEST_DISTRIBUTION_PROTOCOL_VERSION;
use erl_dist::node::{Creation, LocalNode};
use erl_dist::handshake::ClientSideHandshake;
use erl_dist::term::{Atom, Pid};
use erl_dist::message::{channel, Message};

// Connect to a peer node.
let peer_host = "localhost";
let peer_port = 7483;  // NOTE: Usually, port number is retrieved from EPMD.
let connection = TcpStream::connect((peer_host, peer_port)).await?;

// Local node information.
let creation = Creation::random();
let local_node = LocalNode::new("foo@localhost".parse()?, creation);

// Do handshake.
let mut handshake = ClientSideHandshake::new(connection, local_node.clone(), "cookie");
let _status = handshake.execute_send_name(LOWEST_DISTRIBUTION_PROTOCOL_VERSION).await?;
let (connection, peer_node) = handshake.execute_rest(true).await?;

// Create a channel.
let capability_flags = local_node.flags & peer_node.flags;
let (mut tx, _) = channel(connection, capability_flags);

// Send a message.
let from_pid = Pid::new(local_node.name.to_string(), 0, 0, local_node.creation.get());
let to_name = Atom::from("bar");
let msg = Message::reg_send(from_pid, to_name, Atom::from("hello").into());
tx.send(msg).await?;
```

Example commands:
- EPMD Client Example: [epmd_cli.rs](examples/epmd_cli.rs)
- Client Node Example: [send_msg.rs](examples/send_msg.rs)
- Server Node Example: [recv_msg.rs](examples/recv_msg.rs)
