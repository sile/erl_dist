//! Rust Implementation of Erlang Distribution Protocol.
//!
//! Distribution protocol is used to communicate with distributed erlang nodes.
//!
//! Reference: [Distribution Protocol](http://erlang.org/doc/apps/erts/erl_dist_protocol.html)
//!
//! # Examples
//!
//! Gets a node entry from EPMD:
//! ```no_run
//! # use smol::net::TcpStream;
//! use erl_dist::epmd::{DEFAULT_EPMD_PORT, EpmdClient};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # smol::block_on(async {
//! // Connect to the local EPMD.
//! let connection = TcpStream::connect(("localhost", DEFAULT_EPMD_PORT)).await?;
//! let client = EpmdClient::new(connection);
//!
//! // Get the information of a node.
//! let node_name = "foo";
//! if let Some(node) = client.get_node(node_name).await? {
//!     println!("Found: {:?}", node);
//! } else {
//!     println!("Not found");
//! }
//! # Ok(())
//! # })
//! # }
//! ```
//!
//! Sends a message to an Erlang node:
//! ```no_run
//! # use smol::net::TcpStream;
//! use erl_dist::LOWEST_DISTRIBUTION_PROTOCOL_VERSION;
//! use erl_dist::node::{Creation, LocalNode};
//! use erl_dist::handshake::ClientSideHandshake;
//! use erl_dist::term::{Atom, Pid};
//! use erl_dist::message::{channel, Message};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # smol::block_on(async {
//! // Connect to a peer node.
//! let peer_host = "localhost";
//! let peer_port = 7483;  // NOTE: Usually, port number is retrieved from EPMD.
//! let connection = TcpStream::connect((peer_host, peer_port)).await?;
//!
//! // Local node information.
//! let creation = Creation::random();
//! let local_node = LocalNode::new("foo@localhost".parse()?, creation);
//!
//! // Do handshake.
//! let mut handshake = ClientSideHandshake::new(connection, local_node.clone(), "cookie");
//! let _status = handshake.execute_send_name(LOWEST_DISTRIBUTION_PROTOCOL_VERSION).await?;
//! let (connection, peer_node) = handshake.execute_rest(true).await?;
//!
//! // Create a channel.
//! let capability_flags = local_node.flags & peer_node.flags;
//! let (mut tx, _) = channel(connection, capability_flags);
//!
//! // Send a message.
//! let from_pid = Pid::new(local_node.name.to_string(), 0, 0, local_node.creation.get());
//! let to_name = Atom::from("bar");
//! let msg = Message::reg_send(from_pid, to_name, Atom::from("hello").into());
//! tx.send(msg).await?;
//! # Ok(())
//! # })
//! # }
//! ```
//!
//! Example commands:
//! - EPMD Client Example: [send_msg.rs](https://github.com/sile/erl_dist/blob/master/examples/epmd_cli.rs)
//! - Client Node Example: [send_msg.rs](https://github.com/sile/erl_dist/blob/master/examples/send_msg.rs)
//! - Server Node Example: [recv_msg.rs](https://github.com/sile/erl_dist/blob/master/examples/recv_msg.rs)
#![warn(missing_docs)]
pub mod epmd;
pub mod handshake;
pub mod message;
pub mod node;
pub mod term;

mod channel;
mod eetf_ext;
mod flags;
mod io;

pub use self::flags::DistributionFlags;

/// The lowest distribution protocol version this crate can handle.
pub const LOWEST_DISTRIBUTION_PROTOCOL_VERSION: u16 = 6;

/// The highest distribution protocol version this crate can handle.
pub const HIGHEST_DISTRIBUTION_PROTOCOL_VERSION: u16 = 6;

#[cfg(test)]
mod tests {
    use std::process::{Child, Command};

    use orfail::OrFail;

    pub const COOKIE: &str = "test-cookie";

    #[derive(Debug)]
    pub struct TestErlangNode {
        child: Child,
    }

    impl TestErlangNode {
        pub async fn new(name: &str) -> orfail::Result<Self> {
            let child = Command::new("erl")
                .args(&["-sname", name, "-noshell", "-setcookie", COOKIE])
                .spawn()
                .or_fail()?;
            let start = std::time::Instant::now();
            loop {
                if let Ok(client) = try_epmd_client().await {
                    if client.get_node(name).await.or_fail()?.is_some() {
                        break;
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
                if start.elapsed() > std::time::Duration::from_secs(10) {
                    break;
                }
            }
            Ok(Self { child })
        }
    }

    impl Drop for TestErlangNode {
        fn drop(&mut self) {
            let _ = self.child.kill();
        }
    }

    pub async fn try_epmd_client() -> orfail::Result<crate::epmd::EpmdClient<smol::net::TcpStream>>
    {
        let client = smol::net::TcpStream::connect(("127.0.0.1", crate::epmd::DEFAULT_EPMD_PORT))
            .await
            .map(crate::epmd::EpmdClient::new)
            .or_fail()?;
        Ok(client)
    }

    pub async fn epmd_client() -> crate::epmd::EpmdClient<smol::net::TcpStream> {
        try_epmd_client().await.unwrap()
    }
}
