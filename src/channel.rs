#[cfg(doc)]
use crate::handshake;
use crate::io::Connection;
use crate::message::Message;
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
    connection: Connection<T>,
}

impl<T> Sender<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    fn new(connection: T) -> Self {
        Self {
            connection: Connection::new(connection),
        }
    }

    /// Sends a message.
    pub async fn send(&mut self, message: Message) -> Result<(), SendError> {
        if matches!(message, Message::Tick) {
            self.connection.write_u32(0).await?;
        } else {
            let mut buf = Vec::new();
            message.write_into(&mut buf)?;

            self.connection.write_u32(1 + buf.len() as u32).await?;
            self.connection.write_u8(TYPE_TAG).await?;
            self.connection.write_all(&buf).await?;
            self.connection.flush().await?;
        }
        Ok(())
    }
}

/// Receiver of a message channel.
#[derive(Debug)]
pub struct Receiver<T> {
    connection: Connection<T>,
}

impl<T> Receiver<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    fn new(connection: T) -> Self {
        Self {
            connection: Connection::new(connection),
        }
    }

    /// Receives a message.
    pub async fn recv(&mut self) -> Result<Message, RecvError> {
        let size = match self.connection.read_u32().await {
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

        let tag = self.connection.read_u8().await?;
        if tag != TYPE_TAG {
            return Err(RecvError::UnexpectedTypeTag { tag });
        }

        let mut buf = vec![0; size - 1];
        self.connection.read_exact(&mut buf).await?;
        Message::read_from(&mut buf.as_slice())
    }

    /// Receives a message (owned version).
    pub async fn recv_owned(mut self) -> Result<(Message, Self), RecvError> {
        let msg = self.recv().await?;
        Ok((msg, self))
    }
}

/// Possible errors during sending messages.
#[derive(Debug)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum SendError {
    /// Encode error.
    Encode(eetf::EncodeError),

    /// I/O error.
    Io(std::io::Error),
}

impl std::fmt::Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Encode(error) => write!(f, "{error}"),
            Self::Io(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SendError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Encode(error) => Some(error),
            Self::Io(error) => Some(error),
        }
    }
}

impl From<std::io::Error> for SendError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<eetf::EncodeError> for SendError {
    fn from(value: eetf::EncodeError) -> Self {
        Self::Encode(value)
    }
}

/// Possible errors during receiving messages.
#[derive(Debug)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum RecvError {
    /// Connection was closed by the peer.
    Closed,

    /// Unsupported distributed operation.
    UnsupportedOp { op: i32 },

    /// Unexpected type tag.
    UnexpectedTypeTag { tag: u8 },

    /// Decode error.
    Decode(eetf::DecodeError),

    /// I/O error.
    Io(std::io::Error),
}

impl std::fmt::Display for RecvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Closed => write!(f, "connection was closed by the peer"),
            Self::UnsupportedOp { op } => write!(f, "unsupported distributed operation {op}"),
            Self::UnexpectedTypeTag { tag } => {
                write!(f, "expected type tag {TYPE_TAG} but got {tag}")
            }
            Self::Decode(error) => write!(f, "{error}"),
            Self::Io(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for RecvError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Decode(e) => Some(e),
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for RecvError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<eetf::DecodeError> for RecvError {
    fn from(value: eetf::DecodeError) -> Self {
        Self::Decode(value)
    }
}
