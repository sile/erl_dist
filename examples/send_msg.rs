extern crate erl_dist;
extern crate fibers;
extern crate futures;
extern crate handy_async;
extern crate clap;
extern crate eetf;

use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use clap::{App, Arg};
use fibers::Executor;
use fibers::net::TcpStream;
use futures::Future;

fn main() {
    let matches = App::new("handshake")
        .arg(Arg::with_name("EPMD_HOST").short("h").takes_value(true).default_value("127.0.0.1"))
        .arg(Arg::with_name("EPMD_PORT").short("p").takes_value(true).default_value("4369"))
        .arg(Arg::with_name("NODE_NAME").short("n").takes_value(true).default_value("foo"))
        .arg(Arg::with_name("COOKIE")
            .short("c")
            .takes_value(true)
            .default_value("WPKYDIOSJIMJUURLRUHV"))
        .arg(Arg::with_name("LOCAL_NODE")
            .short("l")
            .takes_value(true)
            .default_value("bar@127.0.0.1"))
        .arg(Arg::with_name("DESTINATION")
            .short("d")
            .takes_value(true)
            .default_value("foo"))
        .get_matches();
    let node_name = matches.value_of("NODE_NAME").unwrap().to_string();
    let local_node = matches.value_of("LOCAL_NODE").unwrap().to_string();
    let cookie = matches.value_of("COOKIE").unwrap().to_string();
    let epmd_host = matches.value_of("EPMD_HOST").unwrap();
    let epmd_port = matches.value_of("EPMD_PORT").unwrap();
    let epmd_addr: SocketAddr =
        format!("{}:{}", epmd_host, epmd_port).parse().expect("Invalid epmd address");
    let dest_proc = matches.value_of("DESTINATION").unwrap().to_string();

    let client = erl_dist::epmd::EpmdClient::new();
    let mut executor = fibers::InPlaceExecutor::new().unwrap();

    let epmd_ip = epmd_addr.ip();
    let local_node0 = local_node.to_string();
    let monitor = executor.spawn_monitor(TcpStream::connect(epmd_addr)
        .and_then(move |socket| client.get_node_info(socket, &node_name))
        .and_then(move |info| {
            if let Some(addr) = info.map(|i| SocketAddr::new(epmd_ip, i.port)) {
                fibers::net::TcpStream::connect(addr)
                    .and_then(move |socket| {
                        let local_node = erl_dist::epmd::NodeInfo::new(&local_node, 3333);
                        let handshake = erl_dist::handshake::Handshake::new(local_node, &cookie);
                        handshake.connect(socket)
                    })
                    .boxed()
            } else {
                futures::failed(Error::new(ErrorKind::NotFound, "target node is not found")).boxed()
            }
        })
        .and_then(move |(socket, flags)| {
            use futures::Sink;
            println!("# Distribution Flags: {:?}", flags);
            let tx = erl_dist::channel::sender(socket);
            let from_pid = eetf::Pid::new(local_node0, 0, 0, 0);
            let message =
                erl_dist::message::RegSend::new(from_pid, dest_proc, eetf::Atom::from("send_test"));
            tx.send(From::from(message))
        }));
    let r = executor.run_fiber(monitor).unwrap().expect("Handshake failed");
    println!("# RESULT: {:?}", r);
}
