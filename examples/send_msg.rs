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
use orfail::OrFail;

fn main() -> noargs::Result<()> {
    let mut args = noargs::raw_args();
    args.metadata_mut().app_name = "send_msg";
    args.metadata_mut().app_description = "Send a message to an Erlang node";
    noargs::HELP_FLAG.take_help(&mut args);

    let local_node: erl_dist::node::NodeName = noargs::opt("local")
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
    let destination: String = noargs::opt("destination")
        .short('d')
        .default("foo")
        .doc("Destination process name")
        .take(&mut args)
        .then(|a| a.value().parse())?;
    let message: String = noargs::opt("message")
        .short('m')
        .default("hello_world")
        .doc("Message to send")
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

        let creation = erl_dist::node::Creation::random();
        let stream = smol::net::TcpStream::connect((peer_node.host(), peer_node_info.port))
            .await
            .or_fail()?;
        let local_node = erl_dist::node::LocalNode::new(local_node, creation);
        let mut handshake =
            erl_dist::handshake::ClientSideHandshake::new(stream, local_node.clone(), &cookie);
        let _status = handshake
            .execute_send_name(erl_dist::LOWEST_DISTRIBUTION_PROTOCOL_VERSION)
            .await
            .or_fail()?;
        let (connection, peer_node) = handshake.execute_rest(true).await.or_fail()?;
        println!("Handshake finished: peer={:?}", peer_node);

        let (mut tx, _) =
            erl_dist::message::channel(connection, local_node.flags & peer_node.flags);
        let pid = eetf::Pid::new(local_node.name.to_string(), 0, 0, creation.get());
        let msg = erl_dist::message::Message::reg_send(
            pid,
            eetf::Atom::from(destination),
            eetf::Atom::from(message).into(),
        );
        tx.send(msg).await.or_fail()?;

        Ok(())
    })
}
