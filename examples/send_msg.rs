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
    #[clap(long = "local", default_value = "bar@localhost")]
    local_node: erl_dist::node::NodeName,

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

        let creation = erl_dist::node::Creation::random();
        let stream = smol::net::TcpStream::connect((args.peer_node.host(), peer_node.port)).await?;
        let local_node = erl_dist::node::LocalNode::new(args.local_node.clone(), creation);
        let mut handshake =
            erl_dist::handshake::ClientSideHandshake::new(stream, local_node.clone(), &args.cookie);
        let _status = handshake
            .execute_send_name(erl_dist::LOWEST_DISTRIBUTION_PROTOCOL_VERSION)
            .await?;
        let (connection, peer_node) = handshake.execute_rest(true).await?;
        println!("Handshake finished: peer={:?}", peer_node);

        let (mut tx, _) =
            erl_dist::message::channel(connection, local_node.flags & peer_node.flags);
        let pid = eetf::Pid::new(args.local_node.to_string(), 0, 0, creation.get());
        let msg = erl_dist::message::Message::reg_send(
            pid,
            eetf::Atom::from(args.destination),
            eetf::Atom::from(args.message).into(),
        );
        tx.send(msg).await?;

        Ok(())
    })
}
