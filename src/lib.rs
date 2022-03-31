//! Rust Implementation of Erlang Distribution Protocol.
//!
//! Distribution protocol is used to communicate with distributed erlang nodes.
//!
//! Reference: [Distribution Protocol](http://erlang.org/doc/apps/erts/erl_dist_protocol.html)
//!
//! # Examples
//!
//! - Client Node Example: [send_msg.rs](https://github.com/sile/erl_dist/blob/master/examples/send_msg.rs)
//! - Server Node Example: [recv_msg.rs](https://github.com/sile/erl_dist/blob/master/examples/recv_msg.rs)
pub mod epmd;
pub mod handshake;
pub mod message;
pub mod node;

mod channel;
mod socket;

/// Incarnation identifier of a node.
///
/// [`Creation`] is used by the node to create its pids, ports and references.
/// If the node restarts, the value of [`Creation`] will be changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Creation(u32);

impl Creation {
    /// Makes a new [`Creation`] instance.
    pub const fn new(n: u32) -> Self {
        Self(n)
    }

    /// Gets the value.
    pub const fn get(self) -> u32 {
        self.0
    }
}
