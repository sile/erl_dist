//! Client side handshake example.
//!
//! # Usage Examples
//!
//! ```bash
//! $ cargo run --example handshake -- --help
//! $ cargo run --example handshake -- --peer foo --self bar@localhost --cookie erlang_cookie
//! ```
use orfail::OrFail;

fn main() -> noargs::Result<()> {
    let mut args = noargs::raw_args();
    args.metadata_mut().app_name = "handshake";
    args.metadata_mut().app_description = "Client side handshake example";
    noargs::HELP_FLAG.take_help(&mut args);

    let local_node: erl_dist::node::NodeName = noargs::opt("self")
        .default("bar@localhost")
        .doc("Local node name")
        .take(&mut args)
        .then(|a| a.value().parse())?;
    let peer_node: erl_dist::node::NodeName = noargs::opt("peer")
        .default("foo@localhost")
        .doc("Peer node name")
        .take(&mut args)
        .then(|a| a.value().parse())?;
    let cookie: String = noargs::opt("cookie")
        .default("WPKYDIOSJIMJUURLRUHV")
        .doc("Erlang cookie")
        .take(&mut args)
        .then(|a| a.value().parse())?;

    if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }

    smol::block_on(async {
        let peer_node_info = {
            let addr = (peer_node.host(), erl_dist::epmd::DEFAULT_EPMD_PORT);
            let stream = smol::net::TcpStream::connect(addr).await.or_fail()?;
            let epmd_client = erl_dist::epmd::EpmdClient::new(stream);
            epmd_client
                .get_node(&peer_node.name())
                .await
                .or_fail()?
                .or_fail()?
        };
        println!("Got peer node info: {:?}", peer_node_info);

        let dummy_listening_port = 3333;
        let local_node_entry =
            erl_dist::epmd::NodeEntry::new_hidden(local_node.name(), dummy_listening_port);

        let (keepalive_connection, creation) = {
            let addr = (local_node.host(), erl_dist::epmd::DEFAULT_EPMD_PORT);
            let stream = smol::net::TcpStream::connect(addr).await.or_fail()?;
            let epmd_client = erl_dist::epmd::EpmdClient::new(stream);
            epmd_client.register(local_node_entry).await.or_fail()?
        };
        println!("Registered self node: creation={:?}", creation);

        let stream = smol::net::TcpStream::connect((peer_node.host(), peer_node_info.port))
            .await
            .or_fail()?;
        let mut handshake = erl_dist::handshake::ClientSideHandshake::new(
            stream,
            erl_dist::node::LocalNode::new(local_node.clone(), creation),
            &cookie,
        );
        let _status = handshake
            .execute_send_name(erl_dist::LOWEST_DISTRIBUTION_PROTOCOL_VERSION)
            .await
            .or_fail()?;
        let (_, peer_node) = handshake.execute_rest(true).await.or_fail()?;
        println!("Handshake finished: peer={:?}", peer_node);

        std::mem::drop(keepalive_connection);
        Ok::<(), orfail::Failure>(())
    })?;

    Ok(())
}
