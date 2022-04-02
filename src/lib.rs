//! Rust Implementation of Erlang Distribution Protocol.
//!
//! Distribution protocol is used to communicate with distributed erlang nodes.
//!
//! Reference: [Distribution Protocol](http://erlang.org/doc/apps/erts/erl_dist_protocol.html)
//!
//! # Examples
//!
//! TODO: Add a minimum example code
//!
//! - Client Node Example: [send_msg.rs](https://github.com/sile/erl_dist/blob/master/examples/send_msg.rs)
//! - Server Node Example: [recv_msg.rs](https://github.com/sile/erl_dist/blob/master/examples/recv_msg.rs)

pub mod epmd;
pub mod handshake;
pub mod message;
pub mod node;

mod channel;
mod flags;
mod io;

pub use self::flags::DistributionFlags;

/// The lowest distribution protocol version this crate can handle.
pub const LOWEST_DISTRIBUTION_PROTOCOL_VERSION: u16 = 5;

/// The highest distribution protocol version this crate can handle.
pub const HIGHEST_DISTRIBUTION_PROTOCOL_VERSION: u16 = 6;

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
            loop {
                if let Ok(client) = try_epmd_client().await {
                    if client.get_node(name).await?.is_some() {
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

    pub async fn try_epmd_client() -> anyhow::Result<crate::epmd::EpmdClient<smol::net::TcpStream>>
    {
        let client = smol::net::TcpStream::connect(("127.0.0.1", crate::epmd::DEFAULT_EPMD_PORT))
            .await
            .map(crate::epmd::EpmdClient::new)?;
        Ok(client)
    }

    pub async fn epmd_client() -> crate::epmd::EpmdClient<smol::net::TcpStream> {
        try_epmd_client().await.unwrap()
    }
}
