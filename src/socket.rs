use byteorder::{BigEndian, ByteOrder as _, WriteBytesExt};
use futures::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _};

#[derive(Debug)]
pub struct MessageWriter<'a, T> {
    socket: &'a mut Socket<T>,
    buf: Vec<u8>,
}

impl<'a, T> MessageWriter<'a, T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn finish(self) -> std::io::Result<()> {
        self.socket.write_u16(self.buf.len() as u16).await?; // TODO: validation
        self.socket.write_all(&self.buf).await?;
        self.socket.flush().await?;
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
pub struct MessageReader<'a, T> {
    socket: &'a mut Socket<T>,
    size: usize,
}

impl<'a, T> MessageReader<'a, T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn read_u8(&mut self) -> std::io::Result<u8> {
        self.size = self.size.checked_sub(1).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "unexpected eof")
        })?;
        self.socket.read_u8().await
    }

    pub async fn read_u16(&mut self) -> std::io::Result<u16> {
        self.size = self.size.checked_sub(2).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "unexpected eof")
        })?;
        self.socket.read_u16().await
    }

    pub async fn read_u32(&mut self) -> std::io::Result<u32> {
        self.size = self.size.checked_sub(4).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "unexpected eof")
        })?;
        self.socket.read_u32().await
    }

    pub async fn read_u64(&mut self) -> std::io::Result<u64> {
        self.size = self.size.checked_sub(8).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "unexpected eof")
        })?;
        self.socket.read_u64().await
    }

    pub async fn read_string(&mut self) -> std::io::Result<String> {
        let n = self.size;
        self.size = 0;
        self.socket.read_stringn(n).await
    }

    pub async fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        let n = buf.len();
        self.size = self.size.checked_sub(n).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "unexpected eof")
        })?;
        self.socket.read_exact(buf).await
    }

    pub async fn read_u16_string(&mut self) -> std::io::Result<String> {
        let n = self.read_u16().await? as usize;
        self.size = self.size.checked_sub(n).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "unexpected eof")
        })?;
        self.socket.read_stringn(n).await
    }

    pub async fn consume_remaining_bytes(&mut self) -> std::io::Result<()> {
        let mut buf = vec![0; self.size];
        self.size = 0;
        self.socket.read_exact(&mut buf).await?;
        Ok(())
    }
}

// An internal struct to make it easier to read from and write into a socket.
#[derive(Debug)]
pub struct Socket<T> {
    inner: T,
}

impl<T> Socket<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }

    pub fn message_writer<'a>(&'a mut self) -> MessageWriter<'a, T> {
        MessageWriter {
            socket: self,
            buf: Vec::new(),
        }
    }

    pub async fn message_reader<'a>(&'a mut self) -> std::io::Result<MessageReader<'a, T>> {
        let size = self.read_u16().await? as usize;
        Ok(MessageReader { socket: self, size })
    }

    pub async fn write_u8(&mut self, v: u8) -> std::io::Result<()> {
        self.inner.write_all(&[v]).await
    }

    pub async fn write_u16(&mut self, v: u16) -> std::io::Result<()> {
        let mut buf = [0; 2];
        BigEndian::write_u16(&mut buf, v);
        self.inner.write_all(&buf).await
    }

    // TODO: remove
    // pub async fn write_u32(&mut self, v: u32) -> std::io::Result<()> {
    //     let mut buf = [0; 4];
    //     BigEndian::write_u32(&mut buf, v);
    //     self.inner.write_all(&buf).await
    // }

    pub async fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.inner.write_all(buf).await
    }

    // TODO: validation
    // pub async fn write_u16_str(&mut self, s: &str) -> std::io::Result<()> {
    //     self.write_u16(s.len() as u16).await?;
    //     self.inner.write_all(s.as_bytes()).await?;
    //     Ok(())
    // }

    // pub async fn write_u16_bytes(&mut self, buf: &[u8]) -> std::io::Result<()> {
    //     self.write_u16(buf.len() as u16).await?; // TODO: validation
    //     self.inner.write_all(buf).await?;
    //     Ok(())
    // }

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
