use byteorder::{BigEndian, ByteOrder as _, WriteBytesExt};
use eetf::{DecodeError, EncodeError, FixInteger, Term, Tuple};
use futures::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _};
use std::io::{Read, Write};

#[derive(Debug)]
pub struct Connection<T> {
    inner: T,
}

impl<T> Connection<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }

    pub fn handshake_message_writer(&mut self) -> HandshakeMessageWriter<T> {
        HandshakeMessageWriter {
            connection: self,
            buf: Vec::new(),
        }
    }

    #[allow(clippy::needless_lifetimes)]
    pub async fn handshake_message_reader<'a>(
        &'a mut self,
    ) -> std::io::Result<HandshakeMessageReader<'a, T>> {
        let size = self.read_u16().await? as usize;
        Ok(HandshakeMessageReader {
            connection: self,
            size,
        })
    }

    pub async fn write_u8(&mut self, v: u8) -> std::io::Result<()> {
        self.inner.write_all(&[v]).await
    }

    pub async fn write_u16(&mut self, v: u16) -> std::io::Result<()> {
        let mut buf = [0; 2];
        BigEndian::write_u16(&mut buf, v);
        self.inner.write_all(&buf).await
    }

    pub async fn write_u32(&mut self, v: u32) -> std::io::Result<()> {
        let mut buf = [0; 4];
        BigEndian::write_u32(&mut buf, v);
        self.inner.write_all(&buf).await
    }

    pub async fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.inner.write_all(buf).await
    }

    pub async fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush().await
    }

    pub async fn read_u8(&mut self) -> std::io::Result<u8> {
        let mut buf = [0; 1];
        self.inner.read_exact(&mut buf).await?;
        Ok(buf[0])
    }

    pub async fn read_u16(&mut self) -> std::io::Result<u16> {
        let mut buf = [0; 2];
        self.inner.read_exact(&mut buf).await?;
        Ok(BigEndian::read_u16(&buf))
    }

    pub async fn read_u32(&mut self) -> std::io::Result<u32> {
        let mut buf = [0; 4];
        self.inner.read_exact(&mut buf).await?;
        Ok(BigEndian::read_u32(&buf))
    }

    pub async fn read_u64(&mut self) -> std::io::Result<u64> {
        let mut buf = [0; 8];
        self.inner.read_exact(&mut buf).await?;
        Ok(BigEndian::read_u64(&buf))
    }

    pub async fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        self.inner.read_exact(buf).await
    }

    pub async fn read_string(&mut self) -> std::io::Result<String> {
        let mut buf = String::new();
        self.inner.read_to_string(&mut buf).await?;
        Ok(buf)
    }

    pub async fn read_stringn(&mut self, size: usize) -> std::io::Result<String> {
        let mut buf = vec![0; size];
        self.inner.read_exact(&mut buf).await?;
        String::from_utf8(buf).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "stream did not contain valid UTF-8",
            )
        })
    }

    pub async fn read_u16_bytes(&mut self) -> std::io::Result<Vec<u8>> {
        let mut buf = vec![0; usize::from(self.read_u16().await?)];
        self.inner.read_exact(&mut buf).await?;
        Ok(buf)
    }

    pub async fn read_u16_string(&mut self) -> std::io::Result<String> {
        let buf = self.read_u16_bytes().await?;
        String::from_utf8(buf).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "stream did not contain valid UTF-8",
            )
        })
    }
}

#[derive(Debug)]
pub struct HandshakeMessageWriter<'a, T> {
    connection: &'a mut Connection<T>,
    buf: Vec<u8>,
}

impl<'a, T> HandshakeMessageWriter<'a, T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn finish(self) -> std::io::Result<()> {
        if self.buf.len() > u16::MAX as usize {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "too large bytes: expected less then {}, but got {} bytes",
                    u16::MAX as usize + 1,
                    self.buf.len()
                ),
            ));
        }
        self.connection.write_u16(self.buf.len() as u16).await?;
        self.connection.write_all(&self.buf).await?;
        self.connection.flush().await?;
        Ok(())
    }

    pub fn write_u8(&mut self, v: u8) -> std::io::Result<()> {
        self.buf.write_u8(v)
    }

    pub fn write_u16(&mut self, v: u16) -> std::io::Result<()> {
        self.buf.write_u16::<BigEndian>(v)
    }

    pub fn write_u32(&mut self, v: u32) -> std::io::Result<()> {
        self.buf.write_u32::<BigEndian>(v)
    }

    pub fn write_u64(&mut self, v: u64) -> std::io::Result<()> {
        self.buf.write_u64::<BigEndian>(v)
    }

    pub fn write_all(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.buf.extend_from_slice(bytes);
        Ok(())
    }
}

#[derive(Debug)]
pub struct HandshakeMessageReader<'a, T> {
    connection: &'a mut Connection<T>,
    size: usize,
}

impl<'a, T> HandshakeMessageReader<'a, T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn read_u8(&mut self) -> std::io::Result<u8> {
        self.size = self.size.checked_sub(1).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "unexpected eof")
        })?;
        self.connection.read_u8().await
    }

    pub async fn read_u16(&mut self) -> std::io::Result<u16> {
        self.size = self.size.checked_sub(2).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "unexpected eof")
        })?;
        self.connection.read_u16().await
    }

    pub async fn read_u32(&mut self) -> std::io::Result<u32> {
        self.size = self.size.checked_sub(4).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "unexpected eof")
        })?;
        self.connection.read_u32().await
    }

    pub async fn read_u64(&mut self) -> std::io::Result<u64> {
        self.size = self.size.checked_sub(8).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "unexpected eof")
        })?;
        self.connection.read_u64().await
    }

    pub async fn read_string(&mut self) -> std::io::Result<String> {
        let n = self.size;
        self.size = 0;
        self.connection.read_stringn(n).await
    }

    pub async fn read_bytes(&mut self) -> std::io::Result<Vec<u8>> {
        let n = self.size;
        let mut buf = vec![0; n];
        self.read_exact(&mut buf).await?;
        Ok(buf)
    }

    pub async fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        let n = buf.len();
        self.size = self.size.checked_sub(n).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "unexpected eof")
        })?;
        self.connection.read_exact(buf).await
    }

    pub async fn read_u16_string(&mut self) -> std::io::Result<String> {
        let n = self.read_u16().await? as usize;
        self.size = self.size.checked_sub(n).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "unexpected eof")
        })?;
        self.connection.read_stringn(n).await
    }

    pub async fn consume_remaining_bytes(&mut self) -> std::io::Result<()> {
        let mut buf = vec![0; self.size];
        self.size = 0;
        self.connection.read_exact(&mut buf).await?;
        Ok(())
    }

    pub async fn finish(mut self) -> std::io::Result<()> {
        self.consume_remaining_bytes().await
    }
}

pub trait ReadTermExt: Read {
    fn read_tuple(&mut self) -> Result<Tuple, DecodeError> {
        let term = self.read_term()?;
        term.try_into()
            .map_err(|value| DecodeError::UnexpectedType {
                value,
                expected: "Tuple".to_owned(),
            })
    }

    fn read_term(&mut self) -> Result<Term, DecodeError> {
        Term::decode(self)
    }
}

impl<T: Read> ReadTermExt for T {}

pub trait WriteTermExt: Write {
    fn write_tagged_tuple1(&mut self, tag: i32) -> Result<(), EncodeError> {
        let tuple = Tuple {
            elements: vec![Term::from(FixInteger { value: tag as i32 })],
        };
        self.write_term(tuple)
    }

    fn write_tagged_tuple3<T0, T1>(
        &mut self,
        tag: i32,
        term0: T0,
        term1: T1,
    ) -> Result<(), EncodeError>
    where
        Term: From<T0>,
        Term: From<T1>,
    {
        let tuple = Tuple {
            elements: vec![
                Term::from(FixInteger { value: tag as i32 }),
                Term::from(term0),
                Term::from(term1),
            ],
        };
        self.write_term(tuple)
    }

    fn write_tagged_tuple4<T0, T1, T2>(
        &mut self,
        tag: i32,
        term0: T0,
        term1: T1,
        term2: T2,
    ) -> Result<(), EncodeError>
    where
        Term: From<T0>,
        Term: From<T1>,
        Term: From<T2>,
    {
        let tuple = Tuple {
            elements: vec![
                Term::from(FixInteger { value: tag as i32 }),
                Term::from(term0),
                Term::from(term1),
                Term::from(term2),
            ],
        };
        self.write_term(tuple)
    }

    fn write_tagged_tuple5<T0, T1, T2, T3>(
        &mut self,
        tag: i32,
        term0: T0,
        term1: T1,
        term2: T2,
        term3: T3,
    ) -> Result<(), EncodeError>
    where
        Term: From<T0>,
        Term: From<T1>,
        Term: From<T2>,
        Term: From<T3>,
    {
        let tuple = Tuple {
            elements: vec![
                Term::from(FixInteger { value: tag as i32 }),
                Term::from(term0),
                Term::from(term1),
                Term::from(term2),
                Term::from(term3),
            ],
        };
        self.write_term(tuple)
    }

    fn write_tagged_tuple6<T0, T1, T2, T3, T4>(
        &mut self,
        tag: i32,
        term0: T0,
        term1: T1,
        term2: T2,
        term3: T3,
        term4: T4,
    ) -> Result<(), EncodeError>
    where
        Term: From<T0>,
        Term: From<T1>,
        Term: From<T2>,
        Term: From<T3>,
        Term: From<T4>,
    {
        let tuple = Tuple {
            elements: vec![
                Term::from(FixInteger { value: tag as i32 }),
                Term::from(term0),
                Term::from(term1),
                Term::from(term2),
                Term::from(term3),
                Term::from(term4),
            ],
        };
        self.write_term(tuple)
    }

    #[allow(clippy::too_many_arguments)]
    fn write_tagged_tuple7<T0, T1, T2, T3, T4, T5>(
        &mut self,
        tag: i32,
        term0: T0,
        term1: T1,
        term2: T2,
        term3: T3,
        term4: T4,
        term5: T5,
    ) -> Result<(), EncodeError>
    where
        Term: From<T0>,
        Term: From<T1>,
        Term: From<T2>,
        Term: From<T3>,
        Term: From<T4>,
        Term: From<T5>,
    {
        let tuple = Tuple {
            elements: vec![
                Term::from(FixInteger { value: tag as i32 }),
                Term::from(term0),
                Term::from(term1),
                Term::from(term2),
                Term::from(term3),
                Term::from(term4),
                Term::from(term5),
            ],
        };
        self.write_term(tuple)
    }

    fn write_term<T>(&mut self, term: T) -> Result<(), EncodeError>
    where
        Term: From<T>,
    {
        Term::from(term).encode(self)
    }
}

impl<T: Write> WriteTermExt for T {}
