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
            .get_node(&args.peer_node.name())
            .await?
            .ok_or_else(|| anyhow::anyhow!("no such node: {}", args.peer_node))?;
        println!("Got peer node info: {:?}", peer_node);

        let dummy_listening_port = 3333;
        let self_node = erl_dist::epmd::NodeEntry::new_hidden(
            &args.self_node.to_string(),
            dummy_listening_port,
        );

        let mut self_node_for_epmd = self_node.clone(); // TODO
        self_node_for_epmd.name = args.self_node.name().to_owned();
        let (keepalive_socket, creation) = args
            .local_epmd_client()
            .await?
            .register(self_node_for_epmd)
            .await?;
        println!("Registered self node: creation={:?}", creation);

        let stream = smol::net::TcpStream::connect((args.peer_node.host(), peer_node.port)).await?;
        let mut handshake = erl_dist::handshake::ClientSideHandshake::new(
            stream,
            erl_dist::node::LocalNode::new(args.self_node.clone(), creation),
            &args.cookie,
        );
        let _status = handshake
            .execute_send_name(erl_dist::LOWEST_DISTRIBUTION_PROTOCOL_VERSION)
            .await?; // TODO:
        let (connection, peer_node) = handshake.execute_rest(true).await?;
        println!("Handshake finished: peer={:?}", peer_node);

        let (mut tx, _) = erl_dist::message::channel(connection, peer_node.flags);
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
