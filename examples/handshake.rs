//! Client side handshake example.
//!
//! # Usage Examples
//!
//! ```bash
//! $ cargo run --example handshake -- --help
//! $ cargo run --example handshake -- --peer foo --self bar@localhost --cookie erlang_cookie
//! ```
use clap::Parser;

#[derive(Debug, Parser)]
#[clap(name = "handshake")]
struct Args {
    #[clap(long = "self", default_value = "bar@localhost")]
    self_node: erl_dist::node::NodeName,

    #[clap(long = "peer", default_value = "foo@localhost")]
    peer_node: erl_dist::node::NodeName,

    #[clap(long, default_value = "WPKYDIOSJIMJUURLRUHV")]
    cookie: String,
}

impl Args {
    async fn peer_epmd_client(
        &self,
    ) -> anyhow::Result<erl_dist::epmd::EpmdClient<smol::net::TcpStream>> {
        let addr = (self.peer_node.host(), erl_dist::epmd::DEFAULT_EPMD_PORT);
        let stream = smol::net::TcpStream::connect(addr).await?;
        Ok(erl_dist::epmd::EpmdClient::new(stream))
    }

    async fn local_epmd_client(
        &self,
    ) -> anyhow::Result<erl_dist::epmd::EpmdClient<smol::net::TcpStream>> {
        let addr = (self.self_node.host(), erl_dist::epmd::DEFAULT_EPMD_PORT);
        let stream = smol::net::TcpStream::connect(addr).await?;
        Ok(erl_dist::epmd::EpmdClient::new(stream))
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    smol::block_on(async {
        let peer_node = args
            .peer_epmd_client()
            .await?
            .get_node(&args.peer_node.name())
            .await?
            .ok_or_else(|| anyhow::anyhow!("no such node: {}", args.peer_node))?;
        println!("Got peer node info: {:?}", peer_node);

        let dummy_listening_port = 3333;
        let self_node = erl_dist::epmd::NodeEntry::new_hidden(
            &args.self_node.to_string(),
            dummy_listening_port,
        );

        let (keepalive_socket, creation) = args
            .local_epmd_client()
            .await?
            .register(self_node.clone())
            .await?;
        println!("Registered self node: creation={:?}", creation);

        let stream = smol::net::TcpStream::connect((args.peer_node.host(), peer_node.port)).await?;
        let handshake = erl_dist::handshake::Handshake::new(
            self_node,
            creation,
            erl_dist::handshake::DistributionFlags::default(),
            &args.cookie,
        );
        let (_, peer_info) = handshake.connect(peer_node, stream).await?;
        println!("Handshake finished: peer={:?}", peer_info);

        std::mem::drop(keepalive_socket);
        Ok(())
    })
}
