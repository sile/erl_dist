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
use erl_dist::epmd::{EpmdClient, NodeEntry};

#[derive(Debug, Parser)]
#[clap(name = "epmd_cli")]
struct Args {
    #[clap(long, short = 'h', default_value = "127.0.0.1")]
    epmd_host: String,

    #[clap(long, short = 'p', default_value_t = erl_dist::epmd::DEFAULT_EPMD_PORT)]
    epmd_port: u16,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Names,
    Dump,
    NodeEntry {
        node: String,
    },
    Kill,
    Register {
        name: String,

        #[clap(long, default_value_t = 3000)]
        port: u16,

        #[clap(long)]
        hidden: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    smol::block_on(async {
        let stream =
            smol::net::TcpStream::connect(format!("{}:{}", args.epmd_host, args.epmd_port)).await?;
        let client = EpmdClient::new(stream);

        match args.command {
            Command::Names => {
                // 'NAMES_REQ'
                let names = client.get_names().await?;
                let result = serde_json::json!(names
                    .into_iter()
                    .map(|(name, port)| serde_json::json!({"name": name, "port": port}))
                    .collect::<Vec<_>>());
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            Command::NodeEntry { node } => {
                // 'PORT_PLEASE2_REQ'
                if let Some(node) = client.get_node(&node).await? {
                    let result = serde_json::json!({
                        "name": node.name,
                        "port": node.port,
                        "node_type": format!("{:?} ({})", node.node_type, node.node_type as u8),
                        "protocol": format!("{:?} ({})", node.protocol, node.protocol as u8),
                        "highest_version": node.highest_version as u16,
                        "lowest_version": node.lowest_version as u16,
                         "extra": node.extra
                    });
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    anyhow::bail!("No such node: {:?}", node);
                }
            }
            Command::Dump => {
                // 'DUMP_REQ'
                let result = client.dump().await?;
                println!("{}", result);
            }
            Command::Kill => {
                // 'KILL_REQ'
                let result = client.kill().await?;
                let result = serde_json::json!({ "result": result });
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            Command::Register { name, port, hidden } => {
                // 'ALIVE2_REQ'
                let node = if hidden {
                    NodeEntry::new_hidden(&name, port)
                } else {
                    NodeEntry::new(&name, port)
                };
                let (_, creation) = client.register(node).await?;
                let result = serde_json::json!({
                    "creation": creation.get()
                });
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }
        Ok(())
    })
}
