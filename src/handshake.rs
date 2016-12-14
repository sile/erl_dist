use std::net::{IpAddr, SocketAddr};
use std::io::{Sink, Error, ErrorKind, Write};
use md5;
use rand;
use futures::{self, Future, BoxFuture};
use fibers;
use handy_async::pattern::{Pattern, BoxPattern, Endian};
use handy_async::pattern::combinators::BE;
use handy_async::pattern::read::{U8, U16, U32};
use handy_async::io::misc::Counter;
use handy_async::io::{ReadFrom, WriteInto, PatternWriter};

use super::epmd::NodeInfo;

pub const TAG_SEND_NAME: u8 = 110;
pub const TAG_RECV_STATUS: u8 = 115;

pub const DFLAG_PUBLISHED: u32 = 0x01;
pub const DFLAG_ATOM_CACHE: u32 = 0x02;
pub const DFLAG_EXTENDED_REFERENCES: u32 = 0x04;
pub const DFLAG_DIST_MONITOR: u32 = 0x08;
pub const DFLAG_FUN_TAGS: u32 = 0x10;
pub const DFLAG_DIST_MONITOR_NAME: u32 = 0x20;
pub const DFLAG_HIDDEN_ATOM_CACHE: u32 = 0x40;
pub const DFLAG_NEW_FUN_TAGS: u32 = 0x80;
pub const DFLAG_EXTENDED_PIDS_PORTS: u32 = 0x100;
pub const DFLAG_EXPORT_PTR_TAG: u32 = 0x200;
pub const DFLAG_BIT_BINARIES: u32 = 0x400;
pub const DFLAG_NEW_FLOATS: u32 = 0x800;
pub const DFLAG_UNICODE_IO: u32 = 0x1000;
pub const DFLAG_DIST_HDR_ATOM_CACHE: u32 = 0x2000;
pub const DFLAG_SMALL_ATOM_TAGS: u32 = 0x4000;
pub const DFLAG_UTF8_ATOMS: u32 = 0x10000;
pub const DFLAG_MAP_TAG: u32 = 0x20000;

pub struct Handshake;
impl Handshake {
    pub fn connect(local_node: String,
                   peer_ip: IpAddr,
                   peer_info: NodeInfo,
                   cookie: String)
                   -> BoxFuture<fibers::net::TcpStream, Error> {
        let addr = SocketAddr::new(peer_ip, peer_info.port);
        println!("ADDR: {}", addr);
        println!("IP: {}", peer_ip);
        println!("INFO: {:?}", peer_info);
        fibers::net::TcpStream::connect(addr)
            .and_then(move |socket| {
                send_name_req(local_node, &peer_info).write_into(socket).map_err(|e| e.into_error())
            })
            .and_then(|(socket, _)| {
                println!("CONNECTED: {:?}", socket.peer_addr());
                let resp = U16.be().and_then(|len| (U8, strbuf(len as usize - 1)));
                resp.read_from(socket).map_err(|e| e.into_error())
            })
            .and_then(|(socket, (tag, status))| {
                assert_eq!(tag, TAG_RECV_STATUS); // TODO
                println!("STATUS: {}", status);
                if status == "ok" {
                    // 'recv_challenge'
                    let resp = U16.be().and_then(|len| {
                        (U8, U16.be(), U32.be(), U32.be(), strbuf(len as usize - 11))
                    });
                    resp.read_from(socket).map_err(|e| e.into_error()).boxed()
                } else {
                    // TODO: handle 'ok_simultaneous', 'alive'
                    futures::failed(Error::new(ErrorKind::Other,
                                               format!("'send_name' failed: status={}", status)))
                        .boxed()
                }
            })
            .and_then(move |(socket, x)| {
                println!("RECV_CHALLENGE: {:?}", x);
                let (tag, _version, _flags, challenge, _peer_name) = x;
                assert_eq!(tag, 'n' as u8); // TODO

                let digest = Vec::from(&md5::compute(&format!("{}{}", cookie, challenge)).0[..]);
                let my_challenge = rand::random::<u32>();
                let my_digest = md5::compute(&format!("{}{}", cookie, my_challenge));
                let rep = request(('r' as u8, my_challenge.be(), digest));
                rep.write_into(socket)
                    .and_then(|(socket, _)| {
                        let resp = (U16.be(), U8, vec![0; 16]);
                        resp.read_from(socket)
                    })
                    .map_err(|e| e.into_error())
                    .map(move |v| (v, my_digest))
            })
            .and_then(|((socket, x), my_digest)| {
                println!("RECV_CHALLENGE_ACK: {:?}", x);
                let (len, tag, digest) = x;
                assert_eq!(len, 17);
                assert_eq!(tag, 'a' as u8);
                assert_eq!(digest, my_digest.0);
                Ok(socket)
            })
            .boxed()
    }
}

fn send_name_req<W: Write + Send + 'static>(local_node: String,
                                            info: &NodeInfo)
                                            -> BoxPattern<PatternWriter<W>, ()> {
    let flags = DFLAG_EXTENDED_REFERENCES | DFLAG_EXTENDED_PIDS_PORTS | DFLAG_NEW_FUN_TAGS |
                DFLAG_NEW_FLOATS | DFLAG_MAP_TAG;
    request((TAG_SEND_NAME, info.highest_version.be(), flags.be(), local_node))
        .map(|_| ())
        .boxed()
}

fn request<P: WriteInto<Counter<Sink>> + Clone>(req: P) -> (BE<u16>, P) {
    let counter = Counter::with_sink();
    let (counter, _) = req.clone().write_into(counter).wait().unwrap();
    ((counter.written_size() as u16).be(), req)
}

fn strbuf(len: usize) -> String {
    String::from_utf8(vec![0; len]).unwrap()
}
