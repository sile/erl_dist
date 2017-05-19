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
extern crate erl_dist;
extern crate fibers;
extern crate futures;
extern crate clap;
extern crate eetf;

use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use clap::{App, Arg};
use fibers::{Executor, InPlaceExecutor, Spawn};
use fibers::net::TcpStream;
use futures::{Sink, Future};
use futures::future::Either;
use erl_dist::{EpmdClient, Message, Handshake};
use erl_dist::channel;

fn main() {
    let matches = App::new("send_msg")
        .arg(Arg::with_name("EPMD_HOST")
                 .short("h")
                 .takes_value(true)
                 .default_value("127.0.0.1"))
        .arg(Arg::with_name("EPMD_PORT")
                 .short("p")
                 .takes_value(true)
                 .default_value("4369"))
        .arg(Arg::with_name("PEER_NAME")
                 .long("peer")
                 .takes_value(true)
                 .default_value("foo"))
        .arg(Arg::with_name("COOKIE")
                 .long("cookie")
                 .takes_value(true)
                 .default_value("WPKYDIOSJIMJUURLRUHV"))
        .arg(Arg::with_name("SELF_NODE")
                 .long("self")
                 .takes_value(true)
                 .default_value("bar@localhost"))
        .arg(Arg::with_name("DESTINATION")
                 .short("d")
                 .long("destination")
                 .takes_value(true)
                 .default_value("foo"))
        .arg(Arg::with_name("MESSAGE")
                 .short("m")
                 .long("message")
                 .takes_value(true)
                 .default_value("hello_world"))
        .get_matches();
    let peer_name = matches.value_of("PEER_NAME").unwrap().to_string();
    let self_node = matches.value_of("SELF_NODE").unwrap().to_string();
    let cookie = matches.value_of("COOKIE").unwrap().to_string();
    let epmd_host = matches.value_of("EPMD_HOST").unwrap();
    let epmd_port = matches.value_of("EPMD_PORT").unwrap();
    let epmd_addr: SocketAddr = format!("{}:{}", epmd_host, epmd_port)
        .parse()
        .expect("Invalid epmd address");
    let dest_proc = matches.value_of("DESTINATION").unwrap().to_string();
    let message = matches.value_of("MESSAGE").unwrap().to_string();

    let self_node0 = self_node.to_string();
    let mut executor = InPlaceExecutor::new().unwrap();
    let monitor = executor.spawn_monitor(TcpStream::connect(epmd_addr.clone())
                                             .and_then(move |epmd_socket| {
        // Gets peer node information from the EPMD
        EpmdClient::new().get_node_info(epmd_socket, &peer_name)
    })
                                             .and_then(move |info| {
        if let Some(addr) = info.map(|i| SocketAddr::new(epmd_addr.ip(), i.port)) {
            // Executes the client side handshake
            Either::A(TcpStream::connect(addr).and_then(move |socket| {
                                                            let handshake =
                                                                Handshake::new(&self_node, &cookie);
                                                            handshake.connect(socket)
                                                        }))
        } else {
            Either::B(futures::failed(Error::new(ErrorKind::NotFound, "target node is not found")))
        }
    })
                                             .and_then(move |peer| {
        // Sends a message to the peer node
        println!("# Connected: {}", peer.name);
        println!("# Distribution Flags: {:?}", peer.flags);
        let tx = channel::sender(peer.stream);
        let from_pid = eetf::Pid::new(self_node0, 0, 0, 0);
        let atom = eetf::Atom::from(message);
        let message = Message::reg_send(from_pid, dest_proc, atom);
        println!("# Send: {:?}", message);
        tx.send(message)
    }));
    let _ = executor.run_fiber(monitor).unwrap().expect("Failed");
    println!("# DONE");
}
