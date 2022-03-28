//! Rust Implementation of Erlang Distribution Protocol.
//!
//! Distribution protocol is used to communicate with distributed erlang nodes.
//!
//! Reference: [12 Distribution Protocol](http://erlang.org/doc/apps/erts/erl_dist_protocol.html)
//!
//! # Examples
//!
//! - Client Node Example: [send_msg.rs](https://github.com/sile/erl_dist/blob/master/examples/send_msg.rs)
//! - Server Node Example: [recv_msg.rs](https://github.com/sile/erl_dist/blob/master/examples/recv_msg.rs)
// #![warn(missing_docs)]

// #[macro_use]
// extern crate bitflags;

// macro_rules! invalid_data {
//     ($fmt:expr) => { invalid_data!($fmt,); };
//     ($fmt:expr, $($arg:tt)*) => {
//         ::std::io::Error::new(::std::io::ErrorKind::InvalidData, format!($fmt, $($arg)*))
//     };
// }

// pub use message::Message;

// TODO: pub mod remote_node;

pub mod channel;
pub mod epmd;
pub mod handshake;
pub mod message;
pub mod node;

mod socket;

/// The generation number of a distributed node.
///
/// If a node restarts, the count will be incremented.
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
