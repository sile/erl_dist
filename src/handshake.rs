//! Distribution Handshake.
//!
//! This handshake is used by an Erlang node for connecting to another one.
//!
//! See
//! [Distribution Handshake (Erlang Official Doc)](https://www.erlang.org/doc/apps/erts/erl_dist_protocol.html#distribution-handshake)
//! for more details.
use crate::epmd::NodeEntry;
use crate::node::{Creation, Node, NodeName};
use crate::socket::Socket;
use futures::io::{AsyncRead, AsyncWrite};

pub use self::flags::DistributionFlags;

pub const LOWEST_DISTRIBUTION_PROTOCOL_VERSION: u16 = 5;
pub const HIGHEST_DISTRIBUTION_PROTOCOL_VERSION: u16 = 6;

mod flags;

#[derive(Debug)]
pub struct Handshaked<T> {
    pub connection: T,
    pub local_node: Node,
    pub peer_node: Node,
}

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

        self.send_complement().await?;
        log::debug!("[client] send_complement");

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
                let _version = reader.read_u16().await?;
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

#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub name: NodeName,
    pub flags: DistributionFlags,
    pub creation: Option<Creation>,
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
    #[error("no available version: self={self_lowest}..={self_highest}, peer={peer_lowest}..={peer_highest}")]
    VersionMismatch {
        self_highest: u16,
        self_lowest: u16,
        peer_highest: u16,
        peer_lowest: u16,
    },

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

    #[error(transparent)]
    NodeNameError(#[from] crate::node::NodeNameError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug)]
pub struct Handshake {
    self_node: NodeEntry,
    creation: Creation,
    flags: DistributionFlags,
    cookie: String,
}

impl Handshake {
    pub fn new(
        self_node: NodeEntry,
        creation: Creation,
        flags: DistributionFlags,
        cookie: &str,
    ) -> Self {
        Self {
            self_node,
            creation,
            flags,
            cookie: cookie.to_owned(),
        }
    }

    pub async fn accept<T>(self, socket: T) -> Result<(T, PeerInfo), HandshakeError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        let socket = Socket::new(socket);
        let server = HandshakeServer {
            socket,
            this: self.self_node,
            flags: self.flags,
            creation: self.creation,
            cookie: self.cookie,
            challenge: Challenge::new(),
        };
        server.accept().await
    }
}

#[derive(Debug)]
struct HandshakeServer<T> {
    socket: Socket<T>,
    this: NodeEntry,
    flags: DistributionFlags,
    creation: Creation,
    cookie: String,
    challenge: Challenge,
}

impl<T> HandshakeServer<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    async fn accept(mut self) -> Result<(T, PeerInfo), HandshakeError> {
        let (peer_name, mut flags, mut maybe_creation) = self.recv_name().await?;
        self.send_status().await?;
        self.send_challenge(flags).await?;
        if flags.contains(DistributionFlags::DFLAG_HANDSHAKE_23) && maybe_creation.is_none() {
            let (flags_high, creation) = self.recv_complement().await?;
            maybe_creation = Some(creation);
            flags |= flags_high;
        }
        let (challenge, digest) = self.recv_challenge_reply().await?;
        if self.challenge.digest(&self.cookie) != digest {
            return Err(HandshakeError::InvalidDigest);
        }
        self.send_challenge_ack(challenge).await?;

        let stream = self.socket.into_inner();
        let peer_info = PeerInfo {
            name: peer_name.parse()?,
            flags,
            creation: maybe_creation,
        };
        Ok((stream, peer_info))
    }

    async fn send_challenge_ack(&mut self, challenge: Challenge) -> Result<(), HandshakeError> {
        let mut writer = self.socket.message_writer();
        writer.write_u8(b'a')?;
        writer.write_all(&challenge.digest(&self.cookie).0)?;
        writer.finish().await?;
        Ok(())
    }

    async fn recv_challenge_reply(&mut self) -> Result<(Challenge, Digest), HandshakeError> {
        let mut reader = self.socket.message_reader().await?;
        let tag = reader.read_u8().await?;
        if tag != b'r' {
            return Err(HandshakeError::UnexpectedTag {
                message: "challenge_reply",
                tag,
            });
        }
        let challenge = Challenge(reader.read_u32().await?);
        let mut digest = Digest([0; 16]);
        reader.read_exact(&mut digest.0).await?;
        Ok((challenge, digest))
    }

    async fn recv_complement(&mut self) -> Result<(DistributionFlags, Creation), HandshakeError> {
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
        Ok((flags_high, creation))
    }

    async fn send_challenge(
        &mut self,
        peer_flags: DistributionFlags,
    ) -> Result<(), HandshakeError> {
        let mut writer = self.socket.message_writer();
        if peer_flags.contains(DistributionFlags::DFLAG_HANDSHAKE_23) {
            writer.write_u8(b'N')?;
            writer.write_u64(self.flags.bits())?;
            writer.write_u32(self.challenge.0)?;
            writer.write_u32(self.creation.get())?;
            writer.write_u16(self.this.name.len() as u16)?; // TODO: validate
            writer.write_all(self.this.name.as_bytes())?;
        } else {
            writer.write_u8(b'n')?;
            writer.write_u16(5)?;
            writer.write_u32(self.flags.bits() as u32)?;
            writer.write_u32(self.challenge.0)?;
            writer.write_all(self.this.name.as_bytes())?;
        }
        writer.finish().await?;
        Ok(())
    }

    async fn recv_name(
        &mut self,
    ) -> Result<(String, DistributionFlags, Option<Creation>), HandshakeError> {
        let mut reader = self.socket.message_reader().await?;
        let tag = reader.read_u8().await?;
        match tag {
            b'n' => {
                let version = reader.read_u16().await?;
                if version != 5 {
                    return Err(HandshakeError::InvalidVersionValue { value: version });
                }
                let flags =
                    DistributionFlags::from_bits_truncate(u64::from(reader.read_u32().await?));
                let name = reader.read_string().await?;
                Ok((name, flags, None))
            }
            b'N' => {
                let flags = DistributionFlags::from_bits_truncate(reader.read_u64().await?);
                let creation = Creation::new(reader.read_u32().await?);
                let name = reader.read_u16_string().await?;
                Ok((name, flags, Some(creation)))
            }
            _ => Err(HandshakeError::UnexpectedTag {
                message: "send_name",
                tag,
            }),
        }
    }

    async fn send_status(&mut self) -> Result<(), HandshakeError> {
        // TODO: check flags
        // TODO: check conflict
        let mut writer = self.socket.message_writer();
        writer.write_u8(b's')?;
        writer.write_all(b"ok")?;
        writer.finish().await?;
        Ok(())
    }
}

// TODO: add test
