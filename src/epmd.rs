//! EPMD protocol implementations.
//!
//! "EPMD" stands for "Erlang Port Mapper Daemon" and
//! it provides name resolution functionalities for distributed erlang nodes.
//!
//! See [EPMD Protocol](https://www.erlang.org/doc/apps/erts/erl_dist_protocol.html#epmd-protocol)
//! for more details about the protocol.
use crate::socket::Socket;
use crate::Creation;
use futures::io::{AsyncRead, AsyncWrite};
use std::str::FromStr;

/// The default listening port of the EPMD.
pub const DEFAULT_EPMD_PORT: u16 = 4369;

const TAG_DUMP_REQ: u8 = 100;
const TAG_KILL_REQ: u8 = 107;
const TAG_NAMES_REQ: u8 = 110;
const TAG_ALIVE2_X_RESP: u8 = 118;
const TAG_PORT2_RESP: u8 = 119;
const TAG_ALIVE2_REQ: u8 = 120;
const TAG_ALIVE2_RESP: u8 = 121;
const TAG_PORT_PLEASE2_REQ: u8 = 122;

#[derive(Debug, Clone)]
pub struct NodeName {
    pub name: String,
    pub port: u16,
}

impl FromStr for NodeName {
    type Err = EpmdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with("name ") {
            return Err(EpmdError::MalformedNodeNameLine);
        }

        let s = &s["name ".len()..];
        let pos = s
            .find(" at port ")
            .ok_or(EpmdError::MalformedNodeNameLine)?;
        let name = s[..pos].to_string();
        let port = s[pos + " at port ".len()..]
            .parse()
            .map_err(|_| EpmdError::MalformedNodeNameLine)?;
        Ok(Self { name, port })
    }
}

/// Type of a distributed node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NodeType {
    /// Hidden node (C-node).
    Hidden = 72,

    /// Normal Erlang node.
    Normal = 77,
}

impl TryFrom<u8> for NodeType {
    type Error = EpmdError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            72 => Ok(Self::Hidden),
            77 => Ok(Self::Normal),
            _ => Err(EpmdError::UnknownNodeType { value }),
        }
    }
}

/// Protocol for communicating with a distributed node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Protocol {
    /// TCP/IPv4.
    TcpIpV4 = 0,
}

impl TryFrom<u8> for Protocol {
    type Error = EpmdError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::TcpIpV4),
            _ => Err(EpmdError::UnknownProtocol { value }),
        }
    }
}

/// Node information.
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

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EpmdError {
    #[error("todo")]
    MalformedNodeNameLine,

    #[error("todo")]
    UnexpectedTag,

    #[error("todo")]
    GetNodeInfoError { code: u8 },

    #[error("todo")]
    UnknownNodeType { value: u8 },

    #[error("todo")]
    UnknownProtocol { value: u8 },

    #[error("todo")]
    UnknownResponseTag { tag: u8 },

    #[error("todo")]
    RegisterNodeError { code: u8 },

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug)]
pub struct EpmdClient<T> {
    socket: Socket<T>,
}

impl<T> EpmdClient<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(socket: T) -> Self {
        Self {
            socket: Socket::new(socket),
        }
    }

    /// Registers a node in EPMD.
    ///
    /// The connection created to the EPMD must be kept as long as the node is a distributed node.
    /// When the connection is closed, the node is automatically unregistered from the EPMD.
    pub async fn register(mut self, node: NodeInfo) -> Result<(T, Creation), EpmdError> {
        // Request.
        // TODO: validation
        self.socket.write_u16(1 + node.bytes_len() as u16).await?;
        self.socket.write_u8(TAG_ALIVE2_REQ).await?;
        self.socket.write_u16(node.port).await?;
        self.socket.write_u8(node.node_type as u8).await?;
        self.socket.write_u8(node.protocol as u8).await?;
        self.socket.write_u16(node.highest_version).await?;
        self.socket.write_u16(node.lowest_version).await?;
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
            tag => Err(EpmdError::UnknownResponseTag { tag }),
        }
    }

    /// Gets all registered names from EPMD.
    pub async fn get_names(mut self) -> Result<Vec<NodeName>, EpmdError> {
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
            .map(NodeName::from_str)
            .collect()
    }

    /// Gets the distribution port (and other information) of
    /// the `node_name` node from EPMD.
    ///
    /// If the node has not been registered in the connected EPMD, this method will return `None`.
    pub async fn get_node_info(mut self, node_name: &str) -> Result<Option<NodeInfo>, EpmdError> {
        // Request.
        // TODO: validation
        self.socket.write_u16((1 + node_name.len()) as u16).await?; // Length
        self.socket.write_u8(TAG_PORT_PLEASE2_REQ).await?;
        self.socket.write_all(node_name.as_bytes()).await?;
        self.socket.flush().await?;

        // Response.
        if self.socket.read_u8().await? != TAG_PORT2_RESP {
            return Err(EpmdError::UnexpectedTag);
        }

        match self.socket.read_u8().await? {
            0 => {}
            1 => {
                return Ok(None);
            }
            code => {
                return Err(EpmdError::GetNodeInfoError { code });
            }
        }

        Ok(Some(NodeInfo {
            port: self.socket.read_u16().await?,
            node_type: NodeType::try_from(self.socket.read_u8().await?)?,
            protocol: Protocol::try_from(self.socket.read_u8().await?)?,
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
