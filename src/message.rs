//! Messages passed between distributed nodes.
//!
//! Reference: [12.3 Protocol between Connected Nodes]
//! (http://erlang.org/doc/apps/erts/erl_dist_protocol.html#id105440).
//!
//! Note that distribution headers are not supported in the current version.
use eetf::convert::{TryAsRef, TryInto};
use eetf::{Atom, FixInteger, Pid, Reference, Term, Tuple};
use std::io::{self, Error, ErrorKind, Read, Write};

const CTRL_TYPE_LINK: u8 = 1;
const CTRL_TYPE_SEND: u8 = 2;
const CTRL_TYPE_EXIT: u8 = 3;
const CTRL_TYPE_UNLINK: u8 = 4;
const CTRL_TYPE_NODE_LINK: u8 = 5;
const CTRL_TYPE_REG_SEND: u8 = 6;
const CTRL_TYPE_GROUP_LEADER: u8 = 7;
const CTRL_TYPE_EXIT2: u8 = 8;
const CTRL_TYPE_SEND_TT: u8 = 12;
const CTRL_TYPE_EXIT_TT: u8 = 13;
const CTRL_TYPE_REG_SEND_TT: u8 = 16;
const CTRL_TYPE_EXIT2_TT: u8 = 18;
const CTRL_TYPE_MONITOR_P: u8 = 19;
const CTRL_TYPE_DEMONITOR_P: u8 = 20;
const CTRL_TYPE_MONITOR_P_EXIT: u8 = 21;

const TAG_PASS_THROUGH: u8 = 112;

/// Message.
///
/// This provides various message construction functions.
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Link(Link),
    Send(Send),
    Exit(Exit),
    Unlink(Unlink),
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
    Heartbeat(Heartbeat),
}
impl Message {
    /// Reads a `Message` from `reader`.
    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut buf = [0];
        if let Err(e) = reader.read_exact(&mut buf) {
            if e.kind() == ErrorKind::UnexpectedEof {
                Ok(Message::heartbeat())
            } else {
                Err(e)
            }
        } else if buf[0] != TAG_PASS_THROUGH {
            Err(invalid_data!(
                "Message tag {} is currently unsupported",
                buf[0]
            ))
        } else {
            let ctrl: Tuple = read_term(reader)?;
            let tag = {
                let tag: &FixInteger = ctrl.elements[0].try_as_ref().unwrap();
                tag.value as u8
            };
            Ok(match tag {
                CTRL_TYPE_LINK => Message::from(Link::read_from(ctrl, reader)?),
                CTRL_TYPE_SEND => Message::from(Send::read_from(ctrl, reader)?),
                CTRL_TYPE_EXIT => Message::from(Exit::read_from(ctrl, reader)?),
                CTRL_TYPE_UNLINK => Message::from(Unlink::read_from(ctrl, reader)?),
                CTRL_TYPE_NODE_LINK => Message::from(NodeLink::read_from(ctrl, reader)?),
                CTRL_TYPE_REG_SEND => Message::from(RegSend::read_from(ctrl, reader)?),
                CTRL_TYPE_GROUP_LEADER => Message::from(GroupLeader::read_from(ctrl, reader)?),
                CTRL_TYPE_EXIT2 => Message::from(Exit2::read_from(ctrl, reader)?),
                CTRL_TYPE_SEND_TT => Message::from(SendTt::read_from(ctrl, reader)?),
                CTRL_TYPE_EXIT_TT => Message::from(ExitTt::read_from(ctrl, reader)?),
                CTRL_TYPE_REG_SEND_TT => Message::from(RegSendTt::read_from(ctrl, reader)?),
                CTRL_TYPE_EXIT2_TT => Message::from(Exit2Tt::read_from(ctrl, reader)?),
                CTRL_TYPE_MONITOR_P => Message::from(MonitorP::read_from(ctrl, reader)?),
                CTRL_TYPE_DEMONITOR_P => Message::from(DemonitorP::read_from(ctrl, reader)?),
                CTRL_TYPE_MONITOR_P_EXIT => Message::from(MonitorPExit::read_from(ctrl, reader)?),
                _ => {
                    return Err(invalid_data!("Unknown control message: type={}", tag));
                }
            })
        }
    }

    /// Writes this `Message` into `writer`.
    pub fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
        if let Message::Heartbeat(_) = self {
            Ok(())
        } else {
            writer.write_all(&[TAG_PASS_THROUGH])?;
            match self {
                Message::Link(x) => x.write_into(writer),
                Message::Send(x) => x.write_into(writer),
                Message::Exit(x) => x.write_into(writer),
                Message::Unlink(x) => x.write_into(writer),
                Message::NodeLink(x) => x.write_into(writer),
                Message::RegSend(x) => x.write_into(writer),
                Message::GroupLeader(x) => x.write_into(writer),
                Message::Exit2(x) => x.write_into(writer),
                Message::SendTt(x) => x.write_into(writer),
                Message::ExitTt(x) => x.write_into(writer),
                Message::RegSendTt(x) => x.write_into(writer),
                Message::Exit2Tt(x) => x.write_into(writer),
                Message::MonitorP(x) => x.write_into(writer),
                Message::DemonitorP(x) => x.write_into(writer),
                Message::MonitorPExit(x) => x.write_into(writer),
                Message::Heartbeat(_) => unreachable!(),
            }
        }
    }

    /// Makes a new `Heartbeat` message.
    pub fn heartbeat() -> Self {
        Message::from(Heartbeat)
    }

    /// Makes a new `Link` message.
    pub fn link(from_pid: Pid, to_pid: Pid) -> Self {
        Message::from(Link {
            from_pid: from_pid,
            to_pid: to_pid,
        })
    }

    /// Makes a new `Send` message.
    pub fn send<T>(to_pid: Pid, message: T) -> Self
    where
        Term: From<T>,
    {
        Message::from(Send {
            to_pid: to_pid,
            message: Term::from(message),
        })
    }

    /// Makes a new `Exit` message.
    pub fn exit<T>(from_pid: Pid, to_pid: Pid, reason: T) -> Self
    where
        Term: From<T>,
    {
        Message::from(Exit {
            from_pid: from_pid,
            to_pid: to_pid,
            reason: Term::from(reason),
        })
    }

    /// Makes a new `Unlink` message.
    pub fn unlink(from_pid: Pid, to_pid: Pid) -> Self {
        Message::from(Unlink {
            from_pid: from_pid,
            to_pid: to_pid,
        })
    }

    /// Makes a new `NodeLink` message.
    pub fn node_link() -> Self {
        Message::from(NodeLink)
    }

    /// Makes a new `RegSend` message.
    pub fn reg_send<A, T>(from_pid: Pid, to_name: A, message: T) -> Self
    where
        Atom: From<A>,
        Term: From<T>,
    {
        Message::from(RegSend {
            from_pid: from_pid,
            to_name: Atom::from(to_name),
            message: Term::from(message),
        })
    }

    /// Makes a new `GroupLeader` message.
    pub fn group_leader(from_pid: Pid, to_pid: Pid) -> Self {
        Message::from(GroupLeader {
            from_pid: from_pid,
            to_pid: to_pid,
        })
    }

    /// Makes a new `Exit2` message.
    pub fn exit2<T>(from_pid: Pid, to_pid: Pid, reason: T) -> Self
    where
        Term: From<T>,
    {
        Message::from(Exit2 {
            from_pid: from_pid,
            to_pid: to_pid,
            reason: Term::from(reason),
        })
    }

    /// Makes a new `SendTt` message.
    pub fn send_tt<T, U>(to_pid: Pid, trace_token: T, message: U) -> Self
    where
        Term: From<T>,
        Term: From<U>,
    {
        Message::from(SendTt {
            to_pid: to_pid,
            trace_token: Term::from(trace_token),
            message: Term::from(message),
        })
    }

    /// Makes a new `ExitTt` message.
    pub fn exit_tt<T, U>(from_pid: Pid, to_pid: Pid, trace_token: T, reason: U) -> Self
    where
        Term: From<T>,
        Term: From<U>,
    {
        Message::from(ExitTt {
            from_pid: from_pid,
            to_pid: to_pid,
            trace_token: Term::from(trace_token),
            reason: Term::from(reason),
        })
    }

    /// Makes a new `RegSendTt` message.
    pub fn reg_send_tt<A, T, U>(from_pid: Pid, to_name: A, trace_token: T, message: U) -> Self
    where
        Atom: From<A>,
        Term: From<T>,
        Term: From<U>,
    {
        Message::from(RegSendTt {
            from_pid: from_pid,
            to_name: Atom::from(to_name),
            trace_token: Term::from(trace_token),
            message: Term::from(message),
        })
    }

    /// Makes a new `Exit2Tt` message.
    pub fn exit2_tt<T, U>(from_pid: Pid, to_pid: Pid, trace_token: T, reason: U) -> Self
    where
        Term: From<T>,
        Term: From<U>,
    {
        Message::from(Exit2Tt {
            from_pid: from_pid,
            to_pid: to_pid,
            trace_token: Term::from(trace_token),
            reason: Term::from(reason),
        })
    }

    /// Makes a new `MonitorP` message.
    pub fn monitor_p<T>(from_pid: Pid, to_proc: T, reference: Reference) -> Self
    where
        ProcessRef: From<T>,
    {
        Message::from(MonitorP {
            from_pid: from_pid,
            to_proc: From::from(to_proc),
            reference: reference,
        })
    }

    /// Makes a new `DemonitorP` message.
    pub fn demonitor_p<T>(from_pid: Pid, to_proc: T, reference: Reference) -> Self
    where
        ProcessRef: From<T>,
    {
        Message::from(DemonitorP {
            from_pid: from_pid,
            to_proc: From::from(to_proc),
            reference: reference,
        })
    }

    /// Makes a new `MonitorPExit` message.
    pub fn monitor_p_exit<T>(from_pid: Pid, to_pid: Pid, reference: Reference, reason: T) -> Self
    where
        Term: From<T>,
    {
        Message::from(MonitorPExit {
            from_pid: from_pid,
            to_pid: to_pid,
            reference: reference,
            reason: Term::from(reason),
        })
    }
}
macro_rules! impl_from_for_message {
    ($t:ident) => {
        impl From<$t> for Message {
            fn from(f: $t) -> Self {
                Message::$t(f)
            }
        }
    };
}
impl_from_for_message!(Link);
impl_from_for_message!(Send);
impl_from_for_message!(Exit);
impl_from_for_message!(Unlink);
impl_from_for_message!(NodeLink);
impl_from_for_message!(RegSend);
impl_from_for_message!(GroupLeader);
impl_from_for_message!(Exit2);
impl_from_for_message!(SendTt);
impl_from_for_message!(ExitTt);
impl_from_for_message!(RegSendTt);
impl_from_for_message!(Exit2Tt);
impl_from_for_message!(MonitorP);
impl_from_for_message!(DemonitorP);
impl_from_for_message!(MonitorPExit);
impl_from_for_message!(Heartbeat);

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    pub from_pid: Pid,
    pub to_pid: Pid,
}
impl Link {
    fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
        let ctrl = tagged_tuple3(CTRL_TYPE_LINK, self.from_pid, self.to_pid);
        write_term(writer, ctrl)
    }
    fn read_from<R: Read>(ctrl: Tuple, _reader: &mut R) -> io::Result<Self> {
        let from_pid: &Pid = ctrl.elements[1].try_as_ref().unwrap();
        let to_pid: &Pid = ctrl.elements[2].try_as_ref().unwrap();
        Ok(Link {
            from_pid: from_pid.clone(),
            to_pid: to_pid.clone(),
        })
    }
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq)]
pub struct Send {
    pub to_pid: Pid,
    pub message: Term,
}
impl Send {
    fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
        let ctrl = tagged_tuple3(CTRL_TYPE_SEND, Tuple::nil(), self.to_pid);
        write_term(writer, ctrl)?;
        write_term(writer, self.message)?;
        Ok(())
    }
    fn read_from<R: Read>(ctrl: Tuple, reader: &mut R) -> io::Result<Self> {
        let to_pid: &Pid = ctrl.elements[2].try_as_ref().unwrap();
        let message = read_term(reader)?;
        Ok(Send {
            to_pid: to_pid.clone(),
            message: message,
        })
    }
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq)]
pub struct Exit {
    pub from_pid: Pid,
    pub to_pid: Pid,
    pub reason: Term,
}
impl Exit {
    fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
        let ctrl = tagged_tuple4(CTRL_TYPE_EXIT, self.from_pid, self.to_pid, self.reason);
        write_term(writer, ctrl)
    }
    fn read_from<R: Read>(ctrl: Tuple, _reader: &mut R) -> io::Result<Self> {
        let from_pid: &Pid = ctrl.elements[1].try_as_ref().unwrap();
        let to_pid: &Pid = ctrl.elements[2].try_as_ref().unwrap();
        let reason = ctrl.elements[3].clone();
        Ok(Exit {
            from_pid: from_pid.clone(),
            to_pid: to_pid.clone(),
            reason: reason,
        })
    }
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq)]
pub struct Unlink {
    pub from_pid: Pid,
    pub to_pid: Pid,
}
impl Unlink {
    fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
        let ctrl = tagged_tuple3(CTRL_TYPE_UNLINK, self.from_pid, self.to_pid);
        write_term(writer, ctrl)
    }
    fn read_from<R: Read>(ctrl: Tuple, _reader: &mut R) -> io::Result<Self> {
        let from_pid: &Pid = ctrl.elements[1].try_as_ref().unwrap();
        let to_pid: &Pid = ctrl.elements[2].try_as_ref().unwrap();
        Ok(Unlink {
            from_pid: from_pid.clone(),
            to_pid: to_pid.clone(),
        })
    }
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq)]
pub struct NodeLink;
impl NodeLink {
    fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
        let ctrl = tagged_tuple1(CTRL_TYPE_NODE_LINK);
        write_term(writer, ctrl)
    }
    fn read_from<R: Read>(_ctrl: Tuple, _reader: &mut R) -> io::Result<Self> {
        Ok(NodeLink)
    }
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq)]
pub struct RegSend {
    pub from_pid: Pid,
    pub to_name: Atom,
    pub message: Term,
}
impl RegSend {
    fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
        let ctrl = tagged_tuple4(
            CTRL_TYPE_REG_SEND,
            self.from_pid,
            Tuple::nil(),
            self.to_name,
        );
        write_term(writer, ctrl)?;
        write_term(writer, self.message)?;
        Ok(())
    }
    fn read_from<R: Read>(ctrl: Tuple, reader: &mut R) -> io::Result<Self> {
        let from_pid: &Pid = ctrl.elements[1].try_as_ref().unwrap();
        let to_name: &Atom = ctrl.elements[3].try_as_ref().unwrap();
        let message = read_term(reader)?;
        Ok(RegSend {
            from_pid: from_pid.clone(),
            to_name: to_name.clone(),
            message: message,
        })
    }
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq)]
pub struct GroupLeader {
    pub from_pid: Pid,
    pub to_pid: Pid,
}
impl GroupLeader {
    fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
        let ctrl = tagged_tuple3(CTRL_TYPE_GROUP_LEADER, self.from_pid, self.to_pid);
        write_term(writer, ctrl)
    }
    fn read_from<R: Read>(ctrl: Tuple, _reader: &mut R) -> io::Result<Self> {
        let from_pid: &Pid = ctrl.elements[1].try_as_ref().unwrap();
        let to_pid: &Pid = ctrl.elements[2].try_as_ref().unwrap();
        Ok(GroupLeader {
            from_pid: from_pid.clone(),
            to_pid: to_pid.clone(),
        })
    }
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq)]
pub struct Exit2 {
    pub from_pid: Pid,
    pub to_pid: Pid,
    pub reason: Term,
}
impl Exit2 {
    fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
        let ctrl = tagged_tuple4(CTRL_TYPE_EXIT2, self.from_pid, self.to_pid, self.reason);
        write_term(writer, ctrl)
    }
    fn read_from<R: Read>(ctrl: Tuple, _reader: &mut R) -> io::Result<Self> {
        let from_pid: &Pid = ctrl.elements[1].try_as_ref().unwrap();
        let to_pid: &Pid = ctrl.elements[2].try_as_ref().unwrap();
        let reason = ctrl.elements[3].clone();
        Ok(Exit2 {
            from_pid: from_pid.clone(),
            to_pid: to_pid.clone(),
            reason: reason,
        })
    }
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq)]
pub struct SendTt {
    pub to_pid: Pid,
    pub trace_token: Term,
    pub message: Term,
}
impl SendTt {
    fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
        let ctrl = tagged_tuple4(
            CTRL_TYPE_SEND_TT,
            Tuple::nil(),
            self.to_pid,
            self.trace_token,
        );
        write_term(writer, ctrl)?;
        write_term(writer, self.message)?;
        Ok(())
    }
    fn read_from<R: Read>(ctrl: Tuple, reader: &mut R) -> io::Result<Self> {
        let to_pid: &Pid = ctrl.elements[2].try_as_ref().unwrap();
        let trace_token = ctrl.elements[3].clone();
        let message = read_term(reader)?;
        Ok(SendTt {
            to_pid: to_pid.clone(),
            trace_token: trace_token,
            message: message,
        })
    }
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq)]
pub struct ExitTt {
    pub from_pid: Pid,
    pub to_pid: Pid,
    pub trace_token: Term,
    pub reason: Term,
}
impl ExitTt {
    fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
        let ctrl = tagged_tuple5(
            CTRL_TYPE_EXIT_TT,
            self.from_pid,
            self.to_pid,
            self.trace_token,
            self.reason,
        );
        write_term(writer, ctrl)
    }
    fn read_from<R: Read>(ctrl: Tuple, _reader: &mut R) -> io::Result<Self> {
        let from_pid: &Pid = ctrl.elements[1].try_as_ref().unwrap();
        let to_pid: &Pid = ctrl.elements[2].try_as_ref().unwrap();
        let trace_token = ctrl.elements[3].clone();
        let reason = ctrl.elements[4].clone();
        Ok(ExitTt {
            from_pid: from_pid.clone(),
            to_pid: to_pid.clone(),
            trace_token: trace_token,
            reason: reason,
        })
    }
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq)]
pub struct RegSendTt {
    pub from_pid: Pid,
    pub to_name: Atom,
    pub trace_token: Term,
    pub message: Term,
}
impl RegSendTt {
    fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
        let ctrl = tagged_tuple5(
            CTRL_TYPE_REG_SEND_TT,
            self.from_pid,
            Tuple::nil(),
            self.to_name,
            self.trace_token,
        );
        write_term(writer, ctrl)?;
        write_term(writer, self.message)?;
        Ok(())
    }
    fn read_from<R: Read>(ctrl: Tuple, reader: &mut R) -> io::Result<Self> {
        let from_pid: &Pid = ctrl.elements[1].try_as_ref().unwrap();
        let to_name: &Atom = ctrl.elements[3].try_as_ref().unwrap();
        let trace_token = ctrl.elements[4].clone();
        let message = read_term(reader)?;
        Ok(RegSendTt {
            from_pid: from_pid.clone(),
            to_name: to_name.clone(),
            trace_token: trace_token,
            message: message,
        })
    }
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq)]
pub struct Exit2Tt {
    pub from_pid: Pid,
    pub to_pid: Pid,
    pub trace_token: Term,
    pub reason: Term,
}
impl Exit2Tt {
    fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
        let ctrl = tagged_tuple5(
            CTRL_TYPE_EXIT2_TT,
            self.from_pid,
            self.to_pid,
            self.trace_token,
            self.reason,
        );
        write_term(writer, ctrl)
    }
    fn read_from<R: Read>(ctrl: Tuple, _reader: &mut R) -> io::Result<Self> {
        let from_pid: &Pid = ctrl.elements[1].try_as_ref().unwrap();
        let to_pid: &Pid = ctrl.elements[2].try_as_ref().unwrap();
        let trace_token = ctrl.elements[3].clone();
        let reason = ctrl.elements[4].clone();
        Ok(Exit2Tt {
            from_pid: from_pid.clone(),
            to_pid: to_pid.clone(),
            trace_token: trace_token,
            reason: reason,
        })
    }
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq)]
pub struct MonitorP {
    pub from_pid: Pid,
    pub to_proc: ProcessRef,
    pub reference: Reference,
}
impl MonitorP {
    fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
        let ctrl = tagged_tuple4(
            CTRL_TYPE_MONITOR_P,
            self.from_pid,
            self.to_proc,
            self.reference,
        );
        write_term(writer, ctrl)
    }
    fn read_from<R: Read>(ctrl: Tuple, _reader: &mut R) -> io::Result<Self> {
        let from_pid: &Pid = ctrl.elements[1].try_as_ref().unwrap();
        let to_proc = if let Some(x) = ctrl.elements[2].try_as_ref() as Option<&Pid> {
            ProcessRef::Pid(x.clone())
        } else if let Some(x) = ctrl.elements[2].try_as_ref() as Option<&Atom> {
            ProcessRef::Name(x.clone())
        } else {
            panic!()
        };
        let reference: &Reference = ctrl.elements[3].try_as_ref().unwrap();
        Ok(MonitorP {
            from_pid: from_pid.clone(),
            to_proc: to_proc,
            reference: reference.clone(),
        })
    }
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq)]
pub struct DemonitorP {
    pub from_pid: Pid,
    pub to_proc: ProcessRef,
    pub reference: Reference,
}
impl DemonitorP {
    fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
        let ctrl = tagged_tuple4(
            CTRL_TYPE_DEMONITOR_P,
            self.from_pid,
            self.to_proc,
            self.reference,
        );
        write_term(writer, ctrl)
    }
    fn read_from<R: Read>(ctrl: Tuple, _reader: &mut R) -> io::Result<Self> {
        let from_pid: &Pid = ctrl.elements[1].try_as_ref().unwrap();
        let to_proc = if let Some(x) = ctrl.elements[2].try_as_ref() as Option<&Pid> {
            ProcessRef::Pid(x.clone())
        } else if let Some(x) = ctrl.elements[2].try_as_ref() as Option<&Atom> {
            ProcessRef::Name(x.clone())
        } else {
            panic!()
        };
        let reference: &Reference = ctrl.elements[3].try_as_ref().unwrap();
        Ok(DemonitorP {
            from_pid: from_pid.clone(),
            to_proc: to_proc,
            reference: reference.clone(),
        })
    }
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq)]
pub struct MonitorPExit {
    pub from_pid: Pid,
    pub to_pid: Pid,
    pub reference: Reference,
    pub reason: Term,
}
impl MonitorPExit {
    fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
        let ctrl = tagged_tuple5(
            CTRL_TYPE_DEMONITOR_P,
            self.from_pid,
            self.to_pid,
            self.reference,
            self.reason,
        );
        write_term(writer, ctrl)
    }
    fn read_from<R: Read>(ctrl: Tuple, _reader: &mut R) -> io::Result<Self> {
        let from_pid: &Pid = ctrl.elements[1].try_as_ref().unwrap();
        let to_pid: &Pid = ctrl.elements[2].try_as_ref().unwrap();
        let reference: &Reference = ctrl.elements[3].try_as_ref().unwrap();
        let reason = ctrl.elements[4].clone();
        Ok(MonitorPExit {
            from_pid: from_pid.clone(),
            to_pid: to_pid.clone(),
            reference: reference.clone(),
            reason: reason,
        })
    }
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq)]
pub struct Heartbeat;

/// Identifier or name of a process.
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessRef {
    /// Identifier of the process.
    Pid(Pid),

    /// Name of the process.
    Name(Atom),
}
impl From<Pid> for ProcessRef {
    fn from(f: Pid) -> Self {
        ProcessRef::Pid(f)
    }
}
impl From<Atom> for ProcessRef {
    fn from(f: Atom) -> Self {
        ProcessRef::Name(f)
    }
}
impl From<ProcessRef> for Term {
    fn from(f: ProcessRef) -> Self {
        match f {
            ProcessRef::Pid(x) => Term::from(x),
            ProcessRef::Name(x) => Term::from(x),
        }
    }
}

fn write_term<W: Write, T>(writer: &mut W, term: T) -> io::Result<()>
where
    Term: From<T>,
{
    let term = Term::from(term);
    term.encode(writer).map_err(|e| {
        use eetf::EncodeError;
        if let EncodeError::Io(e) = e {
            e
        } else {
            Error::new(ErrorKind::InvalidInput, Box::new(e))
        }
    })
}

fn read_term<R: Read, T>(reader: &mut R) -> io::Result<T>
where
    Term: TryInto<T>,
{
    use eetf::DecodeError;
    Term::decode(reader)
        .map_err(|e| {
            if let DecodeError::Io(e) = e {
                e
            } else {
                Error::new(ErrorKind::InvalidData, Box::new(e))
            }
        })
        .and_then(|term| {
            term.try_into()
                .map_err(|t| invalid_data!("Unexpected term: {}", t))
        })
}

fn tagged_tuple1(tag: u8) -> Tuple {
    Tuple {
        elements: vec![Term::from(FixInteger { value: tag as i32 })],
    }
}

fn tagged_tuple3<T1, T2>(tag: u8, t1: T1, t2: T2) -> Tuple
where
    Term: From<T1>,
    Term: From<T2>,
{
    Tuple {
        elements: vec![
            Term::from(FixInteger { value: tag as i32 }),
            Term::from(t1),
            Term::from(t2),
        ],
    }
}
fn tagged_tuple4<T1, T2, T3>(tag: u8, t1: T1, t2: T2, t3: T3) -> Tuple
where
    Term: From<T1>,
    Term: From<T2>,
    Term: From<T3>,
{
    Tuple {
        elements: vec![
            Term::from(FixInteger { value: tag as i32 }),
            Term::from(t1),
            Term::from(t2),
            Term::from(t3),
        ],
    }
}
fn tagged_tuple5<T1, T2, T3, T4>(tag: u8, t1: T1, t2: T2, t3: T3, t4: T4) -> Tuple
where
    Term: From<T1>,
    Term: From<T2>,
    Term: From<T3>,
    Term: From<T4>,
{
    Tuple {
        elements: vec![
            Term::from(FixInteger { value: tag as i32 }),
            Term::from(t1),
            Term::from(t2),
            Term::from(t3),
            Term::from(t4),
        ],
    }
}
