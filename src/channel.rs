//! Channel implementation for sending/receiving messages between distributed Erlang nodes.
use std::mem;
use std::io::{Read, Write, Error};
use futures::{Sink, StartSend, AsyncSink, Poll, Async, Future};
use futures::{Stream, BoxFuture};
use handy_async::io::AsyncWrite;
use handy_async::io::futures::WriteAll;

use message::Message;

/// Creates the receiver side of a channel to communicate with the node connected by `reader`.
///
/// # Note
///
/// Before calling this function,
/// the distribution handshake on `reader` must have been completed.
pub fn receiver<R>(reader: R) -> Receiver<R>
    where R: Read + Send + 'static
{
    Receiver(recv_message(reader))
}

/// Creates the sender side of a channel to communicate with the node connected by `writer`.
///
/// # Note
///
/// Before calling this function,
/// the distribution handshake on `writer` must have been completed.
pub fn sender<W>(writer: W) -> Sender<W>
    where W: Write + Send + 'static
{
    Sender(SenderInner::Idle(writer))
}


/// The receiver side of a channel.
pub struct Receiver<R>(BoxFuture<(R, Message), Error>);
impl<R: Read + Send + 'static> Stream for Receiver<R> {
    type Item = Message;
    type Error = Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.0.poll()? {
            Async::Ready((r, m)) => {
                self.0 = recv_message(r);
                Ok(Async::Ready(Some(m)))
            }
            Async::NotReady => Ok(Async::NotReady),
        }
    }
}

/// The sender side of a channel.
#[derive(Debug)]
pub struct Sender<W: Write>(SenderInner<W>);
impl<W> Sink for Sender<W>
    where W: Write
{
    type SinkItem = Message;
    type SinkError = Error;
    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        match mem::replace(&mut self.0, SenderInner::None) {
            SenderInner::Idle(writer) => {
                let mut buf = vec![0; 4];
                item.write_into(&mut buf)?;
                let message_len = buf.len();
                buf[0] = (message_len >> 24) as u8;
                buf[1] = (message_len >> 16) as u8;
                buf[2] = (message_len >> 8) as u8;
                buf[3] = message_len as u8;
                self.0 = SenderInner::Sending(writer.async_write_all(buf));
                Ok(AsyncSink::Ready)
            }
            SenderInner::Sending(future) => {
                self.0 = SenderInner::Sending(future);
                Ok(AsyncSink::NotReady(item))
            }
            SenderInner::None => unreachable!(),
        }
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        match mem::replace(&mut self.0, SenderInner::None) {
            SenderInner::Idle(writer) => {
                self.0 = SenderInner::Idle(writer);
                Ok(Async::Ready(()))
            }
            SenderInner::Sending(mut future) => {
                if let Async::Ready((w, _)) = future.poll().map_err(|e| e.into_error())? {
                    self.0 = SenderInner::Idle(w);
                    Ok(Async::Ready(()))
                } else {
                    self.0 = SenderInner::Sending(future);
                    Ok(Async::NotReady)
                }
            }
            SenderInner::None => unreachable!(),
        }
    }
}

#[derive(Debug)]
enum SenderInner<W: Write> {
    Idle(W),
    Sending(WriteAll<W, Vec<u8>>),
    None,
}

fn recv_message<R: Read + Send + 'static>(reader: R) -> BoxFuture<(R, Message), Error> {
    use handy_async::io::ReadFrom;
    use handy_async::pattern::{Pattern, Endian};
    use handy_async::pattern::read::U32;
    U32.be()
        .and_then(|len| vec![0; len as usize])
        .and_then(|bytes| Message::read_from(&mut &bytes[..]))
        .read_from(reader)
        .map_err(|e| e.into_error())
        .boxed()
}
