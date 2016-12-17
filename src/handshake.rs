use std::io::{Error, ErrorKind, Read, Write};
use md5;
use rand;
use futures::{self, Future, BoxFuture};
use handy_async::pattern::{Pattern, Endian, Buf};
use handy_async::pattern::combinators::BE;
use handy_async::pattern::read::{U8, U16, U32, Utf8};
use handy_async::io::{ReadFrom, WriteInto, ExternalSize};

use epmd::NodeInfo;

pub const TAG_NAME: u8 = 'n' as u8;
pub const TAG_STATUS: u8 = 's' as u8;
pub const TAG_CHALLENGE: u8 = 'n' as u8;
pub const TAG_REPLY: u8 = 'r' as u8;
pub const TAG_ACK: u8 = 'a' as u8;

bitflags! {
    pub flags DistributionFlags: u32 {
        const DFLAG_PUBLISHED = 0x01,
        const DFLAG_ATOM_CACHE = 0x02,
        const DFLAG_EXTENDED_REFERENCES = 0x04,
        const DFLAG_DIST_MONITOR = 0x08,
        const DFLAG_FUN_TAGS = 0x10,
        const DFLAG_DIST_MONITOR_NAME = 0x20,
        const DFLAG_HIDDEN_ATOM_CACHE = 0x40,
        const DFLAG_NEW_FUN_TAGS = 0x80,
        const DFLAG_EXTENDED_PIDS_PORTS = 0x100,
        const DFLAG_EXPORT_PTR_TAG = 0x200,
        const DFLAG_BIT_BINARIES = 0x400,
        const DFLAG_NEW_FLOATS = 0x800,
        const DFLAG_UNICODE_IO = 0x1000,
        const DFLAG_DIST_HDR_ATOM_CACHE = 0x2000,
        const DFLAG_SMALL_ATOM_TAGS = 0x4000,
        const DFLAG_UTF8_ATOMS = 0x10000,
        const DFLAG_MAP_TAG = 0x20000,
    }
}

pub type Connect<S> = BoxFuture<S, Error>;

// TODO: use handy_async::io::StatefulStream struct
// pub type Connect<S> = BoxFuture<(S, DistributionFlags), Error>;

#[derive(Debug, Clone)]
pub struct Handshake {
    local_node: NodeInfo,
    local_host: String,
    in_cookie: String,
    out_cookie: String,
    flags: DistributionFlags,
}
impl Handshake {
    pub fn new(local_node: NodeInfo, cookie: &str) -> Self {
        // TODO
        let flags = DFLAG_EXTENDED_REFERENCES | DFLAG_EXTENDED_PIDS_PORTS | DFLAG_NEW_FUN_TAGS |
                    DFLAG_NEW_FLOATS | DFLAG_MAP_TAG;
        Handshake {
            local_node: local_node,
            local_host: "127.0.0.1".to_string(),
            in_cookie: cookie.to_string(),
            out_cookie: cookie.to_string(),
            flags: flags,
        }
    }
    pub fn flags(&mut self, flags: DistributionFlags) -> &mut Self {
        self.flags = flags;
        self
    }
    pub fn in_cookie(&mut self, cookie: &str) -> &mut Self {
        self.in_cookie = cookie.to_string();
        self
    }
    pub fn out_cookie(&mut self, cookie: &str) -> &mut Self {
        self.out_cookie = cookie.to_string();
        self
    }
    pub fn hostname(&mut self, hostname: &str) -> &mut Self {
        self.local_host = hostname.to_string();
        self
    }
    pub fn connect<S>(&self, peer: S) -> Connect<S>
        where S: Read + Write + Send + 'static
    {
        // 1) `peer` is a connected stream
        let Handshake { local_node, in_cookie, out_cookie, flags, .. } = self.clone();
        futures::finished(peer)
            .and_then(move |peer| {
                // 2) send_name
                println!("2) send_name");
                let name = local_node.name;
                let version = local_node.lowest_version;
                request((TAG_NAME, version.be(), flags.bits().be(), name)).write_into(peer)
            })
            .and_then(|(peer, _)| {
                // 3) recv_status
                println!("3) recv_status");
                let pattern = U16.be()
                    .and_then(|len| {
                        let status = Utf8(vec![0; len as usize -1]).and_then(check_status);
                        (U8.expect_eq(TAG_STATUS), status)
                    });
                pattern.read_from(peer)
            })
            .and_then(|(peer, _)| {
                // 4) recv_challenge
                println!("4) recv_challenge");
                let pattern = U16.be().and_then(|len| {
                    let name = Utf8(vec![0; len as usize - 11]); // TODO: boundary check
                    (U8.expect_eq(TAG_CHALLENGE), U16.be(), U32.be(), U32.be(), name)
                });
                pattern.read_from(peer)
            })
            .and_then(move |(peer, (_, _version, flags, in_challenge, _peer_name))| {
                // 5) send_challenge_reply
                // TODO: check flags (split `Handshake.flag` field to `in_flags` and `out_flags`)
                let flags = DistributionFlags::from_bits_truncate(flags);
                println!("5) send_challenge_reply: {:?}", flags);
                let in_digest = calc_digest(&in_cookie, in_challenge);
                let out_challenge = rand::random::<u32>();
                let out_digest = calc_digest(&out_cookie, out_challenge);

                // TODO: Fix handy_aysnc's ExternalSize implementation for Buf
                let in_digest = Vec::from(&in_digest[..]);

                let reply = request((TAG_REPLY, out_challenge.be(), Buf(in_digest)))
                    .map(move |_| out_digest);
                reply.write_into(peer)
            })
            .and_then(|(peer, out_digest)| {
                // 6) recv_challenge_ack
                println!("6) recv_challenge_ack");
                let digest = Buf([0; 16]).expect_eq(out_digest);
                let ack = (U16.be().expect_eq(17), U8.expect_eq(TAG_ACK), digest);
                ack.read_from(peer)
            })
            .and_then(|(peer, _)| Ok(peer))
            .map_err(|e| e.into_error())
            .boxed()
    }
}

fn check_status(status: String) -> Result<(), Error> {
    match status.as_str() {
        "ok" |
        "ok_simultaneous" => Ok(()),
        "nok" | "now_allowed" | "alive" => {
            let e = Error::new(ErrorKind::ConnectionRefused,
                               format!("Handshake request is refused by the reason {:?}", status));
            Err(e)
        }
        _ => {
            let e = Error::new(ErrorKind::Other, format!("Unknown status: {:?}", status));
            Err(e)
        }
    }
}

fn request<P: ExternalSize>(pattern: P) -> (BE<u16>, P) {
    ((pattern.external_size() as u16).be(), pattern)
}

fn calc_digest(cookie: &str, challenge: u32) -> [u8; 16] {
    md5::compute(&format!("{}{}", cookie, challenge)).0
}
