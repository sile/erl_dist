//! EPMD client example.
//!
//! # Usage Examples
//!
//! ```bash
//! $ cargo run --example epmd_cli -- --help
//! $ cargo run --example epmd_cli names
//! $ cargo run --example epmd_cli node_info foo
//! ```
extern crate clap;
extern crate erl_dist;
extern crate fibers;
extern crate futures;

use clap::{App, Arg, SubCommand};
use erl_dist::epmd::NodeInfo;
use erl_dist::EpmdClient;
use fibers::net::TcpStream;
use fibers::{Executor, InPlaceExecutor, Spawn};
use futures::Future;

fn main() {
    let matches = App::new("epmd_cli")
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
        .subcommand(SubCommand::with_name("names"))
        .subcommand(SubCommand::with_name("dump"))
        .subcommand(
            SubCommand::with_name("node_info").arg(Arg::with_name("NODE").index(1).required(true)),
        )
        .subcommand(SubCommand::with_name("kill"))
        .subcommand(
            SubCommand::with_name("register")
                .arg(Arg::with_name("NAME").index(1).required(true))
                .arg(Arg::with_name("PORT").index(2).required(true))
                .arg(Arg::with_name("HIDDEN").long("hidden")),
        )
        .get_matches();
    let epmd_host = matches.value_of("EPMD_HOST").unwrap();
    let epmd_port = matches.value_of("EPMD_PORT").unwrap();
    let epmd_addr = format!("{}:{}", epmd_host, epmd_port)
        .parse()
        .expect("Invalid epmd address");

    let client = EpmdClient::new();
    let mut executor = InPlaceExecutor::new().unwrap();

    let connect = TcpStream::connect(epmd_addr);
    if let Some(_matches) = matches.subcommand_matches("names") {
        // 'NAMES_REQ'
        //
        let monitor =
            executor.spawn_monitor(connect.and_then(move |socket| client.get_names(socket)));
        let names = executor
            .run_fiber(monitor)
            .unwrap()
            .expect("'names' request failed");
        println!("Registered Names");
        println!("================\n");
        println!("{:?}", names);
    } else if let Some(_matches) = matches.subcommand_matches("dump") {
        // 'DUMP_REQ'
        //
        let monitor = executor.spawn_monitor(connect.and_then(move |socket| client.dump(socket)));
        let dump = executor
            .run_fiber(monitor)
            .unwrap()
            .expect("'dump' request failed");
        println!("Dump");
        println!("=====\n");
        println!("{:?}", dump);
    } else if let Some(matches) = matches.subcommand_matches("node_info") {
        // 'PORT_PLEASE2_REQ'
        //
        let node = matches.value_of("NODE").unwrap().to_string();
        let monitor = executor
            .spawn_monitor(connect.and_then(move |socket| client.get_node_info(socket, &node)));
        let info = executor
            .run_fiber(monitor)
            .unwrap()
            .expect("'node_info' request failed");
        println!("Node Info");
        println!("=========\n");
        println!("{:?}", info);
    } else if let Some(_matches) = matches.subcommand_matches("kill") {
        // 'KILL_REQ'
        //
        let monitor = executor.spawn_monitor(connect.and_then(move |socket| client.kill(socket)));
        let result = executor
            .run_fiber(monitor)
            .unwrap()
            .expect("'kill' request failed");
        println!("KILLED: {:?}", result);
    } else if let Some(matches) = matches.subcommand_matches("register") {
        // 'ALIVE2_REQ'
        //
        let name = matches.value_of("NAME").unwrap();
        let port = matches
            .value_of("PORT")
            .unwrap()
            .parse()
            .expect("Invalid port number");
        let hidden = matches.is_present("HIDDEN");
        let mut node = NodeInfo::new(name, port);
        if hidden {
            node.set_hidden();
        }
        let monitor =
            executor.spawn_monitor(connect.and_then(move |socket| client.register(socket, node)));
        let (_, creation) = executor
            .run_fiber(monitor)
            .unwrap()
            .expect("'register' request failed");
        println!("CREATION: {:?}", creation);
    } else {
        println!("{}", matches.usage());
    }
}
