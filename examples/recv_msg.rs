extern crate erl_dist;
extern crate fibers;
extern crate futures;
extern crate handy_async;
extern crate clap;
extern crate eetf;

use clap::{App, Arg};
use fibers::{Executor, Spawn};
use futures::{Future, Stream};

fn main() {
    let matches = App::new("recv_msg")
        .arg(Arg::with_name("EPMD_HOST").short("h").takes_value(true).default_value("127.0.0.1"))
        .arg(Arg::with_name("EPMD_PORT").short("p").takes_value(true).default_value("4369"))
        .arg(Arg::with_name("COOKIE")
            .short("c")
            .takes_value(true)
            .default_value("WPKYDIOSJIMJUURLRUHV"))
        .arg(Arg::with_name("LOCAL_NODE")
            .short("l")
            .takes_value(true)
            .default_value("bar"))
        .get_matches();
    let local_node = matches.value_of("LOCAL_NODE").unwrap().to_string();
    let cookie = matches.value_of("COOKIE").unwrap().to_string();
    let epmd_host = matches.value_of("EPMD_HOST").unwrap();
    let epmd_port = matches.value_of("EPMD_PORT").unwrap();
    let epmd_addr = format!("{}:{}", epmd_host, epmd_port).parse().expect("Invalid epmd address");

    let client = erl_dist::epmd::EpmdClient::new();
    let mut executor = fibers::InPlaceExecutor::new().unwrap();
    let handle = executor.handle();
    let name = local_node.clone();
    let monitor =
        executor.spawn_monitor(fibers::net::TcpListener::bind("0.0.0.0:0".parse().unwrap())
            .and_then(move |listener| {
                let listen_addr = listener.local_addr().unwrap();
                println!("ADDR: {:?}", listen_addr);
                let info = erl_dist::epmd::NodeInfo::new(&local_node, listen_addr.port());
                fibers::net::TcpStream::connect(epmd_addr).and_then(move |socket| {
                    client.register(socket, info).map(|alive| (listener, alive))
                })
            })
            .and_then(move |(listener, alive)| {
                let port = listener.local_addr().unwrap().port();
                let creation = alive.1.clone();
                println!("# CREATION: {:?}", creation);
                listener.incoming()
                    .for_each(move |(peer, addr)| {
                        println!("# PEER: {:?}", addr);
                        let info = erl_dist::epmd::NodeInfo::new(&name, port);
                        let handshake = erl_dist::handshake::Handshake::new(info, &cookie);
                        // handshake.flags(erl_dist::handshake::DistributionFlags::empty());
                        handle.spawn(peer.and_then(move |peer| {
                                handshake.accept(peer)
                                    .map(|(socket, name, flags)| {
                                        println!("# NAME: {}", name);
                                        println!("# FLAGS: {:?}", flags);
                                        erl_dist::channel::receiver(socket)
                                    })
                                    .and_then(|rx| {
                                        rx.for_each(|msg| {
                                            println!("# RECV: {:?}", msg);
                                            Ok(())
                                        })
                                    })
                            })
                            .then(|r| {
                                println!("# DISCONNECTED: {:?}", r);
                                Ok(())
                            }));
                        Ok(())
                    })
                    .then(move |r| {
                        let _ = alive;
                        r
                    })
            }));
    let r = executor.run_fiber(monitor).unwrap().expect("Handshake failed");
    println!("# RESULT: {:?}", r);
}
