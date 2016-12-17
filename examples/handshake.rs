// // http://erlang.org/doc/apps/erts/erl_dist_protocol.html
// extern crate erl_dist;
// extern crate fibers;
// extern crate futures;
// extern crate handy_async;
// extern crate clap;

// use clap::{App, Arg};
// use fibers::Executor;
// use futures::Future;
// // use handy_async::io::ReadFrom;
// // use handy_async::pattern::read::U8;

fn main() {
    // let matches = App::new("handshake")
    //     .arg(Arg::with_name("EPMD_HOST").short("h").takes_value(true).default_value("127.0.0.1"))
    //     .arg(Arg::with_name("EPMD_PORT").short("p").takes_value(true).default_value("4369"))
    //     .arg(Arg::with_name("NODE_NAME").short("n").takes_value(true).default_value("foo"))
    //     .arg(Arg::with_name("COOKIE")
    //         .short("c")
    //         .takes_value(true)
    //         .default_value("WPKYDIOSJIMJUURLRUHV"))
    //     .arg(Arg::with_name("LOCAL_NODE")
    //         .short("l")
    //         .takes_value(true)
    //         .default_value("bar@127.0.0.1"))
    //     .get_matches();
    // let node_name = matches.value_of("NODE_NAME").unwrap();
    // let local_node = matches.value_of("LOCAL_NODE").unwrap().to_string();
    // let cookie = matches.value_of("COOKIE").unwrap().to_string();
    // let epmd_host = matches.value_of("EPMD_HOST").unwrap();
    // let epmd_port = matches.value_of("EPMD_PORT").unwrap();
    // let epmd_addr = format!("{}:{}", epmd_host, epmd_port).parse().expect("Invalid epmd address");

    // let client = erl_dist::epmd::Client::new(&epmd_addr);
    // let mut executor = fibers::InPlaceExecutor::new().unwrap();

    // let info = {
    //     let monitor = executor.spawn_monitor(client.port_please(&node_name));
    //     executor.run_fiber(monitor)
    //         .unwrap()
    //         .expect("'port_please' request failed")
    //         .expect("Unknown node")
    // };

    // let (tx, rx) = fibers::sync::oneshot::channel();
    // // let client2 = client.clone();
    // // let local_node2 = local_node.to_string();
    // // let handle = executor.handle();
    // // executor.spawn(fibers::net::TcpListener::bind("127.0.0.1:0".parse().unwrap())
    // //     .and_then(move |listener| {
    // //         let local_addr = listener.local_addr().unwrap();
    // //         println!("# LISTEN: {}", local_addr);
    // //         client2.alive(local_node2, local_addr.port()).map(|alive| (listener, alive))
    // //     })
    // //     .and_then(move |(listener, alive)| {
    // //         println!("# ALIVE: {:?}", alive.creation);
    // //         let _ = tx.send(());
    // //         let handle = handle.clone();
    // //         listener.incoming().for_each(move |(socket, addr)| {
    // //             println!("# CONNECTED: {}", addr);
    // //             handle.spawn(socket.and_then(|socket| {
    // //                     U8.into_stream(socket).map_err(|e| e.into_error()).for_each(|b| {
    // //                         println!("# RECV: {}", b);
    // //                         Ok(())
    // //                     })
    // //                 })
    // //                 .map_err(|e| panic!("# E: {}", e)));
    // //             Ok(())
    // //         })
    // //     })
    // //     .then(|r| {
    // //         println!("DONE: {:?}", r);
    // //         Ok(())
    // //     }));
    // let _ = tx.send(());
    // let monitor = executor.spawn_monitor(rx.map_err(|e| panic!("E: {:?}", e))
    //     .and_then(move |_| {
    //         erl_dist::handshake::Handshake::connect(local_node.to_string(),
    //                                                 epmd_addr.ip(),
    //                                                 info,
    //                                                 cookie)
    //     }));
    // let _ = executor.run_fiber(monitor).unwrap().expect("Handshake failed");
}
