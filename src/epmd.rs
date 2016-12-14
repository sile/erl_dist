use std::io::{Sink, Error, ErrorKind, Read};
use std::net::SocketAddr;
use futures::{Future, BoxFuture};
use fibers;
use handy_async::pattern::{Pattern, BoxPattern, Endian, Branch};
use handy_async::pattern::combinators::BE;
use handy_async::pattern::read::{until, U8, U16, U32};
use handy_async::io::misc::Counter;
use handy_async::io::{ReadFrom, WriteInto, PatternReader};

pub const TAG_NAMES_REQ: u8 = 110;
pub const TAG_PORT_PLEASE2_REQ: u8 = 122;
pub const TAG_PORT2_RESP: u8 = 119;

pub struct Client {
    server_addr: SocketAddr,
}
impl Client {
    pub fn new(server_addr: &SocketAddr) -> Self {
        Client { server_addr: server_addr.clone() }
    }
    pub fn names(&self) -> BoxFuture<String, Error> {
        self.connect()
            .and_then(|socket| {
                let req = request(TAG_NAMES_REQ);
                req.write_into(socket).map_err(|e| e.into_error())
            })
            .and_then(|(socket, _)| {
                // TODO:
                let resp = (U32.be(), until(|_, eos| if eos { Ok(Some(())) } else { Ok(None) }))
                    .map(|(_, (s, _))| String::from_utf8(s).unwrap());
                resp.read_from(socket).map(|(_, s)| s).map_err(|e| e.into_error())
            })
            .boxed()
    }
    pub fn port_please(&self, node: &str) -> BoxFuture<Option<NodeInfo>, Error> {
        let node = node.to_string();
        self.connect()
            .and_then(move |socket| {
                let req = request((TAG_PORT_PLEASE2_REQ, node));
                req.write_into(socket).map_err(|e| e.into_error())
            })
            .and_then(|(socket, _)| {
                let resp = U8.and_then(|tag| {
                        if tag == TAG_PORT2_RESP {
                            Branch::A(U8)
                        } else {
                            let e = invalid_data(&format!("Unexpected response tag: {}", tag));
                            Branch::B(Err(e)) as Branch<_, _>
                        }
                    })
                    .and_then(|result| {
                        if result == 0 {
                            Some(read_node_info())
                        } else {
                            None
                        }
                    });
                resp.read_from(socket).map(|(_, r)| r).map_err(|e| e.into_error())
            })
            .boxed()
    }

    fn connect(&self) -> fibers::net::futures::Connect {
        fibers::net::TcpStream::connect(self.server_addr.clone())
    }
}

fn read_node_info<R: Read + Send + 'static>() -> BoxPattern<PatternReader<R>, NodeInfo> {
    (U16.be(),
     U8,
     U8,
     U16.be(),
     U16.be(),
     U16.be().and_then(|len| strbuf(len as usize)),
     U16.be().and_then(|len| vec![0; len as usize]))
        .map(|t| {
            NodeInfo {
                port: t.0,
                node_type: t.1,
                protocol: t.2,
                highest_version: t.3,
                lowest_version: t.4,
                node_name: t.5,
                extra: t.6,
            }
        })
        .boxed()
}

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub port: u16,
    pub node_type: u8,
    pub protocol: u8,
    pub highest_version: u16,
    pub lowest_version: u16,
    pub node_name: String,
    pub extra: Vec<u8>,
}

fn invalid_data(description: &str) -> Error {
    Error::new(ErrorKind::InvalidData, description.to_string())
}

fn request<P: WriteInto<Counter<Sink>> + Clone>(req: P) -> (BE<u16>, P) {
    let counter = Counter::with_sink();
    let (counter, _) = req.clone().write_into(counter).wait().unwrap();
    ((counter.written_size() as u16).be(), req)
}

fn strbuf(len: usize) -> String {
    String::from_utf8(vec![0; len]).unwrap()
}

// fn alive2_req<W: Write + Send + 'static>(port: u16) -> BoxPattern<PatternWriter<W>, ()> {
//     reqfmt((120u8, port.be(), 77u8, 0, 5, 5, ("bar".len() as u16).be(), "bar".to_string(), 0))
//         .map(|_| ())
//         .boxed()
// }

// fn alive2_resp<R: Read + Send + 'static>() -> BoxPattern<PatternReader<R>, (u8, u16)> {
//     (U8, U8, U16.be())
//         .map(|(tag, result, creation)| {
//             assert_eq!(tag, 121);
//             (result, creation)
//         })
//         .boxed()
// }
