//! Client side handshake example.
//!
//! # Usage Examples
//!
//! ```bash
//! $ cargo run --example handshake -- --help
//! $ cargo run --example handshake -- --peer foo --self bar@localhost --cookie erlang_cookie
//! ```
use clap::Parser;
use orfail::OrFail;

#[derive(Debug, Parser)]
#[clap(name = "handshake")]
struct Args {
    #[clap(long = "self", default_value = "bar@localhost")]
    local_node: erl_dist::node::NodeName,

    #[clap(long = "peer", default_value = "foo@localhost")]
    peer_node: erl_dist::node::NodeName,

    #[clap(long, default_value = "WPKYDIOSJIMJUURLRUHV")]
    cookie: String,
}

impl Args {
    async fn peer_epmd_client(
        &self,
    ) -> orfail::Result<erl_dist::epmd::EpmdClient<smol::net::TcpStream>> {
        let addr = (self.peer_node.host(), erl_dist::epmd::DEFAULT_EPMD_PORT);
        let stream = smol::net::TcpStream::connect(addr).await.or_fail()?;
        Ok(erl_dist::epmd::EpmdClient::new(stream))
    }

    async fn local_epmd_client(
        &self,
    ) -> orfail::Result<erl_dist::epmd::EpmdClient<smol::net::TcpStream>> {
        let addr = (self.local_node.host(), erl_dist::epmd::DEFAULT_EPMD_PORT);
        let stream = smol::net::TcpStream::connect(addr).await.or_fail()?;
        Ok(erl_dist::epmd::EpmdClient::new(stream))
    }
}

fn main() -> orfail::Result<()> {
    let args = Args::parse();
    smol::block_on(async {
        let peer_node = args
            .peer_epmd_client()
            .await?
            .get_node(&args.peer_node.name())
            .await
            .or_fail()?
            .or_fail()?;
        println!("Got peer node info: {:?}", peer_node);

        let dummy_listening_port = 3333;
        let local_node =
            erl_dist::epmd::NodeEntry::new_hidden(args.local_node.name(), dummy_listening_port);

        let (keepalive_connection, creation) = args
            .local_epmd_client()
            .await?
            .register(local_node.clone())
            .await
            .or_fail()?;
        println!("Registered self node: creation={:?}", creation);

        let stream = smol::net::TcpStream::connect((args.peer_node.host(), peer_node.port))
            .await
            .or_fail()?;
        let mut handshake = erl_dist::handshake::ClientSideHandshake::new(
            stream,
            erl_dist::node::LocalNode::new(args.local_node.clone(), creation),
            &args.cookie,
        );
        let _status = handshake
            .execute_send_name(erl_dist::LOWEST_DISTRIBUTION_PROTOCOL_VERSION)
            .await
            .or_fail()?;
        let (_, peer_node) = handshake.execute_rest(true).await.or_fail()?;
        println!("Handshake finished: peer={:?}", peer_node);

        std::mem::drop(keepalive_connection);
        Ok(())
    })
}
