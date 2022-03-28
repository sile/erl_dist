//! Client Node Example.
//!
//! The node sends a message (atom) to the specified erlang node.
//!
//! # Usage Examples
//!
//! ```bash
//! $ cargo run --example send_msg -- --help
//! $ cargo run --example send_msg -- --peer foo --destination foo --cookie erlang_cookie -m hello
//! ```
use clap::Parser;

#[derive(Debug, Parser)]
#[clap(name = "send_msg")]
struct Args {
    #[clap(long = "self", default_value = "bar@localhost")]
    self_node: erl_dist::node::NodeName,

    #[clap(long = "peer", default_value = "foo@localhost")]
    peer_node: erl_dist::node::NodeName,

    #[clap(long, default_value = "WPKYDIOSJIMJUURLRUHV")]
    cookie: String,

    #[clap(long, short, default_value = "foo")]
    destination: String,

    #[clap(long, short, default_value = "hello_world")]
    message: String,
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
            .get_node_info(&args.peer_node.name())
            .await?
            .ok_or_else(|| anyhow::anyhow!("no such node: {}", args.peer_node))?;
        println!("Got peer node info: {:?}", peer_node);

        let dummy_listening_port = 3333;
        let self_node =
            erl_dist::epmd::NodeInfoBuilder::new(&args.self_node.to_string(), dummy_listening_port)
                .hidden()
                .build();

        let mut self_node_for_epmd = self_node.clone(); // TODO
        self_node_for_epmd.name = args.self_node.name().to_owned();
        let (keepalive_socket, creation) = args
            .local_epmd_client()
            .await?
            .register(self_node_for_epmd)
            .await?;
        println!("Registered self node: creation={:?}", creation);

        let stream = smol::net::TcpStream::connect((args.peer_node.host(), peer_node.port)).await?;
        let handshake = erl_dist::handshake::Handshake::new(
            self_node,
            creation,
            erl_dist::handshake::DistributionFlags::default(),
            &args.cookie,
        );
        let (stream, peer_info) = handshake.connect(peer_node, stream).await?;
        println!("Handshake finished: peer={:?}", peer_info);

        let (mut tx, _) = erl_dist::channel::channel(stream, peer_info.flags);
        let pid = eetf::Pid::new(args.self_node.to_string(), 0, 0, creation.get());
        let msg = erl_dist::message::Message::reg_send(
            pid,
            eetf::Atom::from(args.destination),
            eetf::Atom::from(args.message).into(),
        );
        tx.send(msg).await?;
        std::mem::drop(keepalive_socket);
        Ok(())
    })
}
