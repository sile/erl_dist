//! Distribution Handshake.
//!
//! This handshake is used by an Erlang node for connecting to another one.
//!
//! See
//! [Distribution Handshake (Erlang Official Doc)](https://www.erlang.org/doc/apps/erts/erl_dist_protocol.html#distribution-handshake)
//! for more details.
use crate::epmd::NodeInfo;
use crate::node::NodeName;
use crate::socket::Socket;
use crate::Creation;
use crate::DistributionProtocolVersion;
use futures::io::{AsyncRead, AsyncWrite};

pub use self::flags::DistributionFlags;

mod flags;

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
        self_highest: DistributionProtocolVersion,
        self_lowest: DistributionProtocolVersion,
        peer_highest: DistributionProtocolVersion,
        peer_lowest: DistributionProtocolVersion,
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
    self_node: NodeInfo,
    creation: Creation,
    flags: DistributionFlags,
    cookie: String,
}

impl Handshake {
    pub fn new(
        self_node: NodeInfo,
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

    pub async fn connect<T>(
        self,
        peer_node: NodeInfo,
        socket: T,
    ) -> Result<(T, PeerInfo), HandshakeError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        let socket = Socket::new(socket);
        let version = self.check_available_highest_version(&peer_node)?;
        let client = HandshakeClient {
            socket,
            version,
            _peer: peer_node,
            this: self.self_node,
            flags: self.flags,
            creation: self.creation,
            cookie: self.cookie,
        };
        client.connect().await
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

    fn check_available_highest_version(
        &self,
        peer_node: &NodeInfo,
    ) -> Result<DistributionProtocolVersion, HandshakeError> {
        let self_version_range = self.self_node.lowest_version..=self.self_node.highest_version;
        let peer_version_range = peer_node.lowest_version..=peer_node.highest_version;
        if self_version_range.contains(&peer_node.highest_version) {
            Ok(peer_node.highest_version)
        } else if peer_version_range.contains(&self.self_node.highest_version) {
            Ok(self.self_node.highest_version)
        } else {
            Err(HandshakeError::VersionMismatch {
                self_highest: self.self_node.highest_version,
                self_lowest: self.self_node.lowest_version,
                peer_highest: peer_node.highest_version,
                peer_lowest: peer_node.lowest_version,
            })
        }
    }
}

#[derive(Debug)]
struct HandshakeServer<T> {
    socket: Socket<T>,
    this: NodeInfo,
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

#[derive(Debug)]
struct HandshakeClient<T> {
    socket: Socket<T>,
    version: DistributionProtocolVersion,
    this: NodeInfo,
    _peer: NodeInfo,
    flags: DistributionFlags,
    creation: Creation,
    cookie: String,
}

impl<T> HandshakeClient<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    async fn connect(mut self) -> Result<(T, PeerInfo), HandshakeError> {
        self.send_name().await?;

        let status = self.recv_status().await?;
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
            "named" => {
                todo!();
            }
            _ => {
                return Err(HandshakeError::UnknownStatus { status });
            }
        }

        let (peer_name, peer_flags, peer_challenge, peer_creation) = self.recv_challenge().await?;
        if self.version == DistributionProtocolVersion::V5 && peer_creation.is_some() {
            self.send_complement().await?;
        }

        let self_challenge = Challenge::new();
        self.send_challenge_reply(self_challenge, peer_challenge)
            .await?;

        self.recv_challenge_ack(self_challenge).await?;

        let peer_info = PeerInfo {
            name: peer_name,
            flags: peer_flags,
            creation: peer_creation,
        };
        Ok((self.socket.into_inner(), peer_info))
    }

    async fn recv_challenge_ack(
        &mut self,
        self_challenge: Challenge,
    ) -> Result<(), HandshakeError> {
        let mut reader = self.socket.message_reader().await?;
        let tag = reader.read_u8().await?;
        if tag != b'a' {
            return Err(HandshakeError::UnexpectedTag {
                message: "recv_challenge_ack",
                tag,
            });
        }

        let mut digest = [0; 16];
        reader.read_exact(&mut digest).await?;
        if digest != self_challenge.digest(&self.cookie).0 {
            return Err(HandshakeError::InvalidDigest);
        }

        Ok(())
    }

    async fn send_challenge_reply(
        &mut self,
        self_challenge: Challenge,
        peer_challenge: Challenge,
    ) -> Result<(), HandshakeError> {
        let mut writer = self.socket.message_writer();
        writer.write_u8(b'r')?;
        writer.write_u32(self_challenge.0)?;
        writer.write_all(&peer_challenge.digest(&self.cookie).0)?;
        writer.finish().await?;
        Ok(())
    }

    async fn send_complement(&mut self) -> Result<(), HandshakeError> {
        let mut writer = self.socket.message_writer();
        writer.write_u8(b'c')?;
        writer.write_u32((self.flags.bits() >> 32) as u32)?;
        writer.write_u32(self.creation.get())?;
        writer.finish().await?;
        Ok(())
    }

    async fn recv_challenge(
        &mut self,
    ) -> Result<(NodeName, DistributionFlags, Challenge, Option<Creation>), HandshakeError> {
        // TODO: version and flag check
        let mut reader = self.socket.message_reader().await?;
        match reader.read_u8().await? {
            b'n' => {
                let version = reader.read_u16().await?;
                assert_eq!(version, 5); // TODO
                let flags =
                    DistributionFlags::from_bits_truncate(u64::from(reader.read_u32().await?)); // TODO
                let challenge = Challenge(reader.read_u32().await?);
                let name = reader.read_string().await?.parse()?;
                Ok((name, flags, challenge, None))
            }
            b'N' => {
                let flags = DistributionFlags::from_bits_truncate(reader.read_u64().await?); // TODO
                let challenge = Challenge(reader.read_u32().await?);
                let creation = Creation::new(reader.read_u32().await?);
                let name = reader.read_u16_string().await?.parse()?;
                reader.consume_remaining_bytes().await?;
                Ok((name, flags, challenge, Some(creation)))
            }
            tag => Err(HandshakeError::UnexpectedTag {
                message: "recv_challenge",
                tag,
            }),
        }
    }

    async fn send_name(&mut self) -> Result<(), HandshakeError> {
        let mut writer = self.socket.message_writer();
        match self.version {
            DistributionProtocolVersion::V5 => {
                writer.write_u8(b'n')?;
                writer.write_u16(self.version as u16)?;
                writer.write_u32(self.flags.bits() as u32)?;
                writer.write_all(self.this.name.as_bytes())?;
            }
            DistributionProtocolVersion::V6 => {
                writer.write_u8(b'N')?;
                writer.write_u64(self.flags.bits())?;
                writer.write_u32(self.creation.get())?;
                writer.write_u16(self.this.name.len() as u16)?; // TODO: validation
                writer.write_all(self.this.name.as_bytes())?;
            }
        }
        writer.finish().await?;
        Ok(())
    }

    async fn send_status(&mut self, status: &str) -> Result<(), HandshakeError> {
        let mut writer = self.socket.message_writer();
        writer.write_u8(b's')?;
        writer.write_all(status.as_bytes())?;
        Ok(())
    }

    async fn recv_status(&mut self) -> Result<String, HandshakeError> {
        let mut reader = self.socket.message_reader().await?;
        let tag = reader.read_u8().await?;
        if tag != b's' {
            todo!();
        }
        let status = reader.read_string().await?;
        Ok(status)
    }
}
