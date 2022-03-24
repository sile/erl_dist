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
use erl_dist::epmd::{EpmdClient, NodeInfo};

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
    // Dump,
    NodeInfo { node: String },
    // Kill,
    // Register {
    //     name: String,
    //     port: u16,
    //     #[clap(long)]
    //     hidden: bool,
    // },
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    smol::block_on(async {
        let stream =
            smol::net::TcpStream::connect(format!("{}:{}", args.epmd_host, args.epmd_port)).await?;
        let client = EpmdClient::new(args.epmd_port, stream);

        match args.command {
            Command::Names => {
                // 'NAMES_REQ'
                let names = client.get_names().await?;
                let result = serde_json::json!(names
                    .into_iter()
                    .map(|n| serde_json::json!({"name": n.name, "port": n.port}))
                    .collect::<Vec<_>>());
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            Command::NodeInfo { node } => {
                // 'PORT_PLEASE2_REQ'
                if let Some(info) = client.get_node_info(&node).await? {
                    let result = serde_json::json!({
                        "name": info.name,
                        "port": info.port,
                        "node_type": format!("{:?} ({})", info.node_type, info.node_type as u8),
                        "protocol": format!("{:?} ({})", info.protocol, info.protocol as u8),
                        "highest_version": info.highest_version,
                        "lowest_version": info.lowest_version,
                        "extra": info.extra
                    });
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    anyhow::bail!("No such node: {:?}", node);
                }
            } // Command::Dump => {
              //     // 'DUMP_REQ'
              //     //
              //     let monitor =
              //         executor.spawn_monitor(connect.and_then(move |socket| client.dump(socket)));
              //     let dump = executor
              //         .run_fiber(monitor)
              //         .unwrap()
              //         .expect("'dump' request failed");
              //     println!("Dump");
              //     println!("=====\n");
              //     println!("{:?}", dump);
              // }
              // Command::Kill => {
              //     // 'KILL_REQ'
              //     //
              //     let monitor =
              //         executor.spawn_monitor(connect.and_then(move |socket| client.kill(socket)));
              //     let result = executor
              //         .run_fiber(monitor)
              //         .unwrap()
              //         .expect("'kill' request failed");
              //     println!("KILLED: {:?}", result);
              // }
              // Command::Register { name, port, hidden } => {
              //     // 'ALIVE2_REQ'
              //     //
              //     let mut node = NodeInfo::new(&name, port);
              //     if hidden {
              //         node.set_hidden();
              //     }
              //     let monitor = executor
              //         .spawn_monitor(connect.and_then(move |socket| client.register(socket, node)));
              //     let (_, creation) = executor
              //         .run_fiber(monitor)
              //         .unwrap()
              //         .expect("'register' request failed");
              //     println!("CREATION: {:?}", creation);
              // }
        }
        Ok(())
    })
}
