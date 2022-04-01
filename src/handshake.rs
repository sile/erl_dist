//! Distribution Handshake.
//!
//! This handshake is used by an Erlang node for connecting to another one.
//!
//! See
//! [Distribution Handshake (Erlang Official Doc)](https://www.erlang.org/doc/apps/erts/erl_dist_protocol.html#distribution-handshake)
//! for more details.
use crate::node::{Creation, Node, NodeName};
use crate::socket::Socket;
use futures::io::{AsyncRead, AsyncWrite};

pub use self::flags::DistributionFlags;

// TODO: move(?)
pub const LOWEST_DISTRIBUTION_PROTOCOL_VERSION: u16 = 5;
pub const HIGHEST_DISTRIBUTION_PROTOCOL_VERSION: u16 = 6;

mod flags;

#[derive(Debug)]
pub struct Handshaked<T> {
    pub connection: T,
    pub local_node: Node,
    pub peer_node: Node,
}

// TODO: ClientSideHandshake
#[derive(Debug)]
pub struct HandshakeClient<T> {
    local_node: Node,
    local_challenge: Challenge,
    cookie: String,
    socket: Socket<T>,
}

impl<T> HandshakeClient<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(connection: T, mut local_node: Node, cookie: &str) -> Self {
        if local_node.creation.is_none() {
            local_node = Node {
                creation: Some(Creation::random()),
                ..local_node
            };
        }
        Self {
            local_node,
            local_challenge: Challenge::new(),
            cookie: cookie.to_owned(),
            socket: Socket::new(connection),
        }
    }

    pub async fn execute(mut self) -> Result<Handshaked<T>, HandshakeError> {
        log::debug!(
            "[client] handshake started: local_node={:?}, local_challenge={:?}",
            self.local_node,
            self.local_challenge
        );

        self.send_name().await?;
        log::debug!("[client] send_name");

        self.recv_status().await?;
        log::debug!("[client] recv_status");

        let (peer_node, peer_challenge) = self.recv_challenge().await?;
        log::debug!(
            "[client] recv_challenge: peer_node={:?}, peer_challenge={:?}",
            peer_node,
            peer_node
        );

        if peer_node.creation.is_some() {
            self.send_complement().await?;
            log::debug!("[client] send_complement");
        }

        self.send_challenge_reply(peer_challenge).await?;
        log::debug!("[client] send_challenge_reply");

        self.recv_challenge_ack().await?;
        log::debug!("[client] recv_challenge_ack");

        Ok(Handshaked {
            connection: self.socket.into_inner(),
            local_node: self.local_node,
            peer_node,
        })
    }

    async fn send_name(&mut self) -> Result<(), HandshakeError> {
        let mut writer = self.socket.message_writer();
        writer.write_u8(b'n')?;
        writer.write_u16(5)?;
        writer.write_u32(self.local_node.flags.bits() as u32)?;
        writer.write_all(self.local_node.name.to_string().as_bytes())?;
        writer.finish().await?;
        Ok(())
    }

    async fn recv_status(&mut self) -> Result<(), HandshakeError> {
        let mut reader = self.socket.message_reader().await?;
        let tag = reader.read_u8().await?;
        if tag != b's' {
            return Err(HandshakeError::UnexpectedTag {
                message: "STATUS",
                tag,
            });
        }
        let status = reader.read_string().await?;
        match status.as_str() {
            "ok" | "ok_simultaneous" => {}
            "nok" => {
                return Err(HandshakeError::OngoingHandshake);
            }
            "not_allowed" => {
                return Err(HandshakeError::NotAllowed);
            }
            "alive" => {
                // TODO: Add an option to send "true" instead.
                self.send_status("false").await?;
                return Err(HandshakeError::AlreadyActive);
            }
            "named:" => {
                // TODO:
                todo!();
            }
            _ => {
                return Err(HandshakeError::UnknownStatus { status });
            }
        }
        reader.finish().await?;
        Ok(())
    }

    async fn send_status(&mut self, status: &str) -> Result<(), HandshakeError> {
        let mut writer = self.socket.message_writer();
        writer.write_u8(b's')?;
        writer.write_all(status.as_bytes())?;
        Ok(())
    }

    async fn recv_challenge(&mut self) -> Result<(Node, Challenge), HandshakeError> {
        let mut reader = self.socket.message_reader().await?;
        let (node, challenge) = match reader.read_u8().await? {
            b'n' => {
                let version = reader.read_u16().await?;
                if version != 5 {
                    return Err(HandshakeError::InvalidVersionValue { value: version });
                }
                let flags =
                    DistributionFlags::from_bits_truncate(u64::from(reader.read_u32().await?));
                let challenge = Challenge(reader.read_u32().await?);
                let name = reader.read_string().await?.parse()?;
                let node = Node {
                    name,
                    flags,
                    creation: None,
                };
                (node, challenge)
            }
            b'N' => {
                let flags = DistributionFlags::from_bits_truncate(reader.read_u64().await?);
                let challenge = Challenge(reader.read_u32().await?);
                let creation = Creation::new(reader.read_u32().await?);
                let name = reader.read_u16_string().await?.parse()?;
                let node = Node {
                    name,
                    flags,
                    creation: Some(creation),
                };
                (node, challenge)
            }
            tag => {
                return Err(HandshakeError::UnexpectedTag {
                    message: "CHALLENGE",
                    tag,
                })
            }
        };
        reader.finish().await?;
        Ok((node, challenge))
    }

    async fn send_complement(&mut self) -> Result<(), HandshakeError> {
        let mut writer = self.socket.message_writer();
        writer.write_u8(b'c')?;
        writer.write_u32((self.local_node.flags.bits() >> 32) as u32)?;
        writer.write_u32(self.local_node.creation.expect("unreachable").get())?;
        writer.finish().await?;
        Ok(())
    }

    async fn send_challenge_reply(
        &mut self,
        peer_challenge: Challenge,
    ) -> Result<(), HandshakeError> {
        let mut writer = self.socket.message_writer();
        writer.write_u8(b'r')?;
        writer.write_u32(self.local_challenge.0)?;
        writer.write_all(&peer_challenge.digest(&self.cookie).0)?;
        writer.finish().await?;
        Ok(())
    }

    async fn recv_challenge_ack(&mut self) -> Result<(), HandshakeError> {
        let mut reader = self.socket.message_reader().await?;
        let tag = reader.read_u8().await?;
        if tag != b'a' {
            return Err(HandshakeError::UnexpectedTag {
                message: "CHALLENGE_ACK",
                tag,
            });
        }

        let mut digest = [0; 16];
        reader.read_exact(&mut digest).await?;
        if digest != self.local_challenge.digest(&self.cookie).0 {
            return Err(HandshakeError::InvalidDigest);
        }
        reader.finish().await?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct ServerSideHandshake<T> {
    local_node: Node,
    local_challenge: Challenge,
    cookie: String,
    socket: Socket<T>,
    peer_node: Option<Node>,
}

impl<T> ServerSideHandshake<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(connection: T, mut local_node: Node, cookie: &str) -> Self {
        if local_node.creation.is_none() {
            local_node = Node {
                creation: Some(Creation::random()),
                ..local_node
            };
        }

        Self {
            local_node,
            local_challenge: Challenge::new(),
            cookie: cookie.to_owned(),
            socket: Socket::new(connection),
            peer_node: None,
        }
    }

    pub async fn execute_recv_name(&mut self) -> Result<(NodeName, bool), HandshakeError> {
        let mut reader = self.socket.message_reader().await?;
        let tag = reader.read_u8().await?;
        let node = match tag {
            b'n' => {
                let version = reader.read_u16().await?;
                if version != 5 {
                    return Err(HandshakeError::InvalidVersionValue { value: version });
                }
                let flags =
                    DistributionFlags::from_bits_truncate(u64::from(reader.read_u32().await?));
                let name = reader.read_string().await?.parse()?;
                Node {
                    name,
                    flags,
                    creation: None,
                }
            }
            b'N' => {
                let flags = DistributionFlags::from_bits_truncate(reader.read_u64().await?);
                let creation = Creation::new(reader.read_u32().await?);
                let name = reader.read_u16_string().await?.parse()?;
                Node {
                    name,
                    flags,
                    creation: Some(creation),
                }
            }
            _ => {
                return Err(HandshakeError::UnexpectedTag {
                    message: "NAME",
                    tag,
                })
            }
        };
        reader.finish().await?;

        let name = node.name.clone();
        let is_dynamic = node.flags.contains(DistributionFlags::DFLAG_NAME_ME);
        self.peer_node = Some(node);
        Ok((name, is_dynamic))
    }

    pub async fn execute_rest(
        mut self,
        status: HandshakeStatus,
    ) -> Result<(T, Node), HandshakeError> {
        let (peer_flags, peer_creation) = if let Some(peer) = &self.peer_node {
            (peer.flags, peer.creation)
        } else {
            return Err(HandshakeError::PhaseError {
                current: "ServerSideHandshake::execute_rest()",
                depends_on: "ServerSideHandshake::execute_recv_name()",
            });
        };

        self.send_status(status).await?;

        self.send_challenge(peer_flags).await?;

        if peer_flags.contains(DistributionFlags::DFLAG_HANDSHAKE_23) && peer_creation.is_none() {
            self.recv_complement().await?;
        }

        let peer_challenge = self.recv_challenge_reply().await?;
        self.send_challenge_ack(peer_challenge).await?;

        let peer_node = self.peer_node.take().expect("unreachable");
        let connection = self.socket.into_inner();
        Ok((connection, peer_node))
    }

    async fn send_status(&mut self, status: HandshakeStatus) -> Result<(), HandshakeError> {
        let mut writer = self.socket.message_writer();
        writer.write_u8(b's')?;
        match &status {
            HandshakeStatus::Ok => writer.write_all(b"ok")?,
            HandshakeStatus::OkSimultaneous => writer.write_all(b"ok_simultaneous")?,
            HandshakeStatus::Nok => writer.write_all(b"nok")?,
            HandshakeStatus::NotAllowed => writer.write_all(b"not_allowed")?,
            HandshakeStatus::Alive => writer.write_all(b"alive")?,
            HandshakeStatus::Named(name) => {
                writer.write_all(b"named:")?;
                writer.write_u16(name.len() as u16)?;
                writer.write_all(name.as_bytes())?;
            }
        }
        writer.finish().await?;

        match status {
            HandshakeStatus::Nok => Err(HandshakeError::OngoingHandshake),
            HandshakeStatus::NotAllowed => Err(HandshakeError::NotAllowed),
            _ => Ok(()),
        }
    }

    async fn send_challenge(
        &mut self,
        peer_flags: DistributionFlags,
    ) -> Result<(), HandshakeError> {
        let mut writer = self.socket.message_writer();
        if peer_flags.contains(DistributionFlags::DFLAG_HANDSHAKE_23) {
            writer.write_u8(b'N')?;
            writer.write_u64(self.local_node.flags.bits())?;
            writer.write_u32(self.local_challenge.0)?;
            writer.write_u32(self.local_node.creation.expect("unreachable").get())?;
            writer.write_u16(self.local_node.name.len() as u16)?;
            writer.write_all(self.local_node.name.to_string().as_bytes())?;
        } else {
            writer.write_u8(b'n')?;
            writer.write_u16(5)?;
            writer.write_u32(self.local_node.flags.bits() as u32)?;
            writer.write_u32(self.local_challenge.0)?;
            writer.write_all(self.local_node.name.to_string().as_bytes())?;
        }
        writer.finish().await?;
        Ok(())
    }

    async fn recv_complement(&mut self) -> Result<(), HandshakeError> {
        let mut reader = self.socket.message_reader().await?;
        let tag = reader.read_u8().await?;
        if tag != b'c' {
            return Err(HandshakeError::UnexpectedTag {
                message: "send_complement",
                tag,
            });
        }
        let flags_high =
            DistributionFlags::from_bits_truncate(u64::from(reader.read_u32().await?) << 32);
        let creation = Creation::new(reader.read_u32().await?);
        reader.finish().await?;

        let peer = self.peer_node.as_mut().expect("unreachable");
        peer.flags |= flags_high;
        peer.creation = Some(creation);

        Ok(())
    }

    async fn recv_challenge_reply(&mut self) -> Result<Challenge, HandshakeError> {
        let mut reader = self.socket.message_reader().await?;
        let tag = reader.read_u8().await?;
        if tag != b'r' {
            return Err(HandshakeError::UnexpectedTag {
                message: "challenge_reply",
                tag,
            });
        }
        let peer_challenge = Challenge(reader.read_u32().await?);
        let mut digest = Digest([0; 16]);
        reader.read_exact(&mut digest.0).await?;
        reader.finish().await?;

        if self.local_challenge.digest(&self.cookie) != digest {
            return Err(HandshakeError::InvalidDigest);
        }

        Ok(peer_challenge)
    }

    async fn send_challenge_ack(
        &mut self,
        peer_challenge: Challenge,
    ) -> Result<(), HandshakeError> {
        let mut writer = self.socket.message_writer();
        writer.write_u8(b'a')?;
        writer.write_all(&peer_challenge.digest(&self.cookie).0)?;
        writer.finish().await?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HandshakeStatus {
    Ok,
    OkSimultaneous,
    Nok,
    NotAllowed,
    Alive,
    Named(String),
}

#[derive(Debug, Clone, Copy)]
struct Challenge(u32);

impl Challenge {
    fn new() -> Self {
        Self(rand::random())
    }

    fn digest(self, cookie: &str) -> Digest {
        Digest(md5::compute(&format!("{}{}", cookie, self.0)).0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Digest([u8; 16]);

#[derive(Debug, thiserror::Error)]
pub enum HandshakeError {
    #[error("peer already has an ongoing handshake with this node")]
    OngoingHandshake,

    #[error("the connection is disallowed for some (unspecified) security reason")]
    NotAllowed,

    #[error("a connection to the node is already active")]
    AlreadyActive,

    #[error("received an unknown status {status:?}")]
    UnknownStatus { status: String },

    #[error("received an unexpected tag {tag} for {message:?}")]
    UnexpectedTag { message: &'static str, tag: u8 },

    #[error("invalid digest value (maybe cookie mismatch)")]
    InvalidDigest,

    #[error("the 'version' value of an old 'send_name' message must be 5, but got {value}")]
    InvalidVersionValue { value: u16 },

    #[error("{current:?} was unexpectedly executed before {depends_on:?}")]
    PhaseError {
        current: &'static str,
        depends_on: &'static str,
    },

    #[error(transparent)]
    NodeNameError(#[from] crate::node::NodeNameError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

// TODO: add test
