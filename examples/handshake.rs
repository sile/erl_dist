// http://erlang.org/doc/apps/erts/erl_dist_protocol.html
extern crate erl_dist;
extern crate fibers;
extern crate futures;
extern crate handy_async;
extern crate clap;

use std::io::{Error, ErrorKind};
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
        .get_matches();
    let node_name = matches.value_of("NODE_NAME").unwrap();
    let local_node = matches.value_of("LOCAL_NODE").unwrap().to_string();
    let cookie = matches.value_of("COOKIE").unwrap().to_string();
    let epmd_host = matches.value_of("EPMD_HOST").unwrap();
    let epmd_port = matches.value_of("EPMD_PORT").unwrap();
    let epmd_addr = format!("{}:{}", epmd_host, epmd_port).parse().expect("Invalid epmd address");

    let client = erl_dist::epmd::EpmdClient::new(epmd_addr);
    let mut executor = fibers::InPlaceExecutor::new().unwrap();

    let monitor = executor.spawn_monitor(client.get_node_addr(node_name).and_then(|addr| {
        if let Some(addr) = addr {
            TcpStream::connect(addr)
                .and_then(move |socket| {
                    let local_node = erl_dist::epmd::NodeInfo::new(&local_node, 3333);
                    let handshake = erl_dist::handshake::Handshake::new(local_node, &cookie);
                    handshake.connect(socket)
                })
                .boxed()
        } else {
            futures::failed(Error::new(ErrorKind::NotFound, "target node is not found")).boxed()
        }
    }));
    let r = executor.run_fiber(monitor).unwrap().expect("Handshake failed");
    println!("R: {:?}", r);
}
