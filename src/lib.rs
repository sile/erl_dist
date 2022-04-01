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

/// Distribution protocol version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DistributionProtocolVersion {
    /// Version 5.
    V5 = 5,

    /// Version 6 (introduced in OTP 23).
    V6 = 6,
}

impl std::fmt::Display for DistributionProtocolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", *self as u16)
    }
}

impl TryFrom<u16> for DistributionProtocolVersion {
    type Error = crate::epmd::EpmdError; // TODO

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            5 => Ok(Self::V5),
            6 => Ok(Self::V6),
            _ => Err(crate::epmd::EpmdError::UnknownVersion { value }),
        }
    }
}

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
            _ => Err(crate::epmd::EpmdError::UnknownProtocol { value }),
        }
    }
}
