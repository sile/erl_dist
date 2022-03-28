//! Server Node Example.
//!
//! The node registers specified name to the EPMD and waits messages from other (connected) nodes.
//! If this node receives a message, it will print the message and discard it.
//!
//! # Usage Examples
//!
//! ```bash
//! $ cargo run --example recv_msg -- --help
//! $ cargo run --example recv_msg -- --name bar --cookie erlang_cookie
//!
//! # On another shell
//! $ erl -sname foo
//! > {bar, bar@localhost} ! hello.
//! ```
use clap::Parser;
use futures::stream::StreamExt;

#[derive(Debug, Parser)]
#[clap(name = "recv_msg")]
struct Args {
    #[clap(long = "self", default_value = "bar@localhost")]
    self_node: erl_dist::node::NodeName,

    #[clap(long, default_value = "WPKYDIOSJIMJUURLRUHV")]
    cookie: String,
}

impl Args {
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
        let listener = smol::net::TcpListener::bind("0.0.0.0:0").await?;
        let listening_port = listener.local_addr()?.port();
        println!("Listening port: {}", listening_port);

        let self_node =
            erl_dist::epmd::NodeInfoBuilder::new(&args.self_node.to_string(), listening_port)
                .build();

        let mut self_node_for_epmd = self_node.clone(); // TODO
        self_node_for_epmd.name = args.self_node.name().to_owned();
        let (keepalive_socket, creation) = args
            .local_epmd_client()
            .await?
            .register(self_node_for_epmd)
            .await?;
        println!("Registered self node: creation={:?}", creation);

        let mut incoming = listener.incoming();
        while let Some(stream) = incoming.next().await {
            let stream = stream?;
            let handshake = erl_dist::handshake::Handshake::new(
                self_node.clone(),
                creation,
                erl_dist::handshake::DistributionFlags::default(),
                &args.cookie,
            );
            let (stream, peer_info) = handshake.accept(stream).await?;
            println!("Connected: {:?}", peer_info);
        }

        std::mem::drop(keepalive_socket);
        Ok(())
    })
}
