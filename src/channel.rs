//! Channel implementation for sending/receiving messages between distributed Erlang nodes.
use crate::handshake::DistributionFlags;
use crate::message::Message;
use crate::socket::Socket;
use futures::io::{AsyncRead, AsyncWrite};

pub fn channel<T>(stream: T, _flags: DistributionFlags) -> (Sender<T>, Receiver<T>)
where
    T: AsyncRead + AsyncWrite + Unpin + Clone,
{
    (Sender::new(stream.clone()), Receiver::new(stream))
}

// TODO: support distribution header
#[derive(Debug)]
pub struct Sender<T> {
    socket: Socket<T>,
}

impl<T> Sender<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    fn new(socket: T) -> Self {
        Self {
            socket: Socket::new(socket),
        }
    }

    pub async fn send(&mut self, message: Message) -> Result<(), SendError> {
        let mut buf = Vec::new();
        message.write_into(&mut buf)?;

        self.socket.write_u32(1 + buf.len() as u32).await?;
        self.socket.write_u8(112).await?;
        self.socket.write_all(&buf).await?;
        self.socket.flush().await?;
        Ok(())
    }

    pub async fn tick(&mut self) -> Result<(), SendError> {
        self.socket.write_u32(0).await?;
        Ok(())
    }
}

const TYPE_TAG: u8 = 112;

#[derive(Debug)]
pub struct Receiver<T> {
    socket: Socket<T>,
}

impl<T> Receiver<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    fn new(socket: T) -> Self {
        Self {
            socket: Socket::new(socket),
        }
    }

    pub async fn recv(&mut self) -> Result<Message, RecvError> {
        let mut size = self.socket.read_u32().await? as usize;
        while size == 0 {
            // Ignore tick packet
            size = self.socket.read_u32().await? as usize;
        }
        let type_tag = self.socket.read_u8().await?;

        if type_tag != TYPE_TAG {
            return Err(RecvError::UnexpectedTypeTag { actual: type_tag });
        }

        let mut buf = vec![0; size - 1]; // TODO: checked_sub()
        self.socket.read_exact(&mut buf).await?;
        Message::read_from(&mut buf.as_slice())
    }

    // TODO: rename
    pub async fn recv2(mut self) -> Result<(Message, Self), RecvError> {
        let msg = self.recv().await?;
        Ok((msg, self))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SendError {
    #[error(transparent)]
    EncodeError(#[from] eetf::EncodeError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum RecvError {
    #[error("unsupported distributed operation {op}")]
    UnsupportedOp { op: i32 },

    #[error("expected type tag {TYPE_TAG} but got {actual}")]
    UnexpectedTypeTag { actual: u8 },

    #[error(transparent)]
    DecodeError(#[from] eetf::DecodeError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
