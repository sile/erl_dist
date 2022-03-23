//! EPMD client example.
//!
//! # Usage Examples
//!
//! ```bash
//! $ cargo run --example epmd_cli -- --help
//! $ cargo run --example epmd_cli names
//! $ cargo run --example epmd_cli node_info foo
//! ```
use clap::{Parser, Subcommand};
use erl_dist::epmd::NodeInfo;
use erl_dist::EpmdClient;
use fibers::net::TcpStream;
use fibers::{Executor, InPlaceExecutor, Spawn};
use futures::Future;

#[derive(Debug, Parser)]
#[clap(name = "epmd_cli")]
struct Args {
    #[clap(long, short = 'h', default_value = "127.0.0.1")]
    epmd_host: String,

    #[clap(long, short = 'p', default_value_t = 4369)]
    epmd_port: u16,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Names,
    Dump,
    NodeInfo {
        node: String,
    },
    Kill,
    Register {
        name: String,
        port: u16,
        #[clap(long)]
        hidden: bool,
    },
}

fn main() {
    let args = Args::parse();
    let epmd_host = args.epmd_host;
    let epmd_port = args.epmd_port;
    let epmd_addr = format!("{}:{}", epmd_host, epmd_port)
        .parse()
        .expect("Invalid epmd address");

    let client = EpmdClient::new();
    let mut executor = InPlaceExecutor::new().unwrap();

    let connect = TcpStream::connect(epmd_addr);
    match args.command {
        Command::Names => {
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
        }
        Command::Dump => {
            // 'DUMP_REQ'
            //
            let monitor =
                executor.spawn_monitor(connect.and_then(move |socket| client.dump(socket)));
            let dump = executor
                .run_fiber(monitor)
                .unwrap()
                .expect("'dump' request failed");
            println!("Dump");
            println!("=====\n");
            println!("{:?}", dump);
        }
        Command::NodeInfo { node } => {
            // 'PORT_PLEASE2_REQ'
            //
            let monitor = executor
                .spawn_monitor(connect.and_then(move |socket| client.get_node_info(socket, &node)));
            let info = executor
                .run_fiber(monitor)
                .unwrap()
                .expect("'node_info' request failed");
            println!("Node Info");
            println!("=========\n");
            println!("{:?}", info);
        }
        Command::Kill => {
            // 'KILL_REQ'
            //
            let monitor =
                executor.spawn_monitor(connect.and_then(move |socket| client.kill(socket)));
            let result = executor
                .run_fiber(monitor)
                .unwrap()
                .expect("'kill' request failed");
            println!("KILLED: {:?}", result);
        }
        Command::Register { name, port, hidden } => {
            // 'ALIVE2_REQ'
            //
            let mut node = NodeInfo::new(&name, port);
            if hidden {
                node.set_hidden();
            }
            let monitor = executor
                .spawn_monitor(connect.and_then(move |socket| client.register(socket, node)));
            let (_, creation) = executor
                .run_fiber(monitor)
                .unwrap()
                .expect("'register' request failed");
            println!("CREATION: {:?}", creation);
        }
    }
}
