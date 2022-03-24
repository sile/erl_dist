//! Rust Implementation of Erlang Distribution Protocol.
//!
//! Distribution protocol is used to communicate with distributed erlang nodes.
//!
//! Reference: [12 Distribution Protocol](http://erlang.org/doc/apps/erts/erl_dist_protocol.html)
//!
//! # Examples
//!
//! - Client Node Example: [send_msg.rs]
//!                        (https://github.com/sile/erl_dist/blob/master/examples/send_msg.rs)
//! - Server Node Example: [recv_msg.rs]
//!                        (https://github.com/sile/erl_dist/blob/master/examples/recv_msg.rs)
// #![warn(missing_docs)]

// #[macro_use]
// extern crate bitflags;

// macro_rules! invalid_data {
//     ($fmt:expr) => { invalid_data!($fmt,); };
//     ($fmt:expr, $($arg:tt)*) => {
//         ::std::io::Error::new(::std::io::ErrorKind::InvalidData, format!($fmt, $($arg)*))
//     };
// }

// pub use epmd::EpmdClient;
// pub use handshake::Handshake;
// pub use message::Message;

// pub mod channel;
pub mod epmd;
// pub mod handshake;
// pub mod message;

// /// The generation number of a distributed node.
// ///
// /// If a node restarts, the count will be incremented.
// ///
// /// Note the counter will wrap around, if it exceeds four.
// #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
// pub struct Creation(u8);
// impl Creation {
//     fn from_u16(c: u16) -> Option<Self> {
//         if c < 4 {
//             Some(Creation(c as u8))
//         } else {
//             None
//         }
//     }

//     /// Returns the inner counter value of this `Creation`.
//     ///
//     /// The range of this value is limited to `0..4`.
//     pub fn as_u8(&self) -> u8 {
//         self.0
//     }
// }

// #[cfg(test)]
// mod tests {
//     #[test]
//     fn it_works() {}
// }
