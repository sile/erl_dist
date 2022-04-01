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
            erl_dist::epmd::NodeEntry::new_hidden(&args.self_node.to_string(), listening_port);

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
            let local_node = erl_dist::node::Node::new(args.self_node.clone(), creation);
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

        std::mem::drop(keepalive_socket);
        Ok(())
    })
}

async fn handle_client(
    local_node: erl_dist::node::Node,
    cookie: String,
    stream: smol::net::TcpStream,
) -> anyhow::Result<()> {
    let mut handshake = erl_dist::handshake::ServerSideHandshake::new(stream, local_node, &cookie);
    let (_peer_name, is_dynamic) = handshake.execute_recv_name().await?;
    if is_dynamic {
        todo!();
    }
    let (stream, peer_node) = handshake
        .execute_rest(erl_dist::handshake::HandshakeStatus::Ok)
        .await?;
    println!("Connected: {:?}", peer_node);

    let (mut tx, rx) = erl_dist::message::channel(stream, peer_node.flags);
    let mut timer = smol::Timer::after(std::time::Duration::from_secs(30)); // interval?
    let mut msg_future = Box::pin(rx.recv2());
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
                msg_future = Box::pin(rx.recv2());
            }
            futures::future::Either::Right((_, f)) => {
                msg_future = f;
            }
        }

        if smol::future::poll_once(&mut timer).await.is_some() {
            tx.tick().await?;
            timer.set_after(std::time::Duration::from_secs(30));
        }
    }
}
