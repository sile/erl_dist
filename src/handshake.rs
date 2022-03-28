//! Distribution Handshake.
//!
//! This handshake is used by an Erlang node for connecting to another one.
//!
//! See
//! [Distribution Handshake (Erlang Official Doc)](https://www.erlang.org/doc/apps/erts/erl_dist_protocol.html#distribution-handshake)
//! for more details.
use crate::epmd::{HandshakeProtocolVersion, NodeInfo};
use crate::node::NodeName;
use crate::socket::Socket;
use crate::Creation;
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

#[derive(Debug, Clone)]
struct Digest([u8; 16]);

#[derive(Debug, thiserror::Error)]
pub enum HandshakeError {
    #[error("no available version: self={self_lowest}..={self_highest}, peer={peer_lowest}..={peer_highest}")]
    VersionMismatch {
        self_highest: HandshakeProtocolVersion,
        self_lowest: HandshakeProtocolVersion,
        peer_highest: HandshakeProtocolVersion,
        peer_lowest: HandshakeProtocolVersion,
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
            peer: peer_node,
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
        };
        server.accept().await
    }

    fn check_available_highest_version(
        &self,
        peer_node: &NodeInfo,
    ) -> Result<HandshakeProtocolVersion, HandshakeError> {
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
}

impl<T> HandshakeServer<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    async fn accept(mut self) -> Result<(T, PeerInfo), HandshakeError> {
        todo!()
    }
}

#[derive(Debug)]
struct HandshakeClient<T> {
    socket: Socket<T>,
    version: HandshakeProtocolVersion,
    this: NodeInfo,
    peer: NodeInfo,
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
        if self.version == HandshakeProtocolVersion::V5 && peer_creation.is_some() {
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
                let flags =
                    DistributionFlags::from_bits_truncate(u64::from(reader.read_u64().await?)); // TODO
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
            HandshakeProtocolVersion::V5 => {
                writer.write_u8(b'n')?;
                writer.write_u16(self.version as u16)?;
                writer.write_u32(self.flags.bits() as u32)?;
                writer.write_all(self.this.name.as_bytes())?;
            }
            HandshakeProtocolVersion::V6 => {
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

// use futures::{self, Future};
// use handy_async::io::{ExternalSize, ReadFrom, WriteInto};
// use handy_async::pattern::combinators::BE;
// use handy_async::pattern::read::{Utf8, U16, U32, U8};
// use handy_async::pattern::{Buf, Endian, Pattern};
// use md5;
// use rand;
// use std::io::{Error, ErrorKind, Read, Result, Write};

// /// The distribution version this crate can handle.
// pub const DISTRIBUTION_VERSION: u16 = 5;

// const TAG_NAME: u8 = b'n';
// const TAG_STATUS: u8 = b's';
// const TAG_CHALLENGE: u8 = b'n';
// const TAG_REPLY: u8 = b'r';
// const TAG_ACK: u8 = b'a';

// /// A structure that represents a connected peer node.
// #[derive(Debug)]
// pub struct Peer<S> {
//     /// The name of this peer node.
//     ///
//     /// The format of the node name is `"${NAME}@${HOST}"`.
//     pub name: String,

//     /// The distribution flags of this peer.
//     pub flags: DistributionFlags,

//     /// The stream used to communicate with this peer.
//     pub stream: S,
// }
// impl<S: Read> Read for Peer<S> {
//     fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
//         self.stream.read(buf)
//     }
// }
// impl<S: Write> Write for Peer<S> {
//     fn write(&mut self, buf: &[u8]) -> Result<usize> {
//         self.stream.write(buf)
//     }
//     fn flush(&mut self) -> Result<()> {
//         self.stream.flush()
//     }
// }

// /// Handshake object.
// #[derive(Debug, Clone)]
// pub struct Handshake {
//     self_node_name: String,
//     self_cookie: String,
//     peer_cookie: String,
//     flags: DistributionFlags,
// }
// impl Handshake {
//     /// Makes a new Handshake object.
//     ///
//     /// The cookies for self and peer nodes are both set to `cookie`.
//     ///
//     /// The set of distribution flags the self node can recognize is
//     /// set to `DistributionFlags::default()`.
//     ///
//     /// Note the format of `self_node_name` must be `"${NAME}@${NAME}"`.
//     pub fn new(self_node_name: &str, cookie: &str) -> Self {
//         Handshake {
//             self_node_name: self_node_name.to_string(),
//             self_cookie: cookie.to_string(),
//             peer_cookie: cookie.to_string(),
//             flags: DistributionFlags::default(),
//         }
//     }

//     /// Sets the set of distribution flags this node can recognize to `flags`.
//     pub fn flags(&mut self, flags: DistributionFlags) -> &mut Self {
//         self.flags = flags;
//         self
//     }

//     /// Sets the cookie of the self node to `cookie`.
//     pub fn self_cookie(&mut self, cookie: &str) -> &mut Self {
//         self.self_cookie = cookie.to_string();
//         self
//     }

//     /// Sets the cookie of the peer node to `cookie`.
//     pub fn peer_cookie(&mut self, cookie: &str) -> &mut Self {
//         self.peer_cookie = cookie.to_string();
//         self
//     }

//     /// Executes the client side handshake for connecting to the `peer` node.
//     ///
//     /// # Note
//     ///
//     /// For executing asynchronously, we assume that `peer` returns
//     /// the `std::io::ErrorKind::WouldBlock` error if an I/O operation would be about to block.
//     ///
//     /// # Examples
//     ///
//     /// Connects to the "foo" node running on localhost at port `4000`:
//     ///
//     /// ```no_run
//     /// use fibers::{Executor, InPlaceExecutor, Spawn};
//     /// use fibers::net::TcpStream;
//     /// use futures::Future;
//     /// use erl_dist::Handshake;
//     ///
//     /// # fn main() {
//     /// let peer_addr = "127.0.0.1:58312".parse().unwrap();
//     /// let mut executor = InPlaceExecutor::new().unwrap();
//     ///
//     /// // Executes the client side handshake.
//     /// let monitor = executor.spawn_monitor(
//     ///     TcpStream::connect(peer_addr).and_then(|socket| {
//     ///         let handshake = Handshake::new("bar@localhost", "erlang cookie");
//     ///         handshake.connect(socket)
//     ///     }));
//     /// let peer = executor.run_fiber(monitor).unwrap().unwrap();
//     ///
//     /// assert_eq!(peer.name, "foo@localhost");
//     /// println!("Flags: {:?}", peer.flags);
//     /// # }
//     /// ```
//     ///
//     /// See the files in the ["example" directory]
//     /// (https://github.com/sile/erl_dist/tree/master/examples) for more examples.
//     pub fn connect<S>(&self, peer: S) -> impl 'static + Future<Item = Peer<S>, Error = Error> + Send
//     where
//         S: Read + Write + Send + 'static,
//     {
//         let peer = Peer {
//             stream: peer,
//             name: String::new(),               // Dummy value
//             flags: DistributionFlags::empty(), // Dummy value
//         };
//         let Handshake {
//             self_node_name,
//             self_cookie,
//             peer_cookie,
//             flags,
//         } = self.clone();
//         futures::finished(peer)
//             .and_then(move |peer| {
//                 // send_name
//                 with_len((
//                     TAG_NAME,
//                     DISTRIBUTION_VERSION.be(),
//                     flags.bits().be(),
//                     self_node_name,
//                 ))
//                 .write_into(peer)
//             })
//             .and_then(|(peer, _)| {
//                 // recv_status
//                 let status = U16.be().and_then(|len| {
//                     let status = Utf8(vec![0; len as usize - 1]).and_then(check_status);
//                     (U8.expect_eq(TAG_STATUS), status)
//                 });
//                 status.read_from(peer)
//             })
//             .and_then(|(peer, _)| {
//                 // recv_challenge
//                 let challenge = U16.be().and_then(|len| {
//                     let name = Utf8(vec![0; len as usize - 11]); // TODO: boundary check
//                     (
//                         U8.expect_eq(TAG_CHALLENGE),
//                         U16.be().expect_eq(DISTRIBUTION_VERSION),
//                         U32.be(),
//                         U32.be(),
//                         name,
//                     )
//                 });
//                 challenge.read_from(peer)
//             })
//             .and_then(
//                 move |(mut peer, (_, _, peer_flags, peer_challenge, peer_name))| {
//                     // send_challenge_reply
//                     peer.name = peer_name;
//                     peer.flags = DistributionFlags::from_bits_truncate(peer_flags);

//                     let peer_digest = calc_digest(&peer_cookie, peer_challenge);
//                     let self_challenge = rand::random::<u32>();
//                     let self_digest = calc_digest(&self_cookie, self_challenge);

//                     let reply = with_len((TAG_REPLY, self_challenge.be(), Buf(peer_digest)))
//                         .map(move |_| self_digest);
//                     reply.write_into(peer)
//                 },
//             )
//             .and_then(|(peer, self_digest)| {
//                 // recv_challenge_ack
//                 let digest = Buf([0; 16]).expect_eq(self_digest);
//                 let ack = (U16.be().expect_eq(17), U8.expect_eq(TAG_ACK), digest);
//                 ack.read_from(peer)
//             })
//             .map(|(peer, _)| peer)
//             .map_err(|e| e.into_error())
//     }

//     /// Executes the server side handshake to accept the node connected by the `peer` stream.
//     ///
//     /// # Note
//     ///
//     /// For executing asynchronously, we assume that `peer` returns
//     /// the `std::io::ErrorKind::WouldBlock` error if an I/O operation would be about to block.
//     ///
//     /// # Examples
//     ///
//     /// ```no_run
//     /// use fibers::net::TcpListener;
//     /// use fibers::{Executor, InPlaceExecutor, Spawn};
//     /// use futures::{Future, Stream};
//     /// use erl_dist::Handshake;
//     ///
//     /// # fn main() {
//     /// let mut executor = InPlaceExecutor::new().unwrap();
//     /// let handle = executor.handle();
//     /// let monitor =
//     ///     executor.spawn_monitor(TcpListener::bind("127.0.0.1:4000".parse().unwrap())
//     ///         .and_then(|listener| {
//     ///             listener.incoming().for_each(move |(peer, _)| {
//     ///                 handle.spawn(peer.and_then(|peer| {
//     ///                         let handshake = Handshake::new("foo@localhost", "erlang cookie");
//     ///                         handshake.accept(peer).and_then(|peer| {
//     ///                             println!("CONNECTED: {}", peer.name);
//     ///                             Ok(())
//     ///                         })
//     ///                     })
//     ///                     .then(|_| Ok(())));
//     ///                 Ok(())
//     ///             })
//     ///         }));
//     /// let _ = executor.run_fiber(monitor).unwrap().unwrap();
//     /// # }
//     /// ```
//     ///
//     /// See the [recv_msg.rs]
//     /// (https://github.com/sile/erl_dist/tree/master/examples/recv_msg.rs)
//     /// file for a running example.
//     pub fn accept<S>(&self, peer: S) -> impl 'static + Future<Item = Peer<S>, Error = Error> + Send
//     where
//         S: Read + Write + Send + 'static,
//     {
//         let peer = Peer {
//             stream: peer,
//             name: String::new(),               // Dummy value
//             flags: DistributionFlags::empty(), // Dummy value
//         };
//         let Handshake {
//             self_node_name,
//             self_cookie,
//             peer_cookie,
//             flags,
//         } = self.clone();
//         futures::finished(peer)
//             .and_then(|peer| {
//                 // recv_name
//                 let recv_name = U16.be().and_then(|len| {
//                     let peer_name = Utf8(vec![0; len as usize - 7]);
//                     (
//                         U8.expect_eq(TAG_NAME),
//                         U16.be().expect_eq(DISTRIBUTION_VERSION),
//                         U32.be(),
//                         peer_name,
//                     )
//                 });
//                 recv_name.read_from(peer)
//             })
//             .and_then(move |(mut peer, (_, _, peer_flags, peer_name))| {
//                 // send_status
//                 let peer_flags = DistributionFlags::from_bits_truncate(peer_flags);
//                 let acceptable_flags = flags & peer_flags;
//                 peer.name = peer_name;
//                 peer.flags = acceptable_flags;

//                 // TODO: Handle conflictions
//                 let status = (TAG_STATUS, "ok".to_string());
//                 with_len(status).write_into(peer)
//             })
//             .and_then(move |(peer, _)| {
//                 // send_challenge
//                 let self_challenge = rand::random::<u32>();
//                 let self_digest = calc_digest(&self_cookie, self_challenge);
//                 let challenge = (
//                     TAG_CHALLENGE,
//                     DISTRIBUTION_VERSION.be(),
//                     peer.flags.bits().be(),
//                     self_challenge.be(),
//                     self_node_name,
//                 );
//                 with_len(challenge)
//                     .map(move |_| self_digest)
//                     .write_into(peer)
//             })
//             .and_then(|(peer, self_digest)| {
//                 // recv_challenge_reply
//                 let reply = (
//                     U16.be().expect_eq(21),
//                     U8.expect_eq(TAG_REPLY),
//                     U32.be(),
//                     Buf([0; 16]).expect_eq(self_digest),
//                 );
//                 reply.read_from(peer)
//             })
//             .and_then(move |(peer, (_, _, peer_challenge, _))| {
//                 // send_challenge_ack
//                 let peer_digest = calc_digest(&peer_cookie, peer_challenge);
//                 let ack = (TAG_ACK, Buf(peer_digest));
//                 with_len(ack).write_into(peer)
//             })
//             .map(|(peer, _)| peer)
//             .map_err(|e| e.into_error())
//     }
// }

// fn check_status(status: String) -> Result<()> {
//     match status.as_str() {
//         "ok" | "ok_simultaneous" => Ok(()),
//         "nok" | "now_allowed" | "alive" => {
//             let e = Error::new(
//                 ErrorKind::ConnectionRefused,
//                 format!("Handshake request is refused by the reason {:?}", status),
//             );
//             Err(e)
//         }
//         _ => {
//             let e = Error::new(ErrorKind::Other, format!("Unknown status: {:?}", status));
//             Err(e)
//         }
//     }
// }
