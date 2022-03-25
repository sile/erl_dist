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
use erl_dist::epmd::{EpmdClient, HandshakeProtocolVersion, NodeInfo, NodeType, Protocol};

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
    NodeInfo {
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
                        "highest_version": info.highest_version as u16,
                        "lowest_version": info.lowest_version as u16,
                        "extra": info.extra
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
                let node = NodeInfo {
                    name: name.to_string(),
                    port,
                    node_type: if hidden {
                        NodeType::Hidden
                    } else {
                        NodeType::Normal
                    },
                    protocol: Protocol::TcpIpV4,
                    highest_version: HandshakeProtocolVersion::V6,
                    lowest_version: HandshakeProtocolVersion::V5,
                    extra: Vec::new(),
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
