//! Server Node Example.
//!
//! The node registers specified name to the EPMD and waits messages from other (connected) nodes.
//! If this node receives a message, it will print the message and discard it.
//!
//! # Usage Examples
//!
//! ```bash
//! $ cargo run --example recv_msg -- --help
//! $ cargo run --example recv_msg -- --local bar@localhost --cookie erlang_cookie
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
    #[clap(long = "local", default_value = "bar@localhost")]
    local_node: erl_dist::node::NodeName,

    #[clap(long, default_value = "WPKYDIOSJIMJUURLRUHV")]
    cookie: String,

    /// Add `PUBLISHED` distribution flag to the node (otherwise, the node runs as a hidden node).
    #[clap(long)]
    published: bool,
}

impl Args {
    async fn local_epmd_client(
        &self,
    ) -> anyhow::Result<erl_dist::epmd::EpmdClient<smol::net::TcpStream>> {
        let addr = (self.local_node.host(), erl_dist::epmd::DEFAULT_EPMD_PORT);
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

        let local_node_entry = if args.published {
            erl_dist::epmd::NodeEntry::new(args.local_node.name(), listening_port)
        } else {
            erl_dist::epmd::NodeEntry::new_hidden(args.local_node.name(), listening_port)
        };

        let (keepalive_connection, creation) = args
            .local_epmd_client()
            .await?
            .register(local_node_entry)
            .await?;
        println!("Registered self node: creation={:?}", creation);

        let mut incoming = listener.incoming();
        while let Some(stream) = incoming.next().await {
            let stream = stream?;
            let mut local_node = erl_dist::node::LocalNode::new(args.local_node.clone(), creation);
            if args.published {
                local_node.flags |= erl_dist::DistributionFlags::PUBLISHED;
            }
            let cookie = args.cookie.clone();
            smol::spawn(async move {
                match handle_client(local_node, cookie, stream).await {
                    Ok(()) => {
                        println!("Client disconnected");
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                    }
                };
            })
            .detach();
        }

        std::mem::drop(keepalive_connection);
        Ok(())
    })
}

async fn handle_client(
    local_node: erl_dist::node::LocalNode,
    cookie: String,
    stream: smol::net::TcpStream,
) -> anyhow::Result<()> {
    let mut handshake =
        erl_dist::handshake::ServerSideHandshake::new(stream, local_node.clone(), &cookie);
    let status = if handshake.execute_recv_name().await?.is_some() {
        erl_dist::handshake::HandshakeStatus::Ok
    } else {
        // Dynamic name.
        erl_dist::handshake::HandshakeStatus::Named {
            name: "generated_name".to_owned(),
            creation: erl_dist::node::Creation::random(),
        }
    };
    let (stream, peer_node) = handshake.execute_rest(status).await?;
    println!("Connected: {:?}", peer_node);

    let (mut tx, rx) = erl_dist::message::channel(stream, local_node.flags & peer_node.flags);
    let mut timer = smol::Timer::after(std::time::Duration::from_secs(30));
    let mut msg_future = Box::pin(rx.recv_owned());
    loop {
        let result = futures::future::select(
            msg_future,
            smol::Timer::after(std::time::Duration::from_secs(10)),
        )
        .await;
        match result {
            futures::future::Either::Left((result, _)) => {
                let (msg, rx) = result?;
                println!("Recv: {:?}", msg);
                msg_future = Box::pin(rx.recv_owned());
            }
            futures::future::Either::Right((_, f)) => {
                msg_future = f;
            }
        }

        if smol::future::poll_once(&mut timer).await.is_some() {
            tx.send(erl_dist::message::Message::Tick).await?;
            timer.set_after(std::time::Duration::from_secs(30));
        }
    }
}
