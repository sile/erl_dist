//! Client side handshake example.
//!
//! # Usage Examples
//!
//! ```bash
//! $ cargo run --example handshake -- --help
//! $ cargo run --example handshake -- --peer foo --self bar@localhost --cookie erlang_cookie
//! ```
use clap::Parser;
use erl_dist::{EpmdClient, Handshake};
use fibers::net::TcpStream;
use fibers::{Executor, InPlaceExecutor, Spawn};
use futures::future::Either;
use futures::Future;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;

#[derive(Debug, Parser)]
#[clap(name = "handshake")]
struct Args {
    #[clap(long, short = 'h', default_value = "127.0.0.1")]
    epmd_host: String,

    #[clap(long, short = 'p', default_value_t = 4369)]
    epmd_port: u16,

    #[clap(long = "peer", default_value = "foo")]
    peer_name: String,

    #[clap(long, default_value = "WPKYDIOSJIMJUURLRUHV")]
    cookie: String,

    #[clap(long = "self", default_value = "bar@localhost")]
    self_node: String,
}

fn main() {
    let args = Args::parse();
    let peer_name = args.peer_name;
    let self_node = args.self_node;
    let cookie = args.cookie;
    let epmd_host = args.epmd_host;
    let epmd_port = args.epmd_port;
    let epmd_addr: SocketAddr = format!("{}:{}", epmd_host, epmd_port)
        .parse()
        .expect("Invalid epmd address");

    let client = EpmdClient::new();
    let mut executor = InPlaceExecutor::new().unwrap();

    let monitor = executor.spawn_monitor(
        TcpStream::connect(epmd_addr.clone())
            .and_then(move |socket| client.get_node_info(socket, &peer_name))
            .and_then(move |info| {
                if let Some(addr) = info.map(|i| SocketAddr::new(epmd_addr.ip(), i.port)) {
                    Either::A(TcpStream::connect(addr).and_then(move |socket| {
                        let handshake = Handshake::new(&self_node, &cookie);
                        handshake.connect(socket)
                    }))
                } else {
                    Either::B(futures::failed(Error::new(
                        ErrorKind::NotFound,
                        "target node is not found",
                    )))
                }
            }),
    );
    let peer = executor
        .run_fiber(monitor)
        .unwrap()
        .expect("Handshake failed");
    println!("Name: {}", peer.name);
    println!("Flags: {:?}", peer.flags);
}
