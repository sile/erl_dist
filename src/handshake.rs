//! Distribution Handshake.
//!
//! This handshake is used by an Erlang node for connecting to another one.
//!
//! See
//! [Distribution Handshake (Erlang Official Doc)](https://www.erlang.org/doc/apps/erts/erl_dist_protocol.html#distribution-handshake)
//! for more details.
use crate::node::{Creation, LocalNode, NodeName, PeerNode};
use crate::socket::Socket;
use crate::DistributionFlags;
use byteorder::{BigEndian, ReadBytesExt};
use futures::io::{AsyncRead, AsyncWrite};

/// Client-side handshake.
#[derive(Debug)]
pub struct ClientSideHandshake<T> {
    local_node: LocalNode,
    local_challenge: Challenge,
    cookie: String,
    socket: Socket<T>,
    send_name_status: Option<HandshakeStatus>,
}

impl<T> ClientSideHandshake<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    /// Makes a new [`ClientSideHandshake`] instance.
    pub fn new(connection: T, local_node: LocalNode, cookie: &str) -> Self {
        Self {
            local_node,
            local_challenge: Challenge::new(),
            cookie: cookie.to_owned(),
            socket: Socket::new(connection),
            send_name_status: None,
        }
    }

    /// Executes the first part of the handshake protocol.
    ///
    /// To complete the handshake, you then need to call [`ClientSideHandshake::execute_rest()`] method
    /// taking into account the [`HandshakeStatus`] replied from the peer node.
    pub async fn execute_send_name(&mut self) -> Result<HandshakeStatus, HandshakeError> {
        self.send_name().await?;
        let status = self.recv_status().await?;
        self.send_name_status = Some(status.clone());
        Ok(status)
    }

    /// Executes the rest part of the handshake protocol.
    ///
    /// You must need to have called [`ClientSideHandshake::execute_send_name()`] before this method.
    /// In a case where [`HandshakeStatus`] of the first part is [`HandshakeStatus::Alive`],
    /// the `do_continue` argument is used to indicate whether this handshake should continue or not
    /// (otherwise the argument is ignored).
    ///
    /// If the [`HandshakeStatus`] returned is a non-ok status, this method call fails immediately.
    pub async fn execute_rest(
        mut self,
        do_continue: bool,
    ) -> Result<(T, PeerNode), HandshakeError> {
        match self.send_name_status {
            None => {
                return Err(HandshakeError::PhaseError {
                    current: "ClientSideHandshake::execute_rest()",
                    depends_on: "ClientSideHandshake::execute_send_name()",
                })
            }
            Some(HandshakeStatus::Nok) => return Err(HandshakeError::OngoingHandshake),
            Some(HandshakeStatus::NotAllowed) => return Err(HandshakeError::NotAllowed),
            Some(HandshakeStatus::Alive) => {
                self.send_status(if do_continue { "true" } else { "false" })
                    .await?;
                if !do_continue {
                    return Err(HandshakeError::AlreadyActive);
                }
            }
            _ => {}
        }

        let (peer_node, peer_challenge) = self.recv_challenge().await?;
        if peer_node.creation.is_some() {
            self.send_complement().await?;
        }
        self.send_challenge_reply(peer_challenge).await?;
        self.recv_challenge_ack().await?;

        let connection = self.socket.into_inner();
        Ok((connection, peer_node))
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

    async fn recv_status(&mut self) -> Result<HandshakeStatus, HandshakeError> {
        let mut reader = self.socket.message_reader().await?;
        let tag = reader.read_u8().await?;
        if tag != b's' {
            return Err(HandshakeError::UnexpectedTag {
                message: "STATUS",
                tag,
            });
        }
        let status = reader.read_bytes().await?;
        let status = match status.as_slice() {
            b"ok" => HandshakeStatus::Ok,
            b"ok_simultaneous" => HandshakeStatus::OkSimultaneous,
            b"nok" => HandshakeStatus::Nok,
            b"not_allowed" => HandshakeStatus::NotAllowed,
            b"alive" => HandshakeStatus::Alive,
            _ => {
                if status.starts_with(b"named:") {
                    use std::io::Read as _;

                    let mut bytes = &status["named:".len()..];
                    let n = u64::from(bytes.read_u16::<BigEndian>()?);
                    let mut name = String::new();
                    bytes.take(n).read_to_string(&mut name)?;
                    HandshakeStatus::Named { name }
                } else {
                    let status = String::from_utf8_lossy(&status).to_string();
                    return Err(HandshakeError::UnknownStatus { status });
                }
            }
        };
        reader.finish().await?;
        Ok(status)
    }

    async fn send_status(&mut self, status: &str) -> Result<(), HandshakeError> {
        let mut writer = self.socket.message_writer();
        writer.write_u8(b's')?;
        writer.write_all(status.as_bytes())?;
        Ok(())
    }

    async fn recv_challenge(&mut self) -> Result<(PeerNode, Challenge), HandshakeError> {
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
                let node = PeerNode {
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
                let node = PeerNode {
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
        writer.write_u32(self.local_node.creation.get())?;
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
            return Err(HandshakeError::CookieMismatch);
        }
        reader.finish().await?;

        Ok(())
    }
}

/// Server-side handshake.
#[derive(Debug)]
pub struct ServerSideHandshake<T> {
    local_node: LocalNode,
    local_challenge: Challenge,
    cookie: String,
    socket: Socket<T>,
    peer_node: Option<PeerNode>,
}

impl<T> ServerSideHandshake<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    /// Makes a new [`ServerSideHandshake`] instance.
    pub fn new(connection: T, local_node: LocalNode, cookie: &str) -> Self {
        Self {
            local_node,
            local_challenge: Challenge::new(),
            cookie: cookie.to_owned(),
            socket: Socket::new(connection),
            peer_node: None,
        }
    }

    /// Executes the first part of the handshake protocol.
    ///
    /// To complete the handshake, you then need to call [`ServerSideHandshake::execute_rest()`] method
    /// taking into account the [`NodeName`] sent from the peer node.
    ///
    /// Note that the second value of the result tuple indicates whether
    /// the peer requested a dynamic node name. If the value is `true` and
    /// you want to continue the handshake, you need to use [`HandshakeStatus::Named`] for the reply.
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
                PeerNode {
                    name,
                    flags,
                    creation: None,
                }
            }
            b'N' => {
                let flags = DistributionFlags::from_bits_truncate(reader.read_u64().await?);
                let creation = Creation::new(reader.read_u32().await?);
                let name = reader.read_u16_string().await?.parse()?;
                PeerNode {
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
        let is_dynamic = node.flags.contains(DistributionFlags::NAME_ME);
        self.peer_node = Some(node);
        Ok((name, is_dynamic))
    }

    /// Executes the rest part of the handshake protocol.
    ///
    /// You must need to have called [`ServerSideHandshake::execute_recv_name()`] before this method.
    ///
    /// Note that if the [`HandshakeStatus`] is a non-ok status, this method call fails just
    /// after sending the status to the peer node.
    pub async fn execute_rest(
        mut self,
        status: HandshakeStatus,
    ) -> Result<(T, PeerNode), HandshakeError> {
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

        if peer_flags.contains(DistributionFlags::HANDSHAKE_23) && peer_creation.is_none() {
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
            HandshakeStatus::Named { name } => {
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
        if peer_flags.contains(DistributionFlags::HANDSHAKE_23) {
            writer.write_u8(b'N')?;
            writer.write_u64(self.local_node.flags.bits())?;
            writer.write_u32(self.local_challenge.0)?;
            writer.write_u32(self.local_node.creation.get())?;
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
            return Err(HandshakeError::CookieMismatch);
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

/// Handshake status.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HandshakeStatus {
    /// The handshake will continue.
    Ok,

    /// The handshake will continue.
    ///
    /// The client-side node is informed that the server-side node has another ongoing connection attempt
    /// that will be shut down (simultaneous connect where the client-side node's name is
    /// greater than the server-side node's name, compared literally).
    OkSimultaneous,

    /// The handshake will not continue.
    ///
    /// The client-side already has an ongoing handshake, which it itself has initiated
    /// (simultaneous connect where the server-side node's name is greater than the client-side node's).
    Nok,

    /// The connection is disallowed for some (unspecified) security reason.
    NotAllowed,

    /// A connection to the node is already active.
    ///
    /// This either means that node the client-side node is confused or
    /// that the TCP connection breakdown of a previous node with this name has not yet reached the server-side node.
    ///
    /// The client-side node then decides whether to continue or not the handshake process.
    Alive,

    /// The handshake willl continue, but the client-side node requested a dynamic node name.
    Named {
        /// Dynamic node name that the server-side node generated.
        name: String,
    },
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

/// Possible errors during handshake.
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
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

    #[error("cookie mismatch")]
    CookieMismatch,

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

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[test]
    fn client_side_handshake_works() {
        let peer_name = "client_side_handshake_works";
        smol::block_on(async {
            let erl_node = crate::tests::TestErlangNode::new(peer_name)
                .await
                .expect("failed to run a test erlang node");

            let peer_entry = crate::tests::epmd_client()
                .await
                .get_node(peer_name)
                .await
                .expect("failed to get node");
            let peer_entry = peer_entry.expect("no such node");

            let connection = smol::net::TcpStream::connect(("localhost", peer_entry.port))
                .await
                .expect("failed to connect");
            let local_node = LocalNode::new("foo@localhost".parse().unwrap(), Creation::random());
            let mut handshake =
                ClientSideHandshake::new(connection, local_node, crate::tests::COOKIE);
            let status = handshake
                .execute_send_name()
                .await
                .expect("failed to execute send name");
            assert_eq!(status, HandshakeStatus::Ok);
            let (_, peer_node) = handshake
                .execute_rest(true)
                .await
                .expect("failed to execute handshake");
            assert_eq!(peer_entry.name, peer_node.name.name());

            std::mem::drop(erl_node);
        });
    }

    #[test]
    fn server_side_handshake_works() {
        smol::block_on(async {
            let listener = smol::net::TcpListener::bind("0.0.0.0:0").await.unwrap();
            let listening_port = listener.local_addr().unwrap().port();
            let (tx, rx) = futures::channel::oneshot::channel();
            let connection = smol::net::TcpStream::connect(("localhost", listening_port))
                .await
                .unwrap();

            smol::spawn(async move {
                let local_node =
                    LocalNode::new("foo@localhost".parse().unwrap(), Creation::random());
                let mut handshake =
                    ClientSideHandshake::new(connection, local_node, crate::tests::COOKIE);
                let _status = handshake.execute_send_name().await.unwrap();
                let (connection, _) = handshake.execute_rest(true).await.unwrap();
                let _ = tx.send(connection);
            })
            .detach();

            let mut incoming = listener.incoming();
            if let Some(connection) = incoming.next().await {
                let local_node =
                    LocalNode::new("bar@localhost".parse().unwrap(), Creation::random());
                let mut handshake =
                    ServerSideHandshake::new(connection.unwrap(), local_node, crate::tests::COOKIE);
                let (peer_name, is_dynamic) = handshake.execute_recv_name().await.unwrap();
                assert_eq!(peer_name.name(), "foo");
                assert!(!is_dynamic);

                handshake.execute_rest(HandshakeStatus::Ok).await.unwrap();
            }
            let _ = rx.await;
        })
    }
}
