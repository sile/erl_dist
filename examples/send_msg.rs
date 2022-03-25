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
// use clap::Parser;
// use erl_dist::channel;
// use erl_dist::{EpmdClient, Handshake, Message};
// use fibers::net::TcpStream;
// use fibers::{Executor, InPlaceExecutor, Spawn};
// use futures::future::Either;
// use futures::{Future, Sink};
// use std::io::{Error, ErrorKind};
// use std::net::SocketAddr;

// #[derive(Debug, Parser)]
// #[clap(name = "send_msg")]
// struct Args {
//     #[clap(long, short = 'h', default_value = "127.0.0.1")]
//     epmd_host: String,

//     #[clap(long, short = 'p', default_value_t = 4369)]
//     epmd_port: u16,

//     #[clap(long = "peer", default_value = "foo")]
//     peer_name: String,

//     #[clap(long, default_value = "WPKYDIOSJIMJUURLRUHV")]
//     cookie: String,

//     #[clap(long = "self", default_value = "bar@localhost")]
//     self_node: String,

//     #[clap(long, short, default_value = "foo")]
//     destination: String,

//     #[clap(long, short, default_value = "hello_world")]
//     message: String,
// }

fn main() {
    // let args = Args::parse();
    // let peer_name = args.peer_name;
    // let self_node = args.self_node;
    // let cookie = args.cookie;
    // let epmd_host = args.epmd_host;
    // let epmd_port = args.epmd_port;
    // let epmd_addr: SocketAddr = format!("{}:{}", epmd_host, epmd_port)
    //     .parse()
    //     .expect("Invalid epmd address");
    // let dest_proc = args.destination;
    // let message = args.message;

    // let self_node0 = self_node.to_string();
    // let mut executor = InPlaceExecutor::new().unwrap();
    // let monitor = executor.spawn_monitor(
    //     TcpStream::connect(epmd_addr.clone())
    //         .and_then(move |epmd_socket| {
    //             // Gets peer node information from the EPMD
    //             EpmdClient::new().get_node_info(epmd_socket, &peer_name)
    //         })
    //         .and_then(move |info| {
    //             if let Some(addr) = info.map(|i| SocketAddr::new(epmd_addr.ip(), i.port)) {
    //                 // Executes the client side handshake
    //                 Either::A(TcpStream::connect(addr).and_then(move |socket| {
    //                     let handshake = Handshake::new(&self_node, &cookie);
    //                     handshake.connect(socket)
    //                 }))
    //             } else {
    //                 Either::B(futures::failed(Error::new(
    //                     ErrorKind::NotFound,
    //                     "target node is not found",
    //                 )))
    //             }
    //         })
    //         .and_then(move |peer| {
    //             // Sends a message to the peer node
    //             println!("# Connected: {}", peer.name);
    //             println!("# Distribution Flags: {:?}", peer.flags);
    //             let tx = channel::sender(peer.stream);
    //             let from_pid = eetf::Pid::new(self_node0, 0, 0, 0);
    //             let atom = eetf::Atom::from(message);
    //             let message = Message::reg_send(from_pid, dest_proc, atom);
    //             println!("# Send: {:?}", message);
    //             tx.send(message)
    //         }),
    // );
    // let _ = executor.run_fiber(monitor).unwrap().expect("Failed");
    // println!("# DONE");
}
