use std::io::Error;
use std::net::SocketAddr;
use futures::{self, Future, BoxFuture};
use regex::Regex;
use fibers::net::TcpStream;
use handy_async::pattern::{Pattern, Endian};
use handy_async::pattern::combinators::BE;
use handy_async::pattern::read::{U8, U16, U32, All, Utf8, LengthPrefixedBytes};
use handy_async::io::{ReadFrom, WriteInto, ExternalSize, AsyncIoError};

pub const DEFAULT_EPMD_PORT: u16 = 4369;

const TAG_KILL_REQ: u8 = 107;
const TAG_PORT_PLEASE2_REQ: u8 = 122;
const TAG_PORT2_RESP: u8 = 119;
const TAG_NAMES_REQ: u8 = 110;
const TAG_DUMP_REQ: u8 = 100;
const TAG_ALIVE2_REQ: u8 = 120;
const TAG_ALIVE2_RESP: u8 = 121;

pub type Kill = BoxFuture<String, Error>;
pub type GetNodeInfo = BoxFuture<Option<NodeInfo>, Error>;
pub type Names = BoxFuture<Vec<Registered>, Error>;
pub type Dump = BoxFuture<String, Error>;
pub type Register = BoxFuture<Connection, Error>;

#[derive(Debug)]
pub struct Connection {
    socket: TcpStream,
    creation: u16,
}
impl Connection {
    pub fn creation(&self) -> u16 {
        self.creation
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registered {
    pub name: String,
    pub port: u16,
}

#[derive(Debug)]
pub struct EpmdClient {
    server_addr: SocketAddr,
}
impl EpmdClient {
    pub fn new(server_addr: SocketAddr) -> Self {
        EpmdClient { server_addr: server_addr }
    }

    pub fn register(&self, node: NodeInfo) -> Register {
        self.with_connect(|socket| {
            futures::finished((socket, ()))
                .and_then(move |(socket, _)| {
                    let pattern = (TAG_ALIVE2_REQ,
                                   node.port.be(),
                                   node.node_type.as_u8(),
                                   node.protocol.as_u8(),
                                   node.highest_version.be(),
                                   node.lowest_version.be(),
                                   (node.name.len() as u16).be(),
                                   node.name,
                                   (node.extra.len() as u16).be(),
                                   node.extra);
                    request(pattern).write_into(socket)
                })
                .and_then(|(socket, _)| {
                    (U8.expect_eq(TAG_ALIVE2_RESP), U8, U16.be()).read_from(socket)
                })
                .and_then(|(socket, (_, result, creation))| {
                    if result != 0 {
                        let e = invalid_data!("ALVIE2 request failed: result={}", result);
                        Err(AsyncIoError::new(socket, e))
                    } else {
                        Ok((socket.clone(),
                            Connection {
                            socket: socket,
                            creation: creation,
                        }))
                    }
                })
        })
    }

    pub fn kill(&self) -> Kill {
        self.with_connect(|socket| {
            futures::finished((socket, ()))
                .and_then(|(socket, _)| request(TAG_KILL_REQ).write_into(socket))
                .and_then(|(socket, _)| Utf8(All).read_from(socket))
        })
    }
    pub fn get_node_addr(&self, node_name: &str) -> BoxFuture<Option<SocketAddr>, Error> {
        let ip = self.server_addr.ip();
        self.get_node_info(node_name)
            .map(move |info| info.map(|info| SocketAddr::new(ip, info.port)))
            .boxed()
    }
    pub fn get_node_info(&self, node_name: &str) -> GetNodeInfo {
        let name = node_name.to_string();
        self.with_connect(|socket| {
            futures::finished((socket, ()))
                .and_then(|(socket, _)| request((TAG_PORT_PLEASE2_REQ, name)).write_into(socket))
                .and_then(|(socket, _)| {
                    let info = (U16.be(),
                                U8,
                                U8,
                                U16.be(),
                                U16.be(),
                                Utf8(LengthPrefixedBytes(U16.be())),
                                LengthPrefixedBytes(U16.be()))
                        .map(|t| {
                            NodeInfo {
                                port: t.0,
                                node_type: From::from(t.1),
                                protocol: From::from(t.2),
                                highest_version: t.3,
                                lowest_version: t.4,
                                name: t.5,
                                extra: t.6,
                            }
                        });
                    let resp = (U8.expect_eq(TAG_PORT2_RESP), U8)
                        .and_then(|(_, result)| { if result == 0 { Some(info) } else { None } });
                    resp.read_from(socket)
                })
        })
    }
    pub fn get_names(&self) -> Names {
        self.with_connect(|socket| {
            futures::finished((socket, ()))
                .and_then(|(socket, _)| request(TAG_NAMES_REQ).write_into(socket))
                .and_then(|(socket, _)| (U32.be(), Utf8(All)).read_from(socket))
                .and_then(|(socket, (_epmd_port, info))| {
                    let re = Regex::new("name (.+) at port ([0-9]+)").unwrap();
                    let names = re.captures_iter(&info)
                        .map(|cap| {
                            Ok(Registered {
                                name: cap.at(1).unwrap().to_string(),
                                port: cap.at(2)
                                    .unwrap()
                                    .parse()
                                    .map_err(|e| invalid_data!("Invalid port number: {}", e))?,
                            })
                        })
                        .collect::<Result<_, _>>();
                    match names {
                        Ok(names) => Ok((socket, names)),
                        Err(e) => Err(AsyncIoError::new(socket, e)),
                    }
                })
        })
    }
    pub fn dump(&self) -> Dump {
        self.with_connect(|socket| {
            futures::finished((socket, ()))
                .and_then(|(socket, _)| request(TAG_DUMP_REQ).write_into(socket))
                .and_then(|(socket, _)| {
                    (U32.be(), Utf8(All)).map(|(_, dump)| dump).read_from(socket)
                })
        })
    }

    fn with_connect<F, T, R>(&self, f: F) -> BoxFuture<T, Error>
        where F: FnOnce(TcpStream) -> R + Send + 'static,
              R: Future<Item = (TcpStream, T), Error = AsyncIoError<TcpStream>> + Send + 'static
    {
        TcpStream::connect(self.server_addr.clone())
            .and_then(|socket| f(socket).map(|(_, v)| v).map_err(|e| e.into_error()))
            .boxed()
    }
}

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub name: String,
    pub port: u16,
    pub node_type: NodeType,
    pub protocol: Protocol,
    pub highest_version: u16,
    pub lowest_version: u16,
    pub extra: Vec<u8>,
}
impl NodeInfo {
    pub fn new(name: &str, port: u16) -> Self {
        NodeInfo {
            name: name.to_string(),
            port: port,
            node_type: NodeType::Normal,
            protocol: Protocol::TcpIpV4,
            highest_version: 5,
            lowest_version: 5,
            extra: Vec::new(),
        }
    }
    pub fn set_hidden(&mut self) -> &mut Self {
        self.node_type = NodeType::Hidden;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Protocol {
    TcpIpV4,
    Unknown(u8),
}
impl Protocol {
    fn as_u8(&self) -> u8 {
        match *self {
            Protocol::TcpIpV4 => 0,
            Protocol::Unknown(b) => b,
        }
    }
}
impl From<u8> for Protocol {
    fn from(f: u8) -> Self {
        match f {
            0 => Protocol::TcpIpV4,
            _ => Protocol::Unknown(f),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeType {
    Hidden,
    Normal,
    Unknown(u8),
}
impl NodeType {
    fn as_u8(&self) -> u8 {
        match *self {
            NodeType::Hidden => 72,
            NodeType::Normal => 77,
            NodeType::Unknown(b) => b,
        }
    }
}
impl From<u8> for NodeType {
    fn from(f: u8) -> Self {
        match f {
            72 => NodeType::Hidden,
            77 => NodeType::Normal,
            _ => NodeType::Unknown(f),
        }
    }
}

fn request<P: ExternalSize>(pattern: P) -> (BE<u16>, P) {
    ((pattern.external_size() as u16).be(), pattern)
}
