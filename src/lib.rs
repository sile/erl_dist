extern crate eetf;
extern crate futures;
extern crate handy_async;
extern crate md5;
extern crate rand;
#[macro_use]
extern crate bitflags;

macro_rules! invalid_data {
    ($fmt:expr) => { invalid_data!($fmt,); };
    ($fmt:expr, $($arg:tt)*) => {
        ::std::io::Error::new(::std::io::ErrorKind::InvalidData, format!($fmt, $($arg)*));
    };
}

pub use epmd::EpmdClient;

pub mod epmd;
pub mod handshake;
pub mod channel;
pub mod message;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Creation(u8);
impl Creation {
    fn from_u16(c: u16) -> Option<Self> {
        if c < 4 { Some(Creation(c as u8)) } else { None }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
