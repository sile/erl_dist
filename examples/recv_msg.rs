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
use clap::{App, Arg};
use erl_dist::epmd::NodeInfo;
use erl_dist::{EpmdClient, Handshake};
use fibers::net::{TcpListener, TcpStream};
use fibers::{Executor, InPlaceExecutor, Spawn};
use futures::{Future, Stream};

fn main() {
    let matches = App::new("recv_msg")
        .arg(
            Arg::with_name("EPMD_HOST")
                .short("h")
                .takes_value(true)
                .default_value("127.0.0.1"),
        )
        .arg(
            Arg::with_name("EPMD_PORT")
                .short("p")
                .takes_value(true)
                .default_value("4369"),
        )
        .arg(
            Arg::with_name("COOKIE")
                .short("c")
                .long("cookie")
                .takes_value(true)
                .default_value("WPKYDIOSJIMJUURLRUHV"),
        )
        .arg(
            Arg::with_name("NODE_NAME")
                .long("name")
                .takes_value(true)
                .default_value("foo"),
        )
        .get_matches();
    let node_name = matches.value_of("NODE_NAME").unwrap().to_string();
    let cookie = matches.value_of("COOKIE").unwrap().to_string();
    let epmd_host = matches.value_of("EPMD_HOST").unwrap();
    let epmd_port = matches.value_of("EPMD_PORT").unwrap();
    let epmd_addr = format!("{}:{}", epmd_host, epmd_port)
        .parse()
        .expect("Invalid epmd address");

    let mut executor = InPlaceExecutor::new().unwrap();
    let handle = executor.handle();
    let full_name = format!("{}@localhost", node_name);
    let monitor = executor.spawn_monitor(
        TcpListener::bind("0.0.0.0:0".parse().unwrap())
            .and_then(move |listener| {
                // Registers the node name and the listening port to the EPMD
                let listen_addr = listener.local_addr().unwrap();
                println!("# Listen: {:?}", listen_addr);
                let info = NodeInfo::new(&node_name, listen_addr.port());
                TcpStream::connect(epmd_addr).and_then(move |socket| {
                    EpmdClient::new()
                        .register(socket, info)
                        .map(|alive| (listener, alive))
                })
            })
            .and_then(move |(listener, alive)| {
                let creation = alive.1.clone();
                println!("# Creation: {:?}", creation);
                listener
                    .incoming()
                    .for_each(move |(peer, addr)| {
                        // New peer is TCP connected
                        // Executes the sever side handshake
                        println!("# Peer Addr: {:?}", addr);
                        let handshake = Handshake::new(&full_name, &cookie);
                        handle.spawn(
                            peer.and_then(move |peer| {
                                handshake
                                    .accept(peer)
                                    .map(|peer| {
                                        println!("# Peer Name: {}", peer.name);
                                        println!("# Peer Flags: {:?}", peer.flags);
                                        erl_dist::channel::receiver(peer.stream)
                                    })
                                    .and_then(|rx| {
                                        // Prints received messages
                                        rx.for_each(|msg| {
                                            println!("# Recv: {:?}", msg);
                                            Ok(())
                                        })
                                    })
                            })
                            .then(|r| {
                                println!("# Disconnected: {:?}", r);
                                Ok(())
                            }),
                        );
                        Ok(())
                    })
                    .then(move |r| {
                        // NOTE: The connection to the EPMD must be kept during the node alive.
                        let _ = alive;
                        r
                    })
            }),
    );
    let _ = executor.run_fiber(monitor).unwrap().expect("Failed");
}
