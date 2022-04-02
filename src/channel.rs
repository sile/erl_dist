#[cfg(doc)]
use crate::handshake;
use crate::message::Message;
use crate::socket::Socket;
use crate::DistributionFlags;
use futures::io::{AsyncRead, AsyncWrite};

/// Makes a channel to send/received messages to/from a connected node.
///
/// Please ensure that the [`handshake`] has been completed using the `connection` before creating a channel.
///
/// `flags` should be an intersection of distribution flags of both nodes.
/// Note that the current implementation doesn't consider the distribution flags.
///
/// Note that, to keep the connection established, you need to send `Message::Tick` periodically.
/// Please see [the official `net_ticktime` doc](https://www.erlang.org/doc/man/kernel_app.html#net_ticktime) for more details.
pub fn channel<T>(connection: T, flags: DistributionFlags) -> (Sender<T>, Receiver<T>)
where
    T: AsyncRead + AsyncWrite + Unpin + Clone,
{
    let _ = flags;
    (Sender::new(connection.clone()), Receiver::new(connection))
}

const TYPE_TAG: u8 = 112;

/// Sender of a message channel.
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

    /// Sends a message.
    pub async fn send(&mut self, message: Message) -> Result<(), SendError> {
        if matches!(message, Message::Tick) {
            self.socket.write_u32(0).await?;
        } else {
            let mut buf = Vec::new();
            message.write_into(&mut buf)?;

            self.socket.write_u32(1 + buf.len() as u32).await?;
            self.socket.write_u8(TYPE_TAG).await?;
            self.socket.write_all(&buf).await?;
            self.socket.flush().await?;
        }
        Ok(())
    }
}

/// Receiver of a message channel.
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

    /// Receives a message.
    pub async fn recv(&mut self) -> Result<Message, RecvError> {
        let size = match self.socket.read_u32().await {
            Ok(size) => size as usize,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    return Err(RecvError::Closed);
                } else {
                    return Err(e.into());
                }
            }
        };
        if size == 0 {
            return Ok(Message::Tick);
        }

        let tag = self.socket.read_u8().await?;
        if tag != TYPE_TAG {
            return Err(RecvError::UnexpectedTypeTag { tag });
        }

        let mut buf = vec![0; size - 1];
        self.socket.read_exact(&mut buf).await?;
        Message::read_from(&mut buf.as_slice())
    }

    /// Receives a message (owned version).
    pub async fn recv_owned(mut self) -> Result<(Message, Self), RecvError> {
        let msg = self.recv().await?;
        Ok((msg, self))
    }
}

/// Possible errors during sending messages.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum SendError {
    #[error(transparent)]
    Encode(#[from] eetf::EncodeError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Possible errors during receiving messages.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum RecvError {
    #[error("connection was closed by the peer")]
    Closed,

    #[error("unsupported distributed operation {op}")]
    UnsupportedOp { op: i32 },

    #[error("expected type tag {TYPE_TAG} but got {tag}")]
    UnexpectedTypeTag { tag: u8 },

    #[error(transparent)]
    Decode(#[from] eetf::DecodeError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
