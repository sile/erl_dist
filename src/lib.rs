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

/// Protocol for communicating with a distributed node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TransportProtocol {
    /// TCP/IPv4.
    TcpIpV4 = 0,
}

impl TryFrom<u8> for TransportProtocol {
    type Error = crate::epmd::EpmdError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::TcpIpV4),
            _ => Err(crate::epmd::EpmdError::UnknownTransportProtocol { value }),
        }
    }
}
