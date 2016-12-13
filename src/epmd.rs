use std::io::{Sink, Error};
use std::net::SocketAddr;
use futures::{Future, BoxFuture};
use fibers;
use handy_async::pattern::{Pattern, Endian};
use handy_async::pattern::combinators::BE;
use handy_async::pattern::read::{until, U32};
use handy_async::io::misc::Counter;
use handy_async::io::{ReadFrom, WriteInto};

pub const TAG_NAMES_REQ: u8 = 110;

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

    fn connect(&self) -> fibers::net::futures::Connect {
        fibers::net::TcpStream::connect(self.server_addr.clone())
    }
}

fn request<P: WriteInto<Counter<Sink>> + Clone>(req: P) -> (BE<u16>, P) {
    let counter = Counter::with_sink();
    let (counter, _) = req.clone().write_into(counter).wait().unwrap();
    ((counter.written_size() as u16).be(), req)
}

// fn port_req<W: Write + Send + 'static>(name: &str) -> BoxPattern<PatternWriter<W>, ()> {
//     reqfmt((122u8, name.to_string())).map(|_| ()).boxed()
// }

// fn port_resp<R: Read + Send + 'static>() -> BoxPattern<PatternReader<R>, u8> {
//     U8.boxed()
// }


// fn strbuf(len: usize) -> String {
//     String::from_utf8(vec![0; len]).unwrap()
// }

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
