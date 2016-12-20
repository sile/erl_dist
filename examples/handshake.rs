//! Client side handshake example.
//!
//! # Usage Examples
//!
//! ```bash
//! $ cargo run --example handshake -- --help
//! $ cargo run --example handshake -- --peer foo --self bar@localhost --cookie erlang_cookie
//! ```
extern crate erl_dist;
extern crate fibers;
extern crate futures;
extern crate clap;

use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use clap::{App, Arg};
use fibers::{Executor, InPlaceExecutor};
use fibers::net::TcpStream;
use futures::Future;
use futures::future::Either;
use erl_dist::{Handshake, EpmdClient};

fn main() {
    let matches = App::new("handshake")
        .arg(Arg::with_name("EPMD_HOST").short("h").takes_value(true).default_value("127.0.0.1"))
        .arg(Arg::with_name("EPMD_PORT").short("p").takes_value(true).default_value("4369"))
        .arg(Arg::with_name("PEER_NAME").long("peer").takes_value(true).default_value("foo"))
        .arg(Arg::with_name("COOKIE")
            .short("c")
            .takes_value(true)
            .default_value("WPKYDIOSJIMJUURLRUHV"))
        .arg(Arg::with_name("SELF_NODE")
            .long("self")
            .takes_value(true)
            .default_value("bar@localhost"))
        .get_matches();
    let peer_name = matches.value_of("PEER_NAME").unwrap().to_string();
    let self_node = matches.value_of("SELF_NODE").unwrap().to_string();
    let cookie = matches.value_of("COOKIE").unwrap().to_string();
    let epmd_host = matches.value_of("EPMD_HOST").unwrap();
    let epmd_port = matches.value_of("EPMD_PORT").unwrap();
    let epmd_addr: SocketAddr =
        format!("{}:{}", epmd_host, epmd_port).parse().expect("Invalid epmd address");

    let client = EpmdClient::new();
    let mut executor = InPlaceExecutor::new().unwrap();

    let monitor = executor.spawn_monitor(TcpStream::connect(epmd_addr.clone())
        .and_then(move |socket| client.get_node_info(socket, &peer_name))
        .and_then(move |info| {
            if let Some(addr) = info.map(|i| SocketAddr::new(epmd_addr.ip(), i.port)) {
                Either::A(TcpStream::connect(addr).and_then(move |socket| {
                    let handshake = Handshake::new(&self_node, &cookie);
                    handshake.connect(socket)
                }))
            } else {
                Either::B(futures::failed(Error::new(ErrorKind::NotFound,
                                                     "target node is not found")))
            }
        }));
    let peer = executor.run_fiber(monitor).unwrap().expect("Handshake failed");
    println!("Name: {}", peer.name);
    println!("Flags: {:?}", peer.flags);
}
