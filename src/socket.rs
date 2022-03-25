use byteorder::{BigEndian, ByteOrder as _};
use futures::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _};

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

    pub fn get_ref(&self) -> &T {
        &self.inner
    }

    pub fn into_inner(self) -> T {
        self.inner
    }

    pub async fn write_u8(&mut self, v: u8) -> std::io::Result<()> {
        self.inner.write_all(&[v]).await
    }

    pub async fn write_u16(&mut self, v: u16) -> std::io::Result<()> {
        let mut buf = [0; 2];
        BigEndian::write_u16(&mut buf, v);
        self.inner.write_all(&buf).await
    }

    pub async fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.inner.write_all(&buf).await
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

    pub async fn read_string(&mut self) -> std::io::Result<String> {
        let mut buf = String::new();
        self.inner.read_to_string(&mut buf).await?;
        Ok(buf)
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
