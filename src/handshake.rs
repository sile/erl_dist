//! Distribution Handshake implementation.
//!
//! This handshake is used by an Erlang node for connecting to another one.
//!
//! See [12.2 Distribution Handshake]
//! (http://erlang.org/doc/apps/erts/erl_dist_protocol.html#id104553)
//! for more details about distribution handshake.
use futures::{self, Future};
use handy_async::io::{ExternalSize, ReadFrom, WriteInto};
use handy_async::pattern::combinators::BE;
use handy_async::pattern::read::{Utf8, U16, U32, U8};
use handy_async::pattern::{Buf, Endian, Pattern};
use md5;
use rand;
use std::io::{Error, ErrorKind, Read, Result, Write};

/// The distribution version this crate can handle.
pub const DISTRIBUTION_VERSION: u16 = 5;

const TAG_NAME: u8 = 'n' as u8;
const TAG_STATUS: u8 = 's' as u8;
const TAG_CHALLENGE: u8 = 'n' as u8;
const TAG_REPLY: u8 = 'r' as u8;
const TAG_ACK: u8 = 'a' as u8;

bitflags! {
    /// Distribution flags.
    pub struct DistributionFlags: u32 {
        /// The node is to be published and part of the global namespace.
        const DFLAG_PUBLISHED = 0x01;

        /// The node implements an atom cache (obsolete).
        const DFLAG_ATOM_CACHE = 0x02;

        /// The node implements extended (3 × 32 bits) references (required).
        ///
        /// This is required today. If not present, the connection is refused.
        const DFLAG_EXTENDED_REFERENCES = 0x04;

        /// The node implements distributed process monitoring.
        const DFLAG_DIST_MONITOR = 0x08;

        /// The node uses separate tag for funs (lambdas) in the distribution protocol.
        const DFLAG_FUN_TAGS = 0x10;

        /// The node implements distributed named process monitoring.
        const DFLAG_DIST_MONITOR_NAME = 0x20;

        /// The (hidden) node implements atom cache (obsolete).
        const DFLAG_HIDDEN_ATOM_CACHE = 0x40;

        /// The node understands new fun tags.
        const DFLAG_NEW_FUN_TAGS = 0x80;

        /// The node can handle extended pids and ports (required).
        ///
        /// This is required today. If not present, the connection is refused.
        const DFLAG_EXTENDED_PIDS_PORTS = 0x100;

        /// This node understands `EXPORT_EXT` tag.
        const DFLAG_EXPORT_PTR_TAG = 0x200;

        /// The node understands bit binaries.
        const DFLAG_BIT_BINARIES = 0x400;

        /// The node understandss new float format.
        const DFLAG_NEW_FLOATS = 0x800;

        /// This node allows unicode characters in I/O operations.
        const DFLAG_UNICODE_IO = 0x1000;

        /// The node implements atom cache in distribution header.
        ///
        /// Note that currently `erl_dist` can not handle distribution headers.
        const DFLAG_DIST_HDR_ATOM_CACHE = 0x2000;

        /// The node understands the `SMALL_ATOM_EXT` tag.
        const DFLAG_SMALL_ATOM_TAGS = 0x4000;

        /// The node understands UTF-8 encoded atoms.
        const DFLAG_UTF8_ATOMS = 0x10000;

        /// The node understands maps.
        const DFLAG_MAP_TAGS = 0x20000;
    }
}
impl Default for DistributionFlags {
    /// The default distribution flags.
    ///
    /// This is equivalent to the following code:
    ///
    /// ```no_run
    /// # use erl_dist::handshake::DistributionFlags;
    /// DistributionFlags::DFLAG_EXTENDED_REFERENCES | DistributionFlags::DFLAG_EXTENDED_PIDS_PORTS |
    /// DistributionFlags::DFLAG_FUN_TAGS | DistributionFlags::DFLAG_NEW_FUN_TAGS |
    /// DistributionFlags::DFLAG_EXPORT_PTR_TAG | DistributionFlags::DFLAG_BIT_BINARIES |
    /// DistributionFlags::DFLAG_NEW_FLOATS | DistributionFlags::DFLAG_SMALL_ATOM_TAGS |
    /// DistributionFlags::DFLAG_UTF8_ATOMS | DistributionFlags::DFLAG_MAP_TAGS
    /// # ;
    /// ```
    fn default() -> Self {
        Self::DFLAG_EXTENDED_REFERENCES
            | Self::DFLAG_EXTENDED_PIDS_PORTS
            | Self::DFLAG_FUN_TAGS
            | Self::DFLAG_NEW_FUN_TAGS
            | Self::DFLAG_EXPORT_PTR_TAG
            | Self::DFLAG_BIT_BINARIES
            | Self::DFLAG_NEW_FLOATS
            | Self::DFLAG_SMALL_ATOM_TAGS
            | Self::DFLAG_UTF8_ATOMS
            | Self::DFLAG_MAP_TAGS
    }
}

/// A structure that represents a connected peer node.
#[derive(Debug)]
pub struct Peer<S> {
    /// The name of this peer node.
    ///
    /// The format of the node name is `"${NAME}@${HOST}"`.
    pub name: String,

    /// The distribution flags of this peer.
    pub flags: DistributionFlags,

    /// The stream used to communicate with this peer.
    pub stream: S,
}
impl<S: Read> Read for Peer<S> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.stream.read(buf)
    }
}
impl<S: Write> Write for Peer<S> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.stream.write(buf)
    }
    fn flush(&mut self) -> Result<()> {
        self.stream.flush()
    }
}

/// Handshake object.
#[derive(Debug, Clone)]
pub struct Handshake {
    self_node_name: String,
    self_cookie: String,
    peer_cookie: String,
    flags: DistributionFlags,
}
impl Handshake {
    /// Makes a new Handshake object.
    ///
    /// The cookies for self and peer nodes are both set to `cookie`.
    ///
    /// The set of distribution flags the self node can recognize is
    /// set to `DistributionFlags::default()`.
    ///
    /// Note the format of `self_node_name` must be `"${NAME}@${NAME}"`.
    pub fn new(self_node_name: &str, cookie: &str) -> Self {
        Handshake {
            self_node_name: self_node_name.to_string(),
            self_cookie: cookie.to_string(),
            peer_cookie: cookie.to_string(),
            flags: DistributionFlags::default(),
        }
    }

    /// Sets the set of distribution flags this node can recognize to `flags`.
    pub fn flags(&mut self, flags: DistributionFlags) -> &mut Self {
        self.flags = flags;
        self
    }

    /// Sets the cookie of the self node to `cookie`.
    pub fn self_cookie(&mut self, cookie: &str) -> &mut Self {
        self.self_cookie = cookie.to_string();
        self
    }

    /// Sets the cookie of the peer node to `cookie`.
    pub fn peer_cookie(&mut self, cookie: &str) -> &mut Self {
        self.peer_cookie = cookie.to_string();
        self
    }

    /// Executes the client side handshake for connecting to the `peer` node.
    ///
    /// # Note
    ///
    /// For executing asynchronously, we assume that `peer` returns
    /// the `std::io::ErrorKind::WouldBlock` error if an I/O operation would be about to block.
    ///
    /// # Examples
    ///
    /// Connects to the "foo" node running on localhost at port `4000`:
    ///
    /// ```no_run
    /// # extern crate erl_dist;
    /// # extern crate futures;
    /// # extern crate fibers;
    /// use fibers::{Executor, InPlaceExecutor, Spawn};
    /// use fibers::net::TcpStream;
    /// use futures::Future;
    /// use erl_dist::Handshake;
    ///
    /// # fn main() {
    /// let peer_addr = "127.0.0.1:58312".parse().unwrap();
    /// let mut executor = InPlaceExecutor::new().unwrap();
    ///
    /// // Executes the client side handshake.
    /// let monitor = executor.spawn_monitor(
    ///     TcpStream::connect(peer_addr).and_then(|socket| {
    ///         let handshake = Handshake::new("bar@localhost", "erlang cookie");
    ///         handshake.connect(socket)
    ///     }));
    /// let peer = executor.run_fiber(monitor).unwrap().unwrap();
    ///
    /// assert_eq!(peer.name, "foo@localhost");
    /// println!("Flags: {:?}", peer.flags);
    /// # }
    /// ```
    ///
    /// See the files in the ["example" directory]
    /// (https://github.com/sile/erl_dist/tree/master/examples) for more examples.
    pub fn connect<S>(&self, peer: S) -> impl 'static + Future<Item = Peer<S>, Error = Error> + Send
    where
        S: Read + Write + Send + 'static,
    {
        let peer = Peer {
            stream: peer,
            name: String::new(),               // Dummy value
            flags: DistributionFlags::empty(), // Dummy value
        };
        let Handshake {
            self_node_name,
            self_cookie,
            peer_cookie,
            flags,
        } = self.clone();
        futures::finished(peer)
            .and_then(move |peer| {
                // send_name
                with_len((
                    TAG_NAME,
                    DISTRIBUTION_VERSION.be(),
                    flags.bits().be(),
                    self_node_name,
                ))
                .write_into(peer)
            })
            .and_then(|(peer, _)| {
                // recv_status
                let status = U16.be().and_then(|len| {
                    let status = Utf8(vec![0; len as usize - 1]).and_then(check_status);
                    (U8.expect_eq(TAG_STATUS), status)
                });
                status.read_from(peer)
            })
            .and_then(|(peer, _)| {
                // recv_challenge
                let challenge = U16.be().and_then(|len| {
                    let name = Utf8(vec![0; len as usize - 11]); // TODO: boundary check
                    (
                        U8.expect_eq(TAG_CHALLENGE),
                        U16.be().expect_eq(DISTRIBUTION_VERSION),
                        U32.be(),
                        U32.be(),
                        name,
                    )
                });
                challenge.read_from(peer)
            })
            .and_then(
                move |(mut peer, (_, _, peer_flags, peer_challenge, peer_name))| {
                    // send_challenge_reply
                    peer.name = peer_name;
                    peer.flags = DistributionFlags::from_bits_truncate(peer_flags);

                    let peer_digest = calc_digest(&peer_cookie, peer_challenge);
                    let self_challenge = rand::random::<u32>();
                    let self_digest = calc_digest(&self_cookie, self_challenge);

                    let reply = with_len((TAG_REPLY, self_challenge.be(), Buf(peer_digest)))
                        .map(move |_| self_digest);
                    reply.write_into(peer)
                },
            )
            .and_then(|(peer, self_digest)| {
                // recv_challenge_ack
                let digest = Buf([0; 16]).expect_eq(self_digest);
                let ack = (U16.be().expect_eq(17), U8.expect_eq(TAG_ACK), digest);
                ack.read_from(peer)
            })
            .map(|(peer, _)| peer)
            .map_err(|e| e.into_error())
    }

    /// Executes the server side handshake to accept the node connected by the `peer` stream.
    ///
    /// # Note
    ///
    /// For executing asynchronously, we assume that `peer` returns
    /// the `std::io::ErrorKind::WouldBlock` error if an I/O operation would be about to block.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # extern crate erl_dist;
    /// # extern crate futures;
    /// # extern crate fibers;
    /// use fibers::net::TcpListener;
    /// use fibers::{Executor, InPlaceExecutor, Spawn};
    /// use futures::{Future, Stream};
    /// use erl_dist::Handshake;
    ///
    /// # fn main() {
    /// let mut executor = InPlaceExecutor::new().unwrap();
    /// let handle = executor.handle();
    /// let monitor =
    ///     executor.spawn_monitor(TcpListener::bind("127.0.0.1:4000".parse().unwrap())
    ///         .and_then(|listener| {
    ///             listener.incoming().for_each(move |(peer, _)| {
    ///                 handle.spawn(peer.and_then(|peer| {
    ///                         let handshake = Handshake::new("foo@localhost", "erlang cookie");
    ///                         handshake.accept(peer).and_then(|peer| {
    ///                             println!("CONNECTED: {}", peer.name);
    ///                             Ok(())
    ///                         })
    ///                     })
    ///                     .then(|_| Ok(())));
    ///                 Ok(())
    ///             })
    ///         }));
    /// let _ = executor.run_fiber(monitor).unwrap().unwrap();
    /// # }
    /// ```
    ///
    /// See the [recv_msg.rs]
    /// (https://github.com/sile/erl_dist/tree/master/examples/recv_msg.rs)
    /// file for a running example.
    pub fn accept<S>(&self, peer: S) -> impl 'static + Future<Item = Peer<S>, Error = Error> + Send
    where
        S: Read + Write + Send + 'static,
    {
        let peer = Peer {
            stream: peer,
            name: String::new(),               // Dummy value
            flags: DistributionFlags::empty(), // Dummy value
        };
        let Handshake {
            self_node_name,
            self_cookie,
            peer_cookie,
            flags,
        } = self.clone();
        futures::finished(peer)
            .and_then(|peer| {
                // recv_name
                let recv_name = U16.be().and_then(|len| {
                    let peer_name = Utf8(vec![0; len as usize - 7]);
                    (
                        U8.expect_eq(TAG_NAME),
                        U16.be().expect_eq(DISTRIBUTION_VERSION),
                        U32.be(),
                        peer_name,
                    )
                });
                recv_name.read_from(peer)
            })
            .and_then(move |(mut peer, (_, _, peer_flags, peer_name))| {
                // send_status
                let peer_flags = DistributionFlags::from_bits_truncate(peer_flags);
                let acceptable_flags = flags & peer_flags;
                peer.name = peer_name;
                peer.flags = acceptable_flags;

                // TODO: Handle conflictions
                let status = (TAG_STATUS, "ok".to_string());
                with_len(status).write_into(peer)
            })
            .and_then(move |(peer, _)| {
                // send_challenge
                let self_challenge = rand::random::<u32>();
                let self_digest = calc_digest(&self_cookie, self_challenge);
                let challenge = (
                    TAG_CHALLENGE,
                    DISTRIBUTION_VERSION.be(),
                    peer.flags.bits().be(),
                    self_challenge.be(),
                    self_node_name,
                );
                with_len(challenge)
                    .map(move |_| self_digest)
                    .write_into(peer)
            })
            .and_then(|(peer, self_digest)| {
                // recv_challenge_reply
                let reply = (
                    U16.be().expect_eq(21),
                    U8.expect_eq(TAG_REPLY),
                    U32.be(),
                    Buf([0; 16]).expect_eq(self_digest),
                );
                reply.read_from(peer)
            })
            .and_then(move |(peer, (_, _, peer_challenge, _))| {
                // send_challenge_ack
                let peer_digest = calc_digest(&peer_cookie, peer_challenge);
                let ack = (TAG_ACK, Buf(peer_digest));
                with_len(ack).write_into(peer)
            })
            .map(|(peer, _)| peer)
            .map_err(|e| e.into_error())
    }
}

fn check_status(status: String) -> Result<()> {
    match status.as_str() {
        "ok" | "ok_simultaneous" => Ok(()),
        "nok" | "now_allowed" | "alive" => {
            let e = Error::new(
                ErrorKind::ConnectionRefused,
                format!("Handshake request is refused by the reason {:?}", status),
            );
            Err(e)
        }
        _ => {
            let e = Error::new(ErrorKind::Other, format!("Unknown status: {:?}", status));
            Err(e)
        }
    }
}

fn with_len<P: ExternalSize>(pattern: P) -> (BE<u16>, P) {
    ((pattern.external_size() as u16).be(), pattern)
}

fn calc_digest(cookie: &str, challenge: u32) -> [u8; 16] {
    md5::compute(&format!("{}{}", cookie, challenge)).0
}
