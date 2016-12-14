extern crate erl_dist;
extern crate fibers;
extern crate futures;
extern crate handy_async;
extern crate clap;
extern crate eetf;

use clap::{App, Arg};
use fibers::Executor;
use futures::Future;
use handy_async::io::WriteInto;
use handy_async::pattern::Endian;

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
    let node_name = matches.value_of("NODE_NAME").unwrap();
    let local_node = matches.value_of("LOCAL_NODE").unwrap().to_string();
    let cookie = matches.value_of("COOKIE").unwrap().to_string();
    let epmd_host = matches.value_of("EPMD_HOST").unwrap();
    let epmd_port = matches.value_of("EPMD_PORT").unwrap();
    let epmd_addr = format!("{}:{}", epmd_host, epmd_port).parse().expect("Invalid epmd address");
    let dest_proc = matches.value_of("DESTINATION").unwrap().to_string();

    let client = erl_dist::epmd::Client::new(&epmd_addr);
    let mut executor = fibers::InPlaceExecutor::new().unwrap();

    let info = {
        let monitor = executor.spawn_monitor(client.port_please(&node_name));
        executor.run_fiber(monitor)
            .unwrap()
            .expect("'port_please' request failed")
            .expect("Unknown node")
    };

    let node_name = local_node.to_string();
    let monitor =
        executor.spawn_monitor(erl_dist::handshake::Handshake::connect(local_node.to_string(),
                                                                       epmd_addr.ip(),
                                                                       info,
                                                                       cookie)
            .and_then(move |socket| {
                let ty = 112u8;
                let ctrl = eetf::Tuple {
                    elements: vec![From::from(eetf::FixInteger { value: 6 }),
                                   From::from(eetf::Pid {
                                       node: eetf::Atom { name: node_name },
                                       id: 0,
                                       serial: 0,
                                       creation: 0,
                                   }),
                                   From::from(eetf::Atom { name: "nil".to_string() }),
                                   From::from(eetf::Atom { name: dest_proc })],
                };
                let msg = eetf::Atom { name: "send_test".to_string() };
                let mut buf = vec![ty];
                eetf::Term::from(ctrl).encode(&mut buf).unwrap();
                eetf::Term::from(msg).encode(&mut buf).unwrap();
                println!("SEND: {:?}", buf);
                ((buf.len() as u32).be(), buf).write_into(socket).then(|r| {
                    println!("DONE: {:?}", r);
                    Ok(())
                })
            }));
    let _ = executor.run_fiber(monitor).unwrap().expect("Handshake failed");
}
