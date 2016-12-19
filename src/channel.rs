use std::mem;
use std::io::{Read, Write, Error};
use futures::{Sink, StartSend, AsyncSink, Poll, Async, Future};
use futures::{Stream, BoxFuture};
use handy_async::io::AsyncWrite;
use handy_async::io::futures::WriteAll;

use message::Message;

fn recv_message<R: Read + Send + 'static>(reader: R) -> BoxFuture<(R, Option<Message>), Error> {
    use handy_async::io::ReadFrom;
    use handy_async::pattern::{Pattern, Endian};
    use handy_async::pattern::read::U32;
    U32.be()
        .and_then(|len| vec![0; len as usize])
        .and_then(|bytes| {
            if bytes.is_empty() {
                // heartbeat request(?)
                Ok(None)
            } else {
                Ok(Some(Message::from_bytes(&bytes)?))
            }
        })
        .read_from(reader)
        .map_err(|e| e.into_error())
        .boxed()
}

pub fn receiver<R: Read + Send + 'static>(reader: R) -> Receiver<R> {
    Receiver(recv_message(reader))
}

pub struct Receiver<R>(BoxFuture<(R, Option<Message>), Error>);
impl<R: Read + Send + 'static> Stream for Receiver<R> {
    type Item = Message;
    type Error = Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.0.poll()? {
            Async::Ready((r, None)) => {
                self.0 = recv_message(r);
                Ok(Async::NotReady)
            }
            Async::Ready((r, Some(m))) => {
                self.0 = recv_message(r);
                Ok(Async::Ready(Some(m)))
            }
            Async::NotReady => Ok(Async::NotReady),
        }
    }
}

pub fn sender<W: Write>(writer: W) -> Sender<W> {
    Sender(SenderInner::Idle(writer))
}

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
                let bytes = item.into_bytes()?;
                self.0 = SenderInner::Sending(writer.async_write_all(bytes));
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
