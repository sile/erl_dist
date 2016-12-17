// http://erlang.org/doc/apps/erts/erl_dist_protocol.html
extern crate erl_dist;
extern crate fibers;
extern crate futures;
extern crate handy_async;
extern crate clap;

use clap::{App, Arg, SubCommand};
use fibers::Executor;

fn main() {
    let matches = App::new("epmd_cli")
        .arg(Arg::with_name("EPMD_HOST").short("h").takes_value(true).default_value("127.0.0.1"))
        .arg(Arg::with_name("EPMD_PORT").short("p").takes_value(true).default_value("4369"))
        .subcommand(SubCommand::with_name("names"))
        .subcommand(SubCommand::with_name("dump"))
        .subcommand(SubCommand::with_name("port_please")
            .arg(Arg::with_name("NODE").index(1).required(true)))
        .subcommand(SubCommand::with_name("kill"))
        .subcommand(SubCommand::with_name("register")
            .arg(Arg::with_name("NAME").index(1).required(true))
            .arg(Arg::with_name("PORT").index(2).required(true))
            .arg(Arg::with_name("HIDDEN").long("hidden")))
        .get_matches();
    let epmd_host = matches.value_of("EPMD_HOST").unwrap();
    let epmd_port = matches.value_of("EPMD_PORT").unwrap();
    let epmd_addr = format!("{}:{}", epmd_host, epmd_port).parse().expect("Invalid epmd address");

    let client = erl_dist::epmd::EpmdClient::new(epmd_addr);
    let mut executor = fibers::InPlaceExecutor::new().unwrap();

    if let Some(_matches) = matches.subcommand_matches("names") {
        let monitor = executor.spawn_monitor(client.get_names());
        let names = executor.run_fiber(monitor).unwrap().expect("'names' request failed");
        println!("Registered Names");
        println!("================\n");
        println!("{:?}", names);
    } else if let Some(_matches) = matches.subcommand_matches("dump") {
        let monitor = executor.spawn_monitor(client.dump());
        let dump = executor.run_fiber(monitor).unwrap().expect("'dump' request failed");
        println!("Dump");
        println!("=====\n");
        println!("{:?}", dump);
    } else if let Some(matches) = matches.subcommand_matches("port_please") {
        let node = matches.value_of("NODE").unwrap();
        let monitor = executor.spawn_monitor(client.get_node_info(node));
        let info = executor.run_fiber(monitor).unwrap().expect("'port_please' request failed");
        println!("Node Info");
        println!("=========\n");
        println!("{:?}", info);
    } else if let Some(_matches) = matches.subcommand_matches("kill") {
        let monitor = executor.spawn_monitor(client.kill());
        let result = executor.run_fiber(monitor).unwrap().expect("'kill' request failed");
        println!("KILLED: {:?}", result);
    } else if let Some(matches) = matches.subcommand_matches("register") {
        let name = matches.value_of("NAME").unwrap();
        let port = matches.value_of("PORT").unwrap().parse().expect("Invalid port number");
        let hidden = matches.is_present("HIDDEN");
        let mut node = erl_dist::epmd::NodeInfo::new(name, port);
        if hidden {
            node.set_hidden();
        }
        let monitor = executor.spawn_monitor(client.register(node));
        let connection = executor.run_fiber(monitor).unwrap().expect("'register' request failed");
        println!("CREATION: {}", connection.creation());
    } else {
        println!("{}", matches.usage());
    }
}
