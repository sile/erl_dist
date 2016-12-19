//! EPMD protocol implementations.
//!
//! "EPMD" stands for "Erlang Port Mapper Daemon" and
//! it provides name resolution functionalities for distributed erlang nodes.
//!
//! See [12.1 EPMD Protocol](http://erlang.org/doc/apps/erts/erl_dist_protocol.html#id103099)
//! for more details about EPMD protocol.
use std::io::{Read, Write, Error};
use futures::{self, Future, BoxFuture};
use handy_async::pattern::{Pattern, Endian};
use handy_async::pattern::combinators::BE;
use handy_async::pattern::read::{U8, U16, U32, All, Utf8, LengthPrefixedBytes};
use handy_async::io::{ReadFrom, WriteInto, ExternalSize};

use Creation;

/// The default listening port of the EPMD.
pub const DEFAULT_EPMD_PORT: u16 = 4369;

const TAG_KILL_REQ: u8 = 107;
const TAG_PORT_PLEASE2_REQ: u8 = 122;
const TAG_PORT2_RESP: u8 = 119;
const TAG_NAMES_REQ: u8 = 110;
const TAG_DUMP_REQ: u8 = 100;
const TAG_ALIVE2_REQ: u8 = 120;
const TAG_ALIVE2_RESP: u8 = 121;

/// EPMD client.
///
/// This implements the client side of the EPMD protocol.
///
/// # Examples
///
/// Queries the information of the "foo" node:
///
/// ```no_run
/// # extern crate erl_dist;
/// # extern crate fibers;
/// # extern crate futures;
/// use fibers::{Executor, InPlaceExecutor};
/// use fibers::net::TcpStream;
/// use futures::Future;
/// use erl_dist::epmd::{DEFAULT_EPMD_PORT, EpmdClient};
///
/// # fn main() {
/// let epmd_addr = format!("127.0.0.1:{}", DEFAULT_EPMD_PORT).parse().unwrap();
/// let target_node = "foo";
/// let mut executor = InPlaceExecutor::new().unwrap();
///
/// // Queries the node information asynchronously.
/// let monitor = executor.spawn_monitor(TcpStream::connect(epmd_addr)
///     .and_then(move |socket| EpmdClient::new().get_node_info(socket, target_node)));
/// let result = executor.run_fiber(monitor).unwrap();
///
/// match result {
///     Err(e) => println!("Failed: {}", e),
///     Ok(None) => println!("Not found:"),
///     Ok(Some(info)) => println!("Found: {:?}", info),
/// }
/// # }
/// ```
///
/// See [epmd_cli.rs](https://github.com/sile/erl_dist/blob/master/examples/epmd_cli.rs) file
/// for more comprehensive examples.
#[derive(Debug)]
pub struct EpmdClient {
    _dummy: (),
}
impl EpmdClient {
    /// Makes a new `EpmdClient` instance.
    pub fn new() -> Self {
        EpmdClient { _dummy: () }
    }

    /// Registers a node in the EPMD connected by `stream`.
    ///
    /// The connection created to the EPMD must be kept as long as the node is a distributed node.
    /// When the connection is closed, the node is automatically unregistered from the EPMD.
    ///
    /// # Note
    ///
    /// For executing asynchronously, we assume that `stream` returns
    /// the `std::io::ErrorKind::WouldBlock` error if an I/O operation would be about to block.
    pub fn register<S>(&self, stream: S, node: NodeInfo) -> BoxFuture<(S, Creation), Error>
        where S: Read + Write + Send + 'static
    {
        futures::finished(stream)
            .and_then(move |stream| {
                let req = (TAG_ALIVE2_REQ,
                           node.port.be(),
                           node.node_type.as_u8(),
                           node.protocol.as_u8(),
                           node.highest_version.be(),
                           node.lowest_version.be(),
                           ((node.name.len() as u16).be(), node.name),
                           ((node.extra.len() as u16).be(), node.extra));
                with_len(req).write_into(stream)
            })
            .and_then(|(stream, _)| {
                let to_creation = |c| {
                    Creation::from_u16(c).ok_or_else(|| invalid_data!("Too large creation: {}", c))
                };
                (U8.expect_eq(TAG_ALIVE2_RESP), U8.expect_eq(0), U16.be().and_then(to_creation))
                    .read_from(stream)
            })
            .map(|(stream, (_, _, creation))| (stream, creation))
            .map_err(|e| e.into_error())
            .boxed()
    }

    /// Queries the distribution port (and other information) of
    /// the `node_name` node to the EPMD connected by `stream`.
    ///
    /// If the node has not been registered in the EPMD, this will return `None`.
    ///
    /// # Note
    ///
    /// For executing asynchronously, we assume that `stream` returns
    /// the `std::io::ErrorKind::WouldBlock` error if an I/O operation would be about to block.
    pub fn get_node_info<S>(&self, stream: S, node_name: &str) -> BoxFuture<Option<NodeInfo>, Error>
        where S: Read + Write + Send + 'static
    {
        let name = node_name.to_string();
        futures::finished((stream, ()))
            .and_then(|(stream, _)| with_len((TAG_PORT_PLEASE2_REQ, name)).write_into(stream))
            .and_then(|(stream, _)| {
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
                            node_type: NodeType::from(t.1),
                            protocol: Protocol::from(t.2),
                            highest_version: t.3,
                            lowest_version: t.4,
                            name: t.5,
                            extra: t.6,
                        }
                    });
                let resp = (U8.expect_eq(TAG_PORT2_RESP), U8)
                    .and_then(|(_, result)| { if result == 0 { Some(info) } else { None } });
                resp.read_from(stream)
            })
            .map(|(_, info)| info)
            .map_err(|e| e.into_error())
            .boxed()
    }

    /// Kills the EPMD connected by `stream`.
    ///
    /// This request kills the running EPMD.
    /// It is almost never used.
    ///
    /// If the EPMD is killed, it will returns `"OK"`.
    ///
    /// # Note
    ///
    /// For executing asynchronously, we assume that `stream` returns
    /// the `std::io::ErrorKind::WouldBlock` error if an I/O operation would be about to block.
    pub fn kill<S>(&self, stream: S) -> BoxFuture<String, Error>
        where S: Read + Write + Send + 'static
    {
        futures::finished((stream, ()))
            .and_then(|(stream, _)| with_len(TAG_KILL_REQ).write_into(stream))
            .and_then(|(stream, _)| Utf8(All).read_from(stream))
            .map(|(_, v)| v)
            .map_err(|e| e.into_error())
            .boxed()
    }

    /// Gets all registered names from the EPMD connected by `stream`.
    ///
    /// The result value is a string written for each active node.
    /// The format of each entry is `"name ${NODE_NAME} at port ${PORT}\n"`.
    ///
    /// # Note
    ///
    /// For executing asynchronously, we assume that `stream` returns
    /// the `std::io::ErrorKind::WouldBlock` error if an I/O operation would be about to block.
    pub fn get_names<S>(&self, stream: S) -> BoxFuture<String, Error>
        where S: Read + Write + Send + 'static
    {
        futures::finished((stream, ()))
            .and_then(|(stream, _)| with_len(TAG_NAMES_REQ).write_into(stream))
            .and_then(|(stream, _)| (U32.be(), Utf8(All)).read_from(stream))
            .map(|(_, (_, names))| names)
            .map_err(|e| e.into_error())
            .boxed()
    }

    /// Dumps all data from the EPMD connected by `stream`.
    ///
    /// This request is not really used, it is to be regarded as a debug feature.
    ///
    /// The result value is a string written for each node kept in the EPMD.
    ///
    /// The format of each entry is
    ///
    /// ```shell
    /// "active name ${NODE_NAME} at port ${PORT}, fd = ${FD}\n"
    /// ```
    ///
    /// or
    ///
    /// ```shell
    /// "old/unused name ${NODE_NAME} at port ${PORT}, fd = ${FD}\n"
    /// ```
    ///
    /// # Note
    ///
    /// For executing asynchronously, we assume that `stream` returns
    /// the `std::io::ErrorKind::WouldBlock` error if an I/O operation would be about to block.
    pub fn dump<S>(&self, stream: S) -> BoxFuture<String, Error>
        where S: Read + Write + Send + 'static
    {
        futures::finished((stream, ()))
            .and_then(|(stream, _)| with_len(TAG_DUMP_REQ).write_into(stream))
            .and_then(|(stream, _)| (U32.be(), Utf8(All)).read_from(stream))
            .map(|(_, (_, dump))| dump)
            .map_err(|e| e.into_error())
            .boxed()
    }
}

/// Information on a distributed node that registered in the EPMD.
#[derive(Debug, Clone)]
pub struct NodeInfo {
    /// The node name.
    pub name: String,

    /// The port number on which the node accept connection requests.
    pub port: u16,

    /// The node type.
    pub node_type: NodeType,

    /// The protocol for communicating with the node.
    pub protocol: Protocol,

    /// The highest distribution version that this node can handle.
    ///
    /// The value in Erlang/OTP R6B and later is 5.
    pub highest_version: u16,

    /// The lowest distribution version that this node can handle.
    ///
    /// The value in Erlang/OTP R6B and later is 5.
    pub lowest_version: u16,

    /// Extra field.
    pub extra: Vec<u8>,
}
impl NodeInfo {
    /// Makes a new `NodeInfo` with the default parameters.
    ///
    /// This is equivalent to the following code:
    ///
    /// ```
    /// # use erl_dist::epmd::{NodeInfo, NodeType, Protocol};
    /// # let name = "foo";
    /// # let port = 0;
    /// NodeInfo {
    ///     name: name.to_string(),
    ///     port: port,
    ///     node_type: NodeType::Normal,
    ///     protocol: Protocol::TcpIpV4,
    ///     highest_version: 5,
    ///     lowest_version: 5,
    ///     extra: Vec::new(),
    /// }
    /// # ;
    /// ```
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

    /// Sets the node type of this `NodeInfo` to `Hidden`.
    pub fn set_hidden(&mut self) -> &mut Self {
        self.node_type = NodeType::Hidden;
        self
    }
}

/// Protocol for communicating with a distributed node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Protocol {
    /// TCP/IPv4.
    TcpIpV4,

    /// Unknown protocol.
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

/// Type of a distributed node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeType {
    /// Hidden node (C-node).
    Hidden,

    /// Normal Erlang node.
    Normal,

    /// Unknown node type.
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

fn with_len<P: ExternalSize>(pattern: P) -> (BE<u16>, P) {
    ((pattern.external_size() as u16).be(), pattern)
}
