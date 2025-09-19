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
use futures::stream::StreamExt;
use orfail::OrFail;

fn main() -> noargs::Result<()> {
    let mut args = noargs::raw_args();
    args.metadata_mut().app_name = "recv_msg";
    args.metadata_mut().app_description = "Receive messages from Erlang nodes";
    noargs::HELP_FLAG.take_help(&mut args);

    let local_node: erl_dist::node::NodeName = noargs::opt("local")
        .default("bar@localhost")
        .doc("Local node name")
        .take(&mut args)
        .then(|a| a.value().parse())?;
    let cookie: String = noargs::opt("cookie")
        .default("WPKYDIOSJIMJUURLRUHV")
        .doc("Erlang cookie")
        .take(&mut args)
        .then(|a| a.value().parse())?;
    let published: bool = noargs::flag("published")
        .doc("Add PUBLISHED distribution flag to the node (otherwise, the node runs as a hidden node)")
        .take(&mut args)
        .is_present();

    if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }

    smol::block_on(async {
        let listener = smol::net::TcpListener::bind("0.0.0.0:0").await.or_fail()?;
        let listening_port = listener.local_addr().or_fail()?.port();
        println!("Listening port: {}", listening_port);

        let local_node_entry = if published {
            erl_dist::epmd::NodeEntry::new(local_node.name(), listening_port)
        } else {
            erl_dist::epmd::NodeEntry::new_hidden(local_node.name(), listening_port)
        };

        let epmd_addr = (local_node.host(), erl_dist::epmd::DEFAULT_EPMD_PORT);
        let stream = smol::net::TcpStream::connect(epmd_addr).await.or_fail()?;
        let epmd_client = erl_dist::epmd::EpmdClient::new(stream);
        let (keepalive_connection, creation) =
            epmd_client.register(local_node_entry).await.or_fail()?;
        println!("Registered self node: creation={:?}", creation);

        let mut incoming = listener.incoming();
        while let Some(stream) = incoming.next().await.transpose().or_fail()? {
            let mut local_node = erl_dist::node::LocalNode::new(local_node.clone(), creation);
            if published {
                local_node.flags |= erl_dist::DistributionFlags::PUBLISHED;
            }
            let cookie = cookie.clone();
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
        Ok::<(), orfail::Failure>(())
    })?;

    Ok(())
}

async fn handle_client(
    local_node: erl_dist::node::LocalNode,
    cookie: String,
    stream: smol::net::TcpStream,
) -> orfail::Result<()> {
    let mut handshake =
        erl_dist::handshake::ServerSideHandshake::new(stream, local_node.clone(), &cookie);
    let status = if handshake.execute_recv_name().await.or_fail()?.is_some() {
        erl_dist::handshake::HandshakeStatus::Ok
    } else {
        // Dynamic name.
        erl_dist::handshake::HandshakeStatus::Named {
            name: "generated_name".to_owned(),
            creation: erl_dist::node::Creation::random(),
        }
    };
    let (stream, peer_node) = handshake.execute_rest(status).await.or_fail()?;
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
                let (msg, rx) = result.or_fail()?;
                println!("Recv: {:?}", msg);
                msg_future = Box::pin(rx.recv_owned());
            }
            futures::future::Either::Right((_, f)) => {
                msg_future = f;
            }
        }

        if smol::future::poll_once(&mut timer).await.is_some() {
            tx.send(erl_dist::message::Message::Tick).await.or_fail()?;
            timer.set_after(std::time::Duration::from_secs(30));
        }
    }
}
