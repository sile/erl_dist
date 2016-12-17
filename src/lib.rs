extern crate eetf;
extern crate futures;
extern crate fibers;
extern crate handy_async;
extern crate md5;
extern crate rand;
extern crate regex;

macro_rules! invalid_data {
    ($fmt:expr) => { invalid_data!($fmt,); };
    ($fmt:expr, $($arg:tt)*) => {
        ::std::io::Error::new(::std::io::ErrorKind::InvalidData, format!($fmt, $($arg)*));
    };
}

pub mod epmd;
pub mod node;
pub mod handshake;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
