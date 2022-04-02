//! EPMD client and other EPMD related components.
//!
//! "EPMD" stands for "Erlang Port Mapper Daemon" and
//! it provides name resolution functionalities for distributed erlang nodes.
//!
//! See [EPMD Protocol (Erlang Official Doc)](https://www.erlang.org/doc/apps/erts/erl_dist_protocol.html#epmd-protocol)
//! for more details.
use crate::capability::{
    HIGHEST_DISTRIBUTION_PROTOCOL_VERSION, LOWEST_DISTRIBUTION_PROTOCOL_VERSION,
};
#[cfg(doc)]
use crate::node::NodeName;
use crate::node::{Creation, NodeType};
use crate::socket::Socket;
use crate::TransportProtocol;
use futures::io::{AsyncRead, AsyncWrite};
use std::str::FromStr;

/// Default EPMD listening port.
pub const DEFAULT_EPMD_PORT: u16 = 4369;

const TAG_DUMP_REQ: u8 = 100;
const TAG_KILL_REQ: u8 = 107;
const TAG_NAMES_REQ: u8 = 110;
const TAG_ALIVE2_X_RESP: u8 = 118;
const TAG_PORT2_RESP: u8 = 119;
const TAG_ALIVE2_REQ: u8 = 120;
const TAG_ALIVE2_RESP: u8 = 121;
const TAG_PORT_PLEASE2_REQ: u8 = 122;

/// Entry of a node registered in EPMD.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeEntry {
    /// Node name.
    ///
    /// Note that it differs from [`NodeName`] as this name doesn't contain the host part.
    pub name: String,

    /// Port number on which this node accepts connection requests.
    pub port: u16,

    /// Node type.
    pub node_type: NodeType,

    /// Transport protocol to communicate with this node.
    pub protocol: TransportProtocol,

    /// Highest distribution protocol version that this node can handle.
    pub highest_version: u16,

    /// Lowest distribution protocol version that this node can handle.
    pub lowest_version: u16,

    /// Extra field.
    pub extra: Vec<u8>,
}

impl NodeEntry {
    /// Makes a [`NodeEntry`] instance for a normal node.
    pub fn new(name: &str, port: u16) -> Self {
        Self {
            name: name.to_owned(),
            port,
            node_type: NodeType::Normal,
            protocol: TransportProtocol::TcpIpV4,
            highest_version: HIGHEST_DISTRIBUTION_PROTOCOL_VERSION,
            lowest_version: LOWEST_DISTRIBUTION_PROTOCOL_VERSION,
            extra: Vec::new(),
        }
    }

    /// Makes a [`NodeEntry`] instance for a hidden node.
    pub fn new_hidden(name: &str, port: u16) -> Self {
        Self {
            name: name.to_owned(),
            port,
            node_type: NodeType::Hidden,
            protocol: TransportProtocol::TcpIpV4,
            highest_version: HIGHEST_DISTRIBUTION_PROTOCOL_VERSION,
            lowest_version: LOWEST_DISTRIBUTION_PROTOCOL_VERSION,
            extra: Vec::new(),
        }
    }

    fn bytes_len(&self) -> usize {
        2 + self.name.len() + // name
        2 + // port
        1 + // node_type
        1 + // protocol
        2 + // highest_version
        2 + // lowest_version
        2 + self.extra.len() // extra
    }
}

/// Possible errors.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum EpmdError {
    /// Unknown response tag.
    #[error("received an unknown tag {tag} as the response of {request}")]
    UnknownResponseTag { request: &'static str, tag: u8 },

    /// Unknown node type.
    #[error("unknown node type {value}")]
    UnknownNodeType { value: u8 },

    /// Unknown transport protocol.
    #[error("unknown transport protocol {value}")]
    UnknownTransportProtocol { value: u8 },

    /// Too long request.
    #[error("request byte size must be less than 0xFFFF, but got {size} bytes")]
    TooLongRequest { size: usize },

    /// `PORT_PLEASE2_REQ` request failure.
    #[error("EPMD responded an error code {code} against a PORT_PLEASE2_REQ request")]
    GetNodeEntryError { code: u8 },

    /// `ALIVE2_REQ` request failure.
    #[error("EPMD responded an error code {code} against an ALIVE2_REQ request")]
    RegisterNodeError { code: u8 },

    /// Malformed `NAMES_RESP` line.
    #[error("found a malformed NAMES_RESP line: expected_format=\"name {{NAME}} at port {{PORT}}\", actual_line={line:?}")]
    MalformedNamesResponse { line: String },

    /// I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// EPMD client.
#[derive(Debug)]
pub struct EpmdClient<T> {
    socket: Socket<T>,
}

impl<T> EpmdClient<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    /// Makes a new [`EpmdClient`] instance.
    ///
    /// `socket` is a connection to communicate with the target EPMD server.
    pub fn new(socket: T) -> Self {
        Self {
            socket: Socket::new(socket),
        }
    }

    /// Registers a node in EPMD.
    ///
    /// The connection created to the EPMD must be kept as long as the node is a distributed node.
    /// When the connection is closed, the node is automatically unregistered from the EPMD.
    pub async fn register(mut self, node: NodeEntry) -> Result<(T, Creation), EpmdError> {
        // Request.
        let size = 1 + node.bytes_len();
        let size = u16::try_from(size).map_err(|_| EpmdError::TooLongRequest { size })?;
        self.socket.write_u16(size).await?;
        self.socket.write_u8(TAG_ALIVE2_REQ).await?;
        self.socket.write_u16(node.port).await?;
        self.socket.write_u8(node.node_type as u8).await?;
        self.socket.write_u8(node.protocol as u8).await?;
        self.socket.write_u16(node.highest_version as u16).await?;
        self.socket.write_u16(node.lowest_version as u16).await?;
        self.socket.write_u16(node.name.len() as u16).await?;
        self.socket.write_all(node.name.as_bytes()).await?;
        self.socket.write_u16(node.extra.len() as u16).await?;
        self.socket.write_all(&node.extra).await?;
        self.socket.flush().await?;

        // Response.
        match self.socket.read_u8().await? {
            TAG_ALIVE2_RESP => {
                match self.socket.read_u8().await? {
                    0 => {}
                    code => return Err(EpmdError::RegisterNodeError { code }),
                }

                let creation = Creation::new(u32::from(self.socket.read_u16().await?));
                Ok((self.socket.into_inner(), creation))
            }
            TAG_ALIVE2_X_RESP => {
                match self.socket.read_u8().await? {
                    0 => {}
                    code => return Err(EpmdError::RegisterNodeError { code }),
                }

                let creation = Creation::new(self.socket.read_u32().await?);
                Ok((self.socket.into_inner(), creation))
            }
            tag => Err(EpmdError::UnknownResponseTag {
                request: "ALIVE2_REQ",
                tag,
            }),
        }
    }

    /// Gets all registered nodes (name and port pairs) from EPMD.
    pub async fn get_names(mut self) -> Result<Vec<(String, u16)>, EpmdError> {
        // Request.
        self.socket.write_u16(1).await?; // Length
        self.socket.write_u8(TAG_NAMES_REQ).await?;
        self.socket.flush().await?;

        // Response.
        let _epmd_port = self.socket.read_u32().await?;
        let node_info_text = self.socket.read_string().await?;

        node_info_text
            .split('\n')
            .filter(|s| !s.is_empty())
            .map(|line| NodeNameAndPort::from_str(line).map(|x| (x.name, x.port)))
            .collect()
    }

    /// Queries the node which has the given name to EPMD.
    ///
    /// If the node has not been registered in the connected EPMD, this method will return `None`.
    pub async fn get_node(mut self, node_name: &str) -> Result<Option<NodeEntry>, EpmdError> {
        // Request.
        let size = 1 + node_name.len();
        let size = u16::try_from(size).map_err(|_| EpmdError::TooLongRequest { size })?;
        self.socket.write_u16(size).await?;
        self.socket.write_u8(TAG_PORT_PLEASE2_REQ).await?;
        self.socket.write_all(node_name.as_bytes()).await?;
        self.socket.flush().await?;

        // Response.
        let tag = self.socket.read_u8().await?;
        if tag != TAG_PORT2_RESP {
            return Err(EpmdError::UnknownResponseTag {
                request: "NAMES_REQ",
                tag,
            });
        }

        match self.socket.read_u8().await? {
            0 => {}
            1 => {
                return Ok(None);
            }
            code => {
                return Err(EpmdError::GetNodeEntryError { code });
            }
        }

        Ok(Some(NodeEntry {
            port: self.socket.read_u16().await?,
            node_type: NodeType::try_from(self.socket.read_u8().await?)?,
            protocol: TransportProtocol::try_from(self.socket.read_u8().await?)?,
            highest_version: self.socket.read_u16().await?,
            lowest_version: self.socket.read_u16().await?,
            name: self.socket.read_u16_string().await?,
            extra: self.socket.read_u16_bytes().await?,
        }))
    }

    /// Kills EPMD.
    ///
    /// This request kills the running EPMD.
    /// It is almost never used.
    ///
    /// If EPMD is killed, this method returns `"OK"`.
    pub async fn kill(mut self) -> Result<String, EpmdError> {
        // Request.
        self.socket.write_u16(1).await?;
        self.socket.write_u8(TAG_KILL_REQ).await?;
        self.socket.flush().await?;

        // Response.
        let result = self.socket.read_string().await?;
        Ok(result)
    }

    /// Dumps all data from EPMD.
    ///
    /// This request is not really used, it is to be regarded as a debug feature.
    ///
    /// The result value is a string written for each node kept in the connected EPMD.
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
    pub async fn dump(mut self) -> Result<String, EpmdError> {
        // Request.
        self.socket.write_u16(1).await?;
        self.socket.write_u8(TAG_DUMP_REQ).await?;
        self.socket.flush().await?;

        // Response.
        let _epmd_port = self.socket.read_u32().await?;
        let info = self.socket.read_string().await?;
        Ok(info)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct NodeNameAndPort {
    name: String,
    port: u16,
}

impl FromStr for NodeNameAndPort {
    type Err = EpmdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let error = || EpmdError::MalformedNamesResponse { line: s.to_owned() };

        if !s.starts_with("name ") {
            return Err(error());
        }

        let s = &s["name ".len()..];
        let pos = s.find(" at port ").ok_or_else(error)?;
        let name = s[..pos].to_string();
        let port = s[pos + " at port ".len()..].parse().map_err(|_| error())?;
        Ok(Self { name, port })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn epmd_client_works() {
    //     let node_name = "erl_dist_test";
    //     smol::block_on(async {
    //         let erl_node = crate::tests::TestErlangNode::new(node_name)
    //             .await
    //             .expect("failed to run a test erlang node");

    //         // Get the information of an existing Erlang node.
    //         let node = crate::tests::epmd_client()
    //             .await
    //             .get_node(node_name)
    //             .await
    //             .expect("failed to get node");
    //         let node = node.expect("no such node");
    //         assert_eq!(node.name, node_name);

    //         // Register a new node.
    //         let client = crate::tests::epmd_client().await;
    //         let new_node_name = "erl_dist_test_new_node";
    //         let new_node = NodeEntry::new_hidden(new_node_name, 3000);
    //         let (stream, _creation) = client
    //             .register(new_node)
    //             .await
    //             .expect("failed to register a new node");

    //         // Get the information of the newly added Erlang node.
    //         let node = crate::tests::epmd_client()
    //             .await
    //             .get_node(new_node_name)
    //             .await
    //             .expect("failed to get node");
    //         let node = node.expect("no such node");
    //         assert_eq!(node.name, new_node_name);

    //         // Deregister the node.
    //         std::mem::drop(stream);
    //         let node = crate::tests::epmd_client()
    //             .await
    //             .get_node(new_node_name)
    //             .await
    //             .expect("failed to get node");
    //         assert!(node.is_none());

    //         std::mem::drop(erl_node);
    //     });
    // }
}
