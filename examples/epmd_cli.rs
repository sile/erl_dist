// http://erlang.org/doc/apps/erts/erl_dist_protocol.html
extern crate erl_dist;
extern crate fibers;
extern crate futures;
extern crate handy_async;
extern crate clap;

use std::io::{Write, Read, Sink};
use futures::Future;
use fibers::Executor;
use fibers::net::TcpStream;
use handy_async::pattern::{Pattern, Endian, BoxPattern};
use handy_async::pattern::combinators::BE;
use handy_async::pattern::read::{U8, U16, U32};
use handy_async::io::misc::Counter;
use handy_async::io::{ReadFrom, PatternReader};
use handy_async::io::{WriteInto, PatternWriter};

fn main() {
    let mut executor = fibers::InPlaceExecutor::new().unwrap();
    let monitor = executor.spawn_monitor(TcpStream::connect("127.0.0.1:4369".parse().unwrap())
        .and_then(|socket| {
            let local = socket.local_addr().unwrap();
            println!("Connected: local={}", local);
            let alive2_req = alive2_req(local.port());
            alive2_req.write_into(socket).map_err(|e| e.into_error())
        })
        .and_then(|(socket, _)| {
            println!("Sent: alive2_req");
            alive2_resp().read_from(socket).map_err(|e| e.into_error())
        })
        .and_then(|(socket, resp)| {
            println!("Recv: alive2_resp: {:?}", resp);
            port_req("foo").write_into(socket).map_err(|e| e.into_error())
        })
        .and_then(|(socket, _)| {
            println!("Sent: port_req");
            port_resp().read_from(socket).map_err(|e| e.into_error())
        })
        .and_then(|(socket, resp)| {
            println!("Recv: port_resp: {:?}", resp);
            Ok(())
        }));
    let r = executor.run_fiber(monitor).unwrap();
    println!("{:?}", r);
}

fn port_req<W: Write + Send + 'static>(name: &str) -> BoxPattern<PatternWriter<W>, ()> {
    reqfmt((122u8, name.to_string())).map(|_| ()).boxed()
}

fn port_resp<R: Read + Send + 'static>() -> BoxPattern<PatternReader<R>, u8> {
    U8.boxed()
}

fn reqfmt<P: WriteInto<Counter<Sink>> + Clone>(req: P) -> (BE<u16>, P) {
    let counter = Counter::with_sink();
    let (counter, _) = req.clone().write_into(counter).wait().unwrap();
    ((counter.written_size() as u16).be(), req)
}

fn alive2_req<W: Write + Send + 'static>(port: u16) -> BoxPattern<PatternWriter<W>, ()> {
    reqfmt((120u8, port.be(), 77u8, 0, 5, 5, ("bar".len() as u16).be(), "bar".to_string(), 0))
        .map(|_| ())
        .boxed()
}

fn alive2_resp<R: Read + Send + 'static>() -> BoxPattern<PatternReader<R>, (u8, u16)> {
    (U8, U8, U16.be())
        .map(|(tag, result, creation)| {
            assert_eq!(tag, 121);
            (result, creation)
        })
        .boxed()
}
