//! EPMD client example.
//!
//! # Usage Examples
//!
//! ```bash
//! $ cargo run --example epmd_cli -- --help
//! $ cargo run --example epmd_cli names
//! $ cargo run --example epmd_cli node_entry foo
//! ```
use erl_dist::epmd::{EpmdClient, NodeEntry};

fn main() -> noargs::Result<()> {
    let mut args = noargs::raw_args();
    args.metadata_mut().app_name = "epmd_cli";
    args.metadata_mut().app_description = "EPMD client utility";
    noargs::HELP_FLAG.take_help(&mut args);

    let epmd_host: String = noargs::opt("host")
        .short('h')
        .default("127.0.0.1")
        .doc("EPMD host")
        .take(&mut args)
        .then(|a| a.value().parse())?;
    let epmd_port: u16 = noargs::opt("port")
        .short('p')
        .default("4369")
        .doc("EPMD port")
        .take(&mut args)
        .then(|a| a.value().parse())?;

    // Handle subcommands
    if noargs::cmd("names")
        .doc("List registered nodes")
        .take(&mut args)
        .is_present()
    {
        if let Some(help) = args.finish()? {
            print!("{help}");
            return Ok(());
        }

        smol::block_on(async {
            let stream =
                smol::net::TcpStream::connect(format!("{}:{}", epmd_host, epmd_port)).await?;
            let client = EpmdClient::new(stream);

            let names = client.get_names().await?;
            let result = nojson::json(|f| {
                f.set_indent_size(2);
                f.set_spacing(true);
                f.array(|f| {
                    for (name, port) in &names {
                        f.element(nojson::json(|f| {
                            f.object(|f| {
                                f.member("name", name.as_str())?;
                                f.member("port", *port)
                            })
                        }))?;
                    }
                    Ok(())
                })
            });
            println!("{result}");
            Ok::<(), Box<dyn std::error::Error>>(())
        })?;
    } else if noargs::cmd("dump")
        .doc("Dump all registered nodes")
        .take(&mut args)
        .is_present()
    {
        if let Some(help) = args.finish()? {
            print!("{help}");
            return Ok(());
        }

        smol::block_on(async {
            let stream =
                smol::net::TcpStream::connect(format!("{}:{}", epmd_host, epmd_port)).await?;
            let client = EpmdClient::new(stream);

            let result = client.dump().await?;
            println!("{}", result);
            Ok::<(), Box<dyn std::error::Error>>(())
        })?;
    } else if noargs::cmd("node_entry")
        .doc("Get node entry information")
        .take(&mut args)
        .is_present()
    {
        let node: String = noargs::arg("<NODE>")
            .doc("Node name to query")
            .take(&mut args)
            .then(|a| a.value().parse())?;

        if let Some(help) = args.finish()? {
            print!("{help}");
            return Ok(());
        }

        smol::block_on(async {
            let stream =
                smol::net::TcpStream::connect(format!("{}:{}", epmd_host, epmd_port)).await?;
            let client = EpmdClient::new(stream);

            let node_info = client.get_node(&node).await?.ok_or("node not found")?;
            let result = nojson::json(|f| {
                f.set_indent_size(2);
                f.set_spacing(true);
                f.object(|f| {
                    f.member("name", node_info.name.as_str())?;
                    f.member("port", node_info.port)?;
                    f.member(
                        "node_type",
                        format!("{:?} ({})", node_info.node_type, u8::from(node_info.node_type))
                            .as_str(),
                    )?;
                    f.member(
                        "protocol",
                        format!("{:?} ({})", node_info.protocol, u8::from(node_info.protocol))
                            .as_str(),
                    )?;
                    f.member("highest_version", node_info.highest_version)?;
                    f.member("lowest_version", node_info.lowest_version)?;
                    f.member("extra", node_info.extra.as_slice())
                })
            });
            println!("{result}");
            Ok::<(), Box<dyn std::error::Error>>(())
        })?;
    } else if noargs::cmd("kill")
        .doc("Kill EPMD daemon")
        .take(&mut args)
        .is_present()
    {
        if let Some(help) = args.finish()? {
            print!("{help}");
            return Ok(());
        }

        smol::block_on(async {
            let stream =
                smol::net::TcpStream::connect(format!("{}:{}", epmd_host, epmd_port)).await?;
            let client = EpmdClient::new(stream);

            let result = client.kill().await?;
            let result = nojson::json(|f| {
                f.set_indent_size(2);
                f.set_spacing(true);
                f.object(|f| f.member("result", result.as_str()))
            });
            println!("{result}");
            Ok::<(), Box<dyn std::error::Error>>(())
        })?;
    } else if noargs::cmd("register")
        .doc("Register a node with EPMD")
        .take(&mut args)
        .is_present()
    {
        let name: String = noargs::arg("<NAME>")
            .doc("Node name to register")
            .take(&mut args)
            .then(|a| a.value().parse())?;
        let port: u16 = noargs::opt("port")
            .default("3000")
            .doc("Port number")
            .take(&mut args)
            .then(|a| a.value().parse())?;
        let hidden: bool = noargs::flag("hidden")
            .doc("Register as hidden node")
            .take(&mut args)
            .is_present();

        if let Some(help) = args.finish()? {
            print!("{help}");
            return Ok(());
        }

        smol::block_on(async {
            let stream =
                smol::net::TcpStream::connect(format!("{}:{}", epmd_host, epmd_port)).await?;
            let client = EpmdClient::new(stream);

            let node = if hidden {
                NodeEntry::new_hidden(&name, port)
            } else {
                NodeEntry::new(&name, port)
            };
            let (_, creation) = client.register(node).await?;
            let result = nojson::json(|f| {
                f.set_indent_size(2);
                f.set_spacing(true);
                f.object(|f| f.member("creation", creation.get()))
            });
            println!("{result}");
            Ok::<(), Box<dyn std::error::Error>>(())
        })?;
    } else if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }

    Ok(())
}
