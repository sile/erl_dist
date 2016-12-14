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
        .subcommand(SubCommand::with_name("port_please")
            .arg(Arg::with_name("NODE").index(1).required(true)))
        .get_matches();
    let epmd_host = matches.value_of("EPMD_HOST").unwrap();
    let epmd_port = matches.value_of("EPMD_PORT").unwrap();
    let epmd_addr = format!("{}:{}", epmd_host, epmd_port).parse().expect("Invalid epmd address");

    let client = erl_dist::epmd::Client::new(&epmd_addr);
    let mut executor = fibers::InPlaceExecutor::new().unwrap();

    if let Some(_matches) = matches.subcommand_matches("names") {
        let monitor = executor.spawn_monitor(client.names());
        let names = executor.run_fiber(monitor).unwrap().expect("'names' request failed");
        println!("Registered Names");
        println!("================\n");
        println!("{}", names);
    }
    if let Some(matches) = matches.subcommand_matches("port_please") {
        let node = matches.value_of("NODE").unwrap();
        let monitor = executor.spawn_monitor(client.port_please(&node));
        let info = executor.run_fiber(monitor).unwrap().expect("'port_please' request failed");
        println!("Node Info");
        println!("=========\n");
        println!("{:?}", info);
    } else {
        println!("{}", matches.usage());
    }
}
