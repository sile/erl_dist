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
pub mod capability;
pub mod epmd;
pub mod handshake;
pub mod message;
pub mod node;

mod channel;
mod socket;

// TODO
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

#[cfg(test)]
mod tests {
    use std::process::{Child, Command};

    pub const COOKIE: &str = "test-cookie";

    #[derive(Debug)]
    pub struct TestErlangNode {
        child: Child,
    }

    impl TestErlangNode {
        pub async fn new(name: &str) -> anyhow::Result<Self> {
            let child = Command::new("erl")
                .args(&["-sname", name, "-noshell", "-setcookie", COOKIE])
                .spawn()?;
            let start = std::time::Instant::now();
            while epmd_client().await.get_node(name).await?.is_none() {
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

    pub async fn epmd_client() -> crate::epmd::EpmdClient<smol::net::TcpStream> {
        let stream = smol::net::TcpStream::connect(("127.0.0.1", crate::epmd::DEFAULT_EPMD_PORT))
            .await
            .unwrap();
        crate::epmd::EpmdClient::new(stream)
    }
}
