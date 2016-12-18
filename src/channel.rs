use std::io::{Write, Error};
use futures::{Sink, StartSend, Poll};

use message::Message;

pub fn sender<W>(writer: W) -> Sender<W>
    where W: Write
{
    Sender(writer)
}

#[derive(Debug, Clone)]
pub struct Sender<W>(W);
impl<W> Sink for Sender<W>
    where W: Write
{
    type SinkItem = Message;
    type SinkError = Error;
    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        panic!()
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        panic!()
    }
}
