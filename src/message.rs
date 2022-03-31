// //! Messages passed between distributed nodes.
// //!
// //! Reference: [12.3 Protocol between Connected Nodes]
// //! (http://erlang.org/doc/apps/erts/erl_dist_protocol.html#id105440).
// //!
// //! Note that distribution headers are not supported in the current version.
use eetf::{Atom, DecodeError, EncodeError, FixInteger, Pid, Reference, Term, Tuple};
use std::io::{Read, Write};

pub use crate::channel::{channel, Receiver, Sender};

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

pub trait TupleExt {
    fn check_len(&self, n: usize) -> Result<(), DecodeError>;
    // TODO: convert(?)
    fn take_as<T>(&mut self, i: usize, expected: &str) -> Result<T, DecodeError>
    where
        Term: TryInto<T, Error = Term>;
    fn take(&mut self, i: usize) -> Term;
}

impl TupleExt for Tuple {
    fn check_len(&self, n: usize) -> Result<(), DecodeError> {
        if self.elements.len() == n {
            Ok(())
        } else {
            Err(DecodeError::UnexpectedType {
                value: self.clone().into(),
                expected: format!("{} elements tuple", n),
            })
        }
    }

    fn take_as<T>(&mut self, i: usize, expected: &str) -> Result<T, DecodeError>
    where
        Term: TryInto<T, Error = Term>,
    {
        let term = std::mem::replace(&mut self.elements[i], eetf::List::nil().into());
        term.try_into()
            .map_err(|value| DecodeError::UnexpectedType {
                value,
                expected: expected.to_owned(),
            })
    }

    fn take(&mut self, i: usize) -> Term {
        std::mem::replace(&mut self.elements[i], eetf::List::nil().into())
    }
}

trait DistributionMessage: Sized {
    const OP: i32;
    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError>;
    fn read_from<R: Read>(reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    pub from_pid: Pid,
    pub to_pid: Pid,
}

impl DistributionMessage for Link {
    const OP: i32 = 1;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple3(Self::OP, self.from_pid, self.to_pid)?;
        Ok(())
    }

    fn read_from<R: Read>(_reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(3)?;
        let from_pid = ctrl_msg.take_as::<Pid>(1, "Pid")?;
        let to_pid = ctrl_msg.take_as::<Pid>(2, "Pid")?;
        Ok(Self { from_pid, to_pid })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Unlink {
    pub from_pid: Pid,
    pub to_pid: Pid,
}

impl DistributionMessage for Unlink {
    const OP: i32 = 4;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple3(Self::OP, self.from_pid, self.to_pid)?;
        Ok(())
    }

    fn read_from<R: Read>(_reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(3)?;
        let from_pid = ctrl_msg.take_as::<Pid>(1, "Pid")?;
        let to_pid = ctrl_msg.take_as::<Pid>(2, "Pid")?;
        Ok(Self { from_pid, to_pid })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupLeader {
    pub from_pid: Pid,
    pub to_pid: Pid,
}

impl DistributionMessage for GroupLeader {
    const OP: i32 = 7;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple3(Self::OP, self.from_pid, self.to_pid)?;
        Ok(())
    }

    fn read_from<R: Read>(_reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(3)?;
        let from_pid = ctrl_msg.take_as::<Pid>(1, "Pid")?;
        let to_pid = ctrl_msg.take_as::<Pid>(2, "Pid")?;
        Ok(Self { from_pid, to_pid })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeLink {}

impl DistributionMessage for NodeLink {
    const OP: i32 = 5;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple1(Self::OP)?;
        Ok(())
    }

    fn read_from<R: Read>(_reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(1)?;
        Ok(Self {})
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Exit {
    pub from_pid: Pid,
    pub to_pid: Pid,
    pub reason: Term,
}

impl DistributionMessage for Exit {
    const OP: i32 = 3;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple4(Self::OP, self.from_pid, self.to_pid, self.reason)?;
        Ok(())
    }

    fn read_from<R: Read>(_reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(4)?;
        let from_pid = ctrl_msg.take_as::<Pid>(1, "Pid")?;
        let to_pid = ctrl_msg.take_as::<Pid>(2, "Pid")?;
        let reason = ctrl_msg.take(3);
        Ok(Self {
            from_pid,
            to_pid,
            reason,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PayloadExit {
    pub from_pid: Pid,
    pub to_pid: Pid,
    pub reason: Term,
}

impl DistributionMessage for PayloadExit {
    const OP: i32 = 24;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple3(Self::OP, self.from_pid, self.to_pid)?;
        writer.write_term(self.reason)?;
        Ok(())
    }

    fn read_from<R: Read>(reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(3)?;
        let from_pid = ctrl_msg.take_as::<Pid>(1, "Pid")?;
        let to_pid = ctrl_msg.take_as::<Pid>(2, "Pid")?;
        let reason = reader.read_term()?;
        Ok(Self {
            from_pid,
            to_pid,
            reason,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PayloadExitTt {
    pub from_pid: Pid,
    pub to_pid: Pid,
    pub trace_token: Term,
    pub reason: Term,
}

impl DistributionMessage for PayloadExitTt {
    const OP: i32 = 25;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple4(Self::OP, self.from_pid, self.to_pid, self.trace_token)?;
        writer.write_term(self.reason)?;
        Ok(())
    }

    fn read_from<R: Read>(reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(4)?;
        let from_pid = ctrl_msg.take_as::<Pid>(1, "Pid")?;
        let to_pid = ctrl_msg.take_as::<Pid>(2, "Pid")?;
        let trace_token = ctrl_msg.take(3);
        let reason = reader.read_term()?;
        Ok(Self {
            from_pid,
            to_pid,
            trace_token,
            reason,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PayloadExit2 {
    pub from_pid: Pid,
    pub to_pid: Pid,
    pub reason: Term,
}

impl DistributionMessage for PayloadExit2 {
    const OP: i32 = 26;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple3(Self::OP, self.from_pid, self.to_pid)?;
        writer.write_term(self.reason)?;
        Ok(())
    }

    fn read_from<R: Read>(reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(3)?;
        let from_pid = ctrl_msg.take_as::<Pid>(1, "Pid")?;
        let to_pid = ctrl_msg.take_as::<Pid>(2, "Pid")?;
        let reason = reader.read_term()?;
        Ok(Self {
            from_pid,
            to_pid,
            reason,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PayloadExit2Tt {
    pub from_pid: Pid,
    pub to_pid: Pid,
    pub trace_token: Term,
    pub reason: Term,
}

impl DistributionMessage for PayloadExit2Tt {
    const OP: i32 = 27;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple4(Self::OP, self.from_pid, self.to_pid, self.trace_token)?;
        writer.write_term(self.reason)?;
        Ok(())
    }

    fn read_from<R: Read>(reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(4)?;
        let from_pid = ctrl_msg.take_as::<Pid>(1, "Pid")?;
        let to_pid = ctrl_msg.take_as::<Pid>(2, "Pid")?;
        let trace_token = ctrl_msg.take(3);
        let reason = reader.read_term()?;
        Ok(Self {
            from_pid,
            to_pid,
            trace_token,
            reason,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Exit2 {
    pub from_pid: Pid,
    pub to_pid: Pid,
    pub reason: Term,
}

impl DistributionMessage for Exit2 {
    const OP: i32 = 8;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple4(Self::OP, self.from_pid, self.to_pid, self.reason)?;
        Ok(())
    }

    fn read_from<R: Read>(_reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(4)?;
        let from_pid = ctrl_msg.take_as::<Pid>(1, "Pid")?;
        let to_pid = ctrl_msg.take_as::<Pid>(2, "Pid")?;
        let reason = ctrl_msg.take(3);
        Ok(Self {
            from_pid,
            to_pid,
            reason,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExitTt {
    pub from_pid: Pid,
    pub to_pid: Pid,
    pub trace_token: Term,
    pub reason: Term,
}

impl DistributionMessage for ExitTt {
    const OP: i32 = 13;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple5(
            Self::OP,
            self.from_pid,
            self.to_pid,
            self.trace_token,
            self.reason,
        )?;
        Ok(())
    }

    fn read_from<R: Read>(_reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(5)?;
        let from_pid = ctrl_msg.take_as::<Pid>(1, "Pid")?;
        let to_pid = ctrl_msg.take_as::<Pid>(2, "Pid")?;
        let trace_token = ctrl_msg.take(3);
        let reason = ctrl_msg.take(4);
        Ok(Self {
            from_pid,
            to_pid,
            trace_token,
            reason,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Exit2Tt {
    pub from_pid: Pid,
    pub to_pid: Pid,
    pub trace_token: Term,
    pub reason: Term,
}

impl DistributionMessage for Exit2Tt {
    const OP: i32 = 18;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple5(
            Self::OP,
            self.from_pid,
            self.to_pid,
            self.trace_token,
            self.reason,
        )?;
        Ok(())
    }

    fn read_from<R: Read>(_reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(5)?;
        let from_pid = ctrl_msg.take_as::<Pid>(1, "Pid")?;
        let to_pid = ctrl_msg.take_as::<Pid>(2, "Pid")?;
        let trace_token = ctrl_msg.take(3);
        let reason = ctrl_msg.take(4);
        Ok(Self {
            from_pid,
            to_pid,
            trace_token,
            reason,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Send {
    pub to_pid: Pid,
    pub message: Term,
}

impl DistributionMessage for Send {
    const OP: i32 = 2;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple3(Self::OP, Tuple::nil(), self.to_pid)?;
        writer.write_term(self.message)?;
        Ok(())
    }

    fn read_from<R: Read>(reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(3)?;
        let to_pid = ctrl_msg.take_as::<Pid>(2, "Pid")?;
        let message = reader.read_term()?;
        Ok(Self { to_pid, message })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SendTt {
    pub to_pid: Pid,
    pub trace_token: Term,
    pub message: Term,
}

impl DistributionMessage for SendTt {
    const OP: i32 = 12;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple4(Self::OP, Tuple::nil(), self.to_pid, self.trace_token)?;
        writer.write_term(self.message)?;
        Ok(())
    }

    fn read_from<R: Read>(reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(4)?;
        let to_pid = ctrl_msg.take_as::<Pid>(2, "Pid")?;
        let trace_token = ctrl_msg.take(3);
        let message = reader.read_term()?;
        Ok(Self {
            to_pid,
            trace_token,
            message,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SendSender {
    pub from_pid: Pid,
    pub to_pid: Pid,
    pub message: Term,
}

impl DistributionMessage for SendSender {
    const OP: i32 = 22;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple3(Self::OP, self.from_pid, self.to_pid)?;
        writer.write_term(self.message)?;
        Ok(())
    }

    fn read_from<R: Read>(reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(3)?;
        let from_pid = ctrl_msg.take_as::<Pid>(1, "Pid")?;
        let to_pid = ctrl_msg.take_as::<Pid>(2, "Pid")?;
        let message = reader.read_term()?;
        Ok(Self {
            from_pid,
            to_pid,
            message,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SendSenderTt {
    pub from_pid: Pid,
    pub to_pid: Pid,
    pub trace_token: Term,
    pub message: Term,
}

impl DistributionMessage for SendSenderTt {
    const OP: i32 = 23;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple4(Self::OP, self.from_pid, self.to_pid, self.trace_token)?;
        writer.write_term(self.message)?;
        Ok(())
    }

    fn read_from<R: Read>(reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(4)?;
        let from_pid = ctrl_msg.take_as::<Pid>(1, "Pid")?;
        let to_pid = ctrl_msg.take_as::<Pid>(2, "Pid")?;
        let trace_token = ctrl_msg.take(3);
        let message = reader.read_term()?;
        Ok(Self {
            from_pid,
            to_pid,
            trace_token,
            message,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegSend {
    pub from_pid: Pid,
    pub to_name: Atom,
    pub message: Term,
}

impl DistributionMessage for RegSend {
    const OP: i32 = 6;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple4(Self::OP, self.from_pid, Tuple::nil(), self.to_name)?;
        writer.write_term(self.message)?;
        Ok(())
    }

    fn read_from<R: Read>(reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(4)?;
        let from_pid = ctrl_msg.take_as::<Pid>(1, "pid")?;
        let to_name = ctrl_msg.take_as::<Atom>(3, "atom")?;
        let message = reader.read_term()?;
        Ok(Self {
            from_pid,
            to_name,
            message,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegSendTt {
    pub from_pid: Pid,
    pub to_name: Atom,
    pub trace_token: Term,
    pub message: Term,
}

impl DistributionMessage for RegSendTt {
    const OP: i32 = 16;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple5(
            Self::OP,
            self.from_pid,
            Tuple::nil(),
            self.to_name,
            self.trace_token,
        )?;
        writer.write_term(self.message)?;
        Ok(())
    }

    fn read_from<R: Read>(reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(5)?;
        let from_pid = ctrl_msg.take_as::<Pid>(1, "pid")?;
        let to_name = ctrl_msg.take_as::<Atom>(3, "atom")?;
        let trace_token = ctrl_msg.take(4);
        let message = reader.read_term()?;
        Ok(Self {
            from_pid,
            to_name,
            trace_token,
            message,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonitorP {
    pub from_pid: Pid,
    pub to_proc: Term, // TODO: pid or atom
    pub reference: Reference,
}

impl DistributionMessage for MonitorP {
    const OP: i32 = 19;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple4(Self::OP, self.from_pid, self.to_proc, self.reference)?;
        Ok(())
    }

    fn read_from<R: Read>(_reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(4)?;
        let from_pid = ctrl_msg.take_as::<Pid>(1, "pid")?;
        let to_proc = ctrl_msg.take(2);
        let reference = ctrl_msg.take_as::<Reference>(3, "ref")?;
        Ok(Self {
            from_pid,
            to_proc,
            reference,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonitorPExit {
    pub from_pid: Pid,
    pub to_proc: Term, // TODO: pid or atom
    pub reference: Reference,
    pub reason: Term,
}

impl DistributionMessage for MonitorPExit {
    const OP: i32 = 21;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple5(
            Self::OP,
            self.from_pid,
            self.to_proc,
            self.reference,
            self.reason,
        )?;
        Ok(())
    }

    fn read_from<R: Read>(_reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(5)?;
        let from_pid = ctrl_msg.take_as::<Pid>(1, "pid")?;
        let to_proc = ctrl_msg.take(2);
        let reference = ctrl_msg.take_as::<Reference>(3, "ref")?;
        let reason = ctrl_msg.take(4);
        Ok(Self {
            from_pid,
            to_proc,
            reference,
            reason,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PayloadMonitorPExit {
    pub from_pid: Pid,
    pub to_proc: Term, // TODO: pid or atom
    pub reference: Reference,
    pub reason: Term,
}

impl DistributionMessage for PayloadMonitorPExit {
    const OP: i32 = 28;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple4(Self::OP, self.from_pid, self.to_proc, self.reference)?;
        writer.write_term(self.reason)?;
        Ok(())
    }

    fn read_from<R: Read>(reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(4)?;
        let from_pid = ctrl_msg.take_as::<Pid>(1, "pid")?;
        let to_proc = ctrl_msg.take(2);
        let reference = ctrl_msg.take_as::<Reference>(3, "ref")?;
        let reason = reader.read_term()?;
        Ok(Self {
            from_pid,
            to_proc,
            reference,
            reason,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DemonitorP {
    pub from_pid: Pid,
    pub to_proc: Term, // TODO: pid or atom
    pub reference: Reference,
}

impl DistributionMessage for DemonitorP {
    const OP: i32 = 20;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple4(Self::OP, self.from_pid, self.to_proc, self.reference)?;
        Ok(())
    }

    fn read_from<R: Read>(_reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(4)?;
        let from_pid = ctrl_msg.take_as::<Pid>(1, "pid")?;
        let to_proc = ctrl_msg.take(2);
        let reference = ctrl_msg.take_as::<Reference>(3, "ref")?;
        Ok(Self {
            from_pid,
            to_proc,
            reference,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpawnRequest {
    pub req_id: Reference,
    pub from_pid: Pid,
    pub group_leader: Pid,
    pub mfa: Term,      // TODO: {atom(), atom(), integer()}
    pub opt_list: Term, // TODO: [term()],
    pub arg_list: Term, // TODO: [term()]
}

impl DistributionMessage for SpawnRequest {
    const OP: i32 = 29;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple6(
            Self::OP,
            self.req_id,
            self.from_pid,
            self.group_leader,
            self.mfa,
            self.opt_list,
        )?;
        writer.write_term(self.arg_list)?;
        Ok(())
    }

    fn read_from<R: Read>(reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(6)?;
        let req_id = ctrl_msg.take_as::<Reference>(1, "ref")?;
        let from_pid = ctrl_msg.take_as::<Pid>(2, "pid")?;
        let group_leader = ctrl_msg.take_as::<Pid>(3, "pid")?;
        let mfa = ctrl_msg.take(4);
        let opt_list = ctrl_msg.take(5);
        let arg_list = reader.read_term()?;
        Ok(Self {
            req_id,
            from_pid,
            group_leader,
            mfa,
            opt_list,
            arg_list,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpawnRequestTt {
    pub req_id: Reference,
    pub from_pid: Pid,
    pub group_leader: Pid,
    pub mfa: Term,      // TODO: {atom(), atom(), integer()}
    pub opt_list: Term, // TODO: [term()],
    pub trace_token: Term,
    pub arg_list: Term, // TODO: [term()]
}

impl DistributionMessage for SpawnRequestTt {
    const OP: i32 = 30;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple7(
            Self::OP,
            self.req_id,
            self.from_pid,
            self.group_leader,
            self.mfa,
            self.opt_list,
            self.trace_token,
        )?;
        writer.write_term(self.arg_list)?;
        Ok(())
    }

    fn read_from<R: Read>(reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(7)?;
        let req_id = ctrl_msg.take_as::<Reference>(1, "ref")?;
        let from_pid = ctrl_msg.take_as::<Pid>(2, "pid")?;
        let group_leader = ctrl_msg.take_as::<Pid>(3, "pid")?;
        let mfa = ctrl_msg.take(4);
        let opt_list = ctrl_msg.take(5);
        let trace_token = ctrl_msg.take(6);
        let arg_list = reader.read_term()?;
        Ok(Self {
            req_id,
            from_pid,
            group_leader,
            mfa,
            opt_list,
            trace_token,
            arg_list,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpawnReply {
    pub req_id: Reference,
    pub to_pid: Pid,
    pub flags: FixInteger,
    pub result: Term, // TODO: pid() or atom()
}

impl DistributionMessage for SpawnReply {
    const OP: i32 = 31;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple5(Self::OP, self.req_id, self.to_pid, self.flags, self.result)?;
        Ok(())
    }

    fn read_from<R: Read>(_reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(5)?;
        let req_id = ctrl_msg.take_as::<Reference>(1, "ref")?;
        let to_pid = ctrl_msg.take_as::<Pid>(2, "pid")?;
        let flags = ctrl_msg.take_as::<FixInteger>(3, "integer")?;
        let result = ctrl_msg.take(4);
        Ok(Self {
            req_id,
            to_pid,
            flags,
            result,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpawnReplyTt {
    pub req_id: Reference,
    pub to_pid: Pid,
    pub flags: FixInteger,
    pub result: Term, // TODO: pid() or atom()
    pub trace_token: Term,
}

impl DistributionMessage for SpawnReplyTt {
    const OP: i32 = 32;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple6(
            Self::OP,
            self.req_id,
            self.to_pid,
            self.flags,
            self.result,
            self.trace_token,
        )?;
        Ok(())
    }

    fn read_from<R: Read>(_reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(6)?;
        let req_id = ctrl_msg.take_as::<Reference>(1, "ref")?;
        let to_pid = ctrl_msg.take_as::<Pid>(2, "pid")?;
        let flags = ctrl_msg.take_as::<FixInteger>(3, "integer")?;
        let result = ctrl_msg.take(4);
        let trace_token = ctrl_msg.take(5);
        Ok(Self {
            req_id,
            to_pid,
            flags,
            result,
            trace_token,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnlinkId {
    pub id: Term,
    pub from_pid: Pid,
    pub to_pid: Pid,
}

impl DistributionMessage for UnlinkId {
    const OP: i32 = 35;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple4(Self::OP, self.id, self.from_pid, self.to_pid)?;
        Ok(())
    }

    fn read_from<R: Read>(_reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(4)?;
        let id = ctrl_msg.take(1);
        let from_pid = ctrl_msg.take_as::<Pid>(2, "Pid")?;
        let to_pid = ctrl_msg.take_as::<Pid>(3, "Pid")?;
        Ok(Self {
            id,
            from_pid,
            to_pid,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnlinkIdAck {
    pub id: Term,
    pub from_pid: Pid,
    pub to_pid: Pid,
}

impl DistributionMessage for UnlinkIdAck {
    const OP: i32 = 36;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple4(Self::OP, self.id, self.from_pid, self.to_pid)?;
        Ok(())
    }

    fn read_from<R: Read>(_reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(4)?;
        let id = ctrl_msg.take(1);
        let from_pid = ctrl_msg.take_as::<Pid>(2, "Pid")?;
        let to_pid = ctrl_msg.take_as::<Pid>(3, "Pid")?;
        Ok(Self {
            id,
            from_pid,
            to_pid,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AliasSend {
    pub from_pid: Pid,
    pub alias: Term, // Reference(?)
    pub message: Term,
}

impl DistributionMessage for AliasSend {
    const OP: i32 = 33;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple3(Self::OP, self.from_pid, self.alias)?;
        writer.write_term(self.message)?;
        Ok(())
    }

    fn read_from<R: Read>(reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(3)?;
        let from_pid = ctrl_msg.take_as::<Pid>(1, "Pid")?;
        let alias = ctrl_msg.take(2);
        let message = reader.read_term()?;
        Ok(Self {
            from_pid,
            alias,
            message,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AliasSendTt {
    pub from_pid: Pid,
    pub alias: Term, // Reference(?)
    pub trace_token: Term,
    pub message: Term,
}

impl DistributionMessage for AliasSendTt {
    const OP: i32 = 34;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple4(Self::OP, self.from_pid, self.alias, self.trace_token)?;
        writer.write_term(self.message)?;
        Ok(())
    }

    fn read_from<R: Read>(reader: &mut R, mut ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        ctrl_msg.check_len(4)?;
        let from_pid = ctrl_msg.take_as::<Pid>(1, "Pid")?;
        let alias = ctrl_msg.take(2);
        let trace_token = ctrl_msg.take(3);
        let message = reader.read_term()?;
        Ok(Self {
            from_pid,
            alias,
            trace_token,
            message,
        })
    }
}

/// Message.
///
/// This provides various message construction functions.
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Link(Link),
    Send(Send),
    Exit(Exit),
    Unlink(Unlink), // deprecated
    NodeLink(NodeLink),
    RegSend(RegSend),
    GroupLeader(GroupLeader),
    Exit2(Exit2),
    SendTt(SendTt),
    ExitTt(ExitTt),
    RegSendTt(RegSendTt),
    Exit2Tt(Exit2Tt),
    MonitorP(MonitorP),
    DemonitorP(DemonitorP),
    MonitorPExit(MonitorPExit),
    SendSender(SendSender),
    SendSenderTt(SendSenderTt),
    PayloadExit(PayloadExit),
    PayloadExitTt(PayloadExitTt),
    PayloadExit2(PayloadExit2),
    PayloadExit2Tt(PayloadExit2Tt),
    PayloadMonitorPExit(PayloadMonitorPExit),
    SpawnRequest(SpawnRequest),
    SpawnRequestTt(SpawnRequestTt),
    SpawnReply(SpawnReply),
    SpawnReplyTt(SpawnReplyTt),
    UnlinkId(UnlinkId),
    UnlinkIdAck(UnlinkIdAck),
    AliasSend(AliasSend),
    AliasSendTt(AliasSendTt),
}

impl Message {
    // TODO: pub into_compatible(self, flags) -> Result<Self>;

    pub fn send(to_pid: Pid, message: Term) -> Self {
        Self::Send(Send { to_pid, message })
    }

    pub fn reg_send(from_pid: Pid, to_name: Atom, message: Term) -> Self {
        Self::RegSend(RegSend {
            from_pid,
            to_name,
            message,
        })
    }

    pub fn write_into<W: Write>(self, writer: &mut W) -> Result<(), crate::channel::SendError> {
        match self {
            Self::Link(x) => x.write_into(writer)?,
            Self::Send(x) => x.write_into(writer)?,
            Self::Exit(x) => x.write_into(writer)?,
            Self::Unlink(x) => x.write_into(writer)?,
            Self::NodeLink(x) => x.write_into(writer)?,
            Self::RegSend(x) => x.write_into(writer)?,
            Self::GroupLeader(x) => x.write_into(writer)?,
            Self::Exit2(x) => x.write_into(writer)?,
            Self::SendTt(x) => x.write_into(writer)?,
            Self::ExitTt(x) => x.write_into(writer)?,
            Self::RegSendTt(x) => x.write_into(writer)?,
            Self::Exit2Tt(x) => x.write_into(writer)?,
            Self::MonitorP(x) => x.write_into(writer)?,
            Self::DemonitorP(x) => x.write_into(writer)?,
            Self::MonitorPExit(x) => x.write_into(writer)?,
            Self::SendSender(x) => x.write_into(writer)?,
            Self::SendSenderTt(x) => x.write_into(writer)?,
            Self::PayloadExit(x) => x.write_into(writer)?,
            Self::PayloadExitTt(x) => x.write_into(writer)?,
            Self::PayloadExit2(x) => x.write_into(writer)?,
            Self::PayloadExit2Tt(x) => x.write_into(writer)?,
            Self::PayloadMonitorPExit(x) => x.write_into(writer)?,
            Self::SpawnRequest(x) => x.write_into(writer)?,
            Self::SpawnRequestTt(x) => x.write_into(writer)?,
            Self::SpawnReply(x) => x.write_into(writer)?,
            Self::SpawnReplyTt(x) => x.write_into(writer)?,
            Self::UnlinkId(x) => x.write_into(writer)?,
            Self::UnlinkIdAck(x) => x.write_into(writer)?,
            Self::AliasSend(x) => x.write_into(writer)?,
            Self::AliasSendTt(x) => x.write_into(writer)?,
        }
        Ok(())
    }

    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self, crate::channel::RecvError> {
        let mut ctrl_msg = reader.read_tuple()?;
        if ctrl_msg.elements.is_empty() {
            return Err(DecodeError::UnexpectedType {
                value: ctrl_msg.into(),
                expected: "non empty tuple".to_owned(),
            }
            .into());
        }
        let op = ctrl_msg.take_as::<FixInteger>(0, "integer")?;
        let msg = match op.value {
            Link::OP => Link::read_from(reader, ctrl_msg).map(Self::Link)?,
            Send::OP => Send::read_from(reader, ctrl_msg).map(Self::Send)?,
            Exit::OP => Exit::read_from(reader, ctrl_msg).map(Self::Exit)?,
            Unlink::OP => Unlink::read_from(reader, ctrl_msg).map(Self::Unlink)?,
            NodeLink::OP => NodeLink::read_from(reader, ctrl_msg).map(Self::NodeLink)?,
            RegSend::OP => RegSend::read_from(reader, ctrl_msg).map(Self::RegSend)?,
            GroupLeader::OP => GroupLeader::read_from(reader, ctrl_msg).map(Self::GroupLeader)?,
            Exit2::OP => Exit2::read_from(reader, ctrl_msg).map(Self::Exit2)?,
            SendTt::OP => SendTt::read_from(reader, ctrl_msg).map(Self::SendTt)?,
            ExitTt::OP => ExitTt::read_from(reader, ctrl_msg).map(Self::ExitTt)?,
            RegSendTt::OP => RegSendTt::read_from(reader, ctrl_msg).map(Self::RegSendTt)?,
            Exit2Tt::OP => Exit2Tt::read_from(reader, ctrl_msg).map(Self::Exit2Tt)?,
            MonitorP::OP => MonitorP::read_from(reader, ctrl_msg).map(Self::MonitorP)?,
            DemonitorP::OP => DemonitorP::read_from(reader, ctrl_msg).map(Self::DemonitorP)?,
            MonitorPExit::OP => {
                MonitorPExit::read_from(reader, ctrl_msg).map(Self::MonitorPExit)?
            }
            SendSender::OP => SendSender::read_from(reader, ctrl_msg).map(Self::SendSender)?,
            SendSenderTt::OP => {
                SendSenderTt::read_from(reader, ctrl_msg).map(Self::SendSenderTt)?
            }
            PayloadExit::OP => PayloadExit::read_from(reader, ctrl_msg).map(Self::PayloadExit)?,
            PayloadExitTt::OP => {
                PayloadExitTt::read_from(reader, ctrl_msg).map(Self::PayloadExitTt)?
            }
            PayloadExit2::OP => {
                PayloadExit2::read_from(reader, ctrl_msg).map(Self::PayloadExit2)?
            }
            PayloadExit2Tt::OP => {
                PayloadExit2Tt::read_from(reader, ctrl_msg).map(Self::PayloadExit2Tt)?
            }
            PayloadMonitorPExit::OP => {
                PayloadMonitorPExit::read_from(reader, ctrl_msg).map(Self::PayloadMonitorPExit)?
            }
            SpawnRequest::OP => {
                SpawnRequest::read_from(reader, ctrl_msg).map(Self::SpawnRequest)?
            }
            SpawnRequestTt::OP => {
                SpawnRequestTt::read_from(reader, ctrl_msg).map(Self::SpawnRequestTt)?
            }
            SpawnReply::OP => SpawnReply::read_from(reader, ctrl_msg).map(Self::SpawnReply)?,
            SpawnReplyTt::OP => {
                SpawnReplyTt::read_from(reader, ctrl_msg).map(Self::SpawnReplyTt)?
            }
            UnlinkId::OP => UnlinkId::read_from(reader, ctrl_msg).map(Self::UnlinkId)?,
            UnlinkIdAck::OP => UnlinkIdAck::read_from(reader, ctrl_msg).map(Self::UnlinkIdAck)?,
            AliasSend::OP => AliasSend::read_from(reader, ctrl_msg).map(Self::AliasSend)?,
            AliasSendTt::OP => AliasSendTt::read_from(reader, ctrl_msg).map(Self::AliasSendTt)?,
            op => return Err(crate::channel::RecvError::UnsupportedOp { op }),
        };
        Ok(msg)
    }
}
