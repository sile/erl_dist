//! Messages passed between distributed nodes, and channels for those messages.
//!
//! Reference: [Protocol between Connected Nodes](https://www.erlang.org/doc/apps/erts/erl_dist_protocol.html#protocol-between-connected-nodes)
//!
//! Note that distribution headers are not supported in the current version.
use crate::eetf_ext;
use crate::io::{ReadTermExt, WriteTermExt};
use crate::term::{Atom, FixInteger, List, Mfa, Pid, PidOrAtom, Reference, Term, Tuple};
#[cfg(doc)]
use crate::DistributionFlags;
use eetf::{DecodeError, EncodeError};
use std::io::{Read, Write};

pub use crate::channel::{channel, Receiver, RecvError, SendError, Sender};

trait DistributionMessage: Sized {
    const OP: i32;
    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError>;
    fn read_from<R: Read>(reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError>;
}

/// This signal is sent by `from_pid` in order to create a link between `from_pid` and `to_pid`.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
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

    fn read_from<R: Read>(_reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (from_pid, to_pid) = eetf_ext::try_from_tagged_tuple3(ctrl_msg)?;
        Ok(Self { from_pid, to_pid })
    }
}

/// This signal is used to send a message to `to_pid`.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
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

    fn read_from<R: Read>(reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (_, to_pid): (Term, _) = eetf_ext::try_from_tagged_tuple3(ctrl_msg)?;
        let message = reader.read_term()?;
        Ok(Self { to_pid, message })
    }
}

/// This signal is sent when a link has been broken.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
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

    fn read_from<R: Read>(_reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (from_pid, to_pid, reason) = eetf_ext::try_from_tagged_tuple4(ctrl_msg)?;
        Ok(Self {
            from_pid,
            to_pid,
            reason,
        })
    }
}

/// **DEPRECATED** This signal is sent by `from_pid` in order to remove a link between `from_pid` and `to_pid`.
///
/// This signal has been deprecated and will not be supported in OTP 26.
/// For more information see the documentation of the [new link protocol](https://www.erlang.org/doc/apps/erts/erl_dist_protocol.html#new_link_protocol).
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
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

    fn read_from<R: Read>(_reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (from_pid, to_pid) = eetf_ext::try_from_tagged_tuple3(ctrl_msg)?;
        Ok(Self { from_pid, to_pid })
    }
}

/// Node link.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeLink;

impl DistributionMessage for NodeLink {
    const OP: i32 = 5;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple1(Self::OP)?;
        Ok(())
    }

    fn read_from<R: Read>(_reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        eetf_ext::check_tuple_len(&ctrl_msg, 1)?;
        Ok(Self {})
    }
}

/// This signal is used to send a message to a named process.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
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

    fn read_from<R: Read>(reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (from_pid, _, to_name): (_, Term, _) = eetf_ext::try_from_tagged_tuple4(ctrl_msg)?;
        let message = reader.read_term()?;
        Ok(Self {
            from_pid,
            to_name,
            message,
        })
    }
}

/// Group leader.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
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

    fn read_from<R: Read>(_reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (from_pid, to_pid) = eetf_ext::try_from_tagged_tuple3(ctrl_msg)?;
        Ok(Self { from_pid, to_pid })
    }
}

/// This signal is sent by a call to the `erlang:exit/2` bif.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
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

    fn read_from<R: Read>(_reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (from_pid, to_pid, reason) = eetf_ext::try_from_tagged_tuple4(ctrl_msg)?;
        Ok(Self {
            from_pid,
            to_pid,
            reason,
        })
    }
}

/// [`Send`] with a trace token.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
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

    fn read_from<R: Read>(reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (_, trace_token, to_pid): (Term, _, _) = eetf_ext::try_from_tagged_tuple4(ctrl_msg)?;
        let message = reader.read_term()?;
        Ok(Self {
            to_pid,
            trace_token,
            message,
        })
    }
}

/// [`Exit`] with a trace token.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
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

    fn read_from<R: Read>(_reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (from_pid, to_pid, trace_token, reason) = eetf_ext::try_from_tagged_tuple5(ctrl_msg)?;
        Ok(Self {
            from_pid,
            to_pid,
            trace_token,
            reason,
        })
    }
}

/// [`RegSend`] with a trace token.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
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

    fn read_from<R: Read>(reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (from_pid, _, to_name, trace_token): (_, Term, _, _) =
            eetf_ext::try_from_tagged_tuple5(ctrl_msg)?;
        let message = reader.read_term()?;
        Ok(Self {
            from_pid,
            to_name,
            trace_token,
            message,
        })
    }
}

/// [`Exit2`] with a trace token.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
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

    fn read_from<R: Read>(_reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (from_pid, to_pid, trace_token, reason) = eetf_ext::try_from_tagged_tuple5(ctrl_msg)?;
        Ok(Self {
            from_pid,
            to_pid,
            trace_token,
            reason,
        })
    }
}

/// `from_pid` = monitoring process and `to_proc` = monitored process pid or name (atom).
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub struct MonitorP {
    pub from_pid: Pid,
    pub to_proc: PidOrAtom,
    pub reference: Reference,
}

impl DistributionMessage for MonitorP {
    const OP: i32 = 19;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple4(Self::OP, self.from_pid, self.to_proc, self.reference)?;
        Ok(())
    }

    fn read_from<R: Read>(_reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (from_pid, to_proc, reference) = eetf_ext::try_from_tagged_tuple4(ctrl_msg)?;
        Ok(Self {
            from_pid,
            to_proc,
            reference,
        })
    }
}

/// `from_pid` = monitoring process and `to_proc` = monitored process pid or name (atom).
///
/// We include `from_pid` just in case we want to trace this.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub struct DemonitorP {
    pub from_pid: Pid,
    pub to_proc: PidOrAtom,
    pub reference: Reference,
}

impl DistributionMessage for DemonitorP {
    const OP: i32 = 20;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple4(Self::OP, self.from_pid, self.to_proc, self.reference)?;
        Ok(())
    }

    fn read_from<R: Read>(_reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (from_pid, to_proc, reference) = eetf_ext::try_from_tagged_tuple4(ctrl_msg)?;
        Ok(Self {
            from_pid,
            to_proc,
            reference,
        })
    }
}

/// `from_proc` = monitored process pid or name (atom), `to_pid` = monitoring process, and `reason` = exit reason for the monitored process.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub struct MonitorPExit {
    pub from_pid: Pid,
    pub to_proc: PidOrAtom,
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

    fn read_from<R: Read>(_reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (from_pid, to_proc, reference, reason) = eetf_ext::try_from_tagged_tuple5(ctrl_msg)?;
        Ok(Self {
            from_pid,
            to_proc,
            reference,
            reason,
        })
    }
}

/// This control message replaces the [`Send`] control message and will be sent when the distribution flag `DistributionFlags::SEND_SENDER` has been negotiated in the connection setup handshake.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
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

    fn read_from<R: Read>(reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (from_pid, to_pid) = eetf_ext::try_from_tagged_tuple3(ctrl_msg)?;
        let message = reader.read_term()?;
        Ok(Self {
            from_pid,
            to_pid,
            message,
        })
    }
}

/// [`SendSender`] with a trace token.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
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

    fn read_from<R: Read>(reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (from_pid, to_pid, trace_token) = eetf_ext::try_from_tagged_tuple4(ctrl_msg)?;
        let message = reader.read_term()?;
        Ok(Self {
            from_pid,
            to_pid,
            trace_token,
            message,
        })
    }
}

/// This control message replaces the [`Exit`] control message and will be sent when the distribution flag [`DistributionFlags::EXIT_PAYLOAD`] has been negotiated in the connection setup handshake.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
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

    fn read_from<R: Read>(reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (from_pid, to_pid) = eetf_ext::try_from_tagged_tuple3(ctrl_msg)?;
        let reason = reader.read_term()?;
        Ok(Self {
            from_pid,
            to_pid,
            reason,
        })
    }
}

/// [`PayloadExit`] with a trace token.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
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

    fn read_from<R: Read>(reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (from_pid, to_pid, trace_token) = eetf_ext::try_from_tagged_tuple4(ctrl_msg)?;
        let reason = reader.read_term()?;
        Ok(Self {
            from_pid,
            to_pid,
            trace_token,
            reason,
        })
    }
}

/// This control message replaces the [`Exit2`] control message and will be sent when the distribution flag [`DistributionFlags::EXIT_PAYLOAD`] has been negotiated in the connection setup handshake.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
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

    fn read_from<R: Read>(reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (from_pid, to_pid) = eetf_ext::try_from_tagged_tuple3(ctrl_msg)?;
        let reason = reader.read_term()?;
        Ok(Self {
            from_pid,
            to_pid,
            reason,
        })
    }
}

/// [`PayloadExit2`] with a trace token.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
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

    fn read_from<R: Read>(reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (from_pid, to_pid, trace_token) = eetf_ext::try_from_tagged_tuple4(ctrl_msg)?;
        let reason = reader.read_term()?;
        Ok(Self {
            from_pid,
            to_pid,
            trace_token,
            reason,
        })
    }
}

/// This control message replaces the [`MonitorPExit`] control message and will be sent when the distribution flag [`DistributionFlags::EXIT_PAYLOAD`] has been negotiated in the connection setup handshake.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub struct PayloadMonitorPExit {
    pub from_pid: Pid,
    pub to_proc: PidOrAtom,
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

    fn read_from<R: Read>(reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (from_pid, to_proc, reference) = eetf_ext::try_from_tagged_tuple4(ctrl_msg)?;
        let reason = reader.read_term()?;
        Ok(Self {
            from_pid,
            to_proc,
            reference,
            reason,
        })
    }
}

/// This signal is sent by the `spawn_request()` BIF.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub struct SpawnRequest {
    pub req_id: Reference,
    pub from_pid: Pid,
    pub group_leader: Pid,
    pub mfa: Mfa,
    pub opt_list: List,
    pub arg_list: List,
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

    fn read_from<R: Read>(reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (req_id, from_pid, group_leader, mfa, opt_list) =
            eetf_ext::try_from_tagged_tuple6(ctrl_msg)?;
        let arg_list = eetf_ext::try_from_term(reader.read_term()?, "list")?;
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

/// [`SpawnRequest`] with a trace token.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub struct SpawnRequestTt {
    pub req_id: Reference,
    pub from_pid: Pid,
    pub group_leader: Pid,
    pub mfa: Mfa,
    pub opt_list: List,
    pub trace_token: Term,
    pub arg_list: List,
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

    fn read_from<R: Read>(reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (req_id, from_pid, group_leader, mfa, opt_list, trace_token) =
            eetf_ext::try_from_tagged_tuple7(ctrl_msg)?;
        let arg_list = eetf_ext::try_from_term(reader.read_term()?, "list")?;
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

/// This signal is sent as a reply to a process previously sending a [`SpawnRequest`] signal.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub struct SpawnReply {
    pub req_id: Reference,
    pub to_pid: Pid,
    pub flags: FixInteger,
    pub result: PidOrAtom,
}

impl DistributionMessage for SpawnReply {
    const OP: i32 = 31;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple5(Self::OP, self.req_id, self.to_pid, self.flags, self.result)?;
        Ok(())
    }

    fn read_from<R: Read>(_reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (req_id, to_pid, flags, result) = eetf_ext::try_from_tagged_tuple5(ctrl_msg)?;
        Ok(Self {
            req_id,
            to_pid,
            flags,
            result,
        })
    }
}

/// [`SpawnReply`] with a trace token.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub struct SpawnReplyTt {
    pub req_id: Reference,
    pub to_pid: Pid,
    pub flags: FixInteger,
    pub result: PidOrAtom,
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

    fn read_from<R: Read>(_reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (req_id, to_pid, flags, result, trace_token) =
            eetf_ext::try_from_tagged_tuple6(ctrl_msg)?;
        Ok(Self {
            req_id,
            to_pid,
            flags,
            result,
            trace_token,
        })
    }
}

/// This signal is sent by `from_pid` in order to remove a link between `from_pid` and `to_pid`.
///
/// This unlink signal replaces the [`Unlink`] signal.
/// This signal is only passed when the [new link protocol](https://www.erlang.org/doc/apps/erts/erl_dist_protocol.html#new_link_protocol) has been negotiated using the [`DistributionFlags::UNLINK_ID`] distribution flag.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
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

    fn read_from<R: Read>(_reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (id, from_pid, to_pid) = eetf_ext::try_from_tagged_tuple4(ctrl_msg)?;
        Ok(Self {
            id,
            from_pid,
            to_pid,
        })
    }
}

/// An unlink acknowledgement signal.
///
/// This signal is sent as an acknowledgement of the reception of an [`UnlinkId`] signal.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
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

    fn read_from<R: Read>(_reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (id, from_pid, to_pid) = eetf_ext::try_from_tagged_tuple4(ctrl_msg)?;
        Ok(Self {
            id,
            from_pid,
            to_pid,
        })
    }
}

/// This control message is used when sending the message Message to the process identified by the process alias `alias`.
///
/// Nodes that can handle this control message sets the distribution flag [`DistributionFlags::ALIAS`] in the connection setup handshake.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub struct AliasSend {
    pub from_pid: Pid,
    pub alias: Reference,
    pub message: Term,
}

impl DistributionMessage for AliasSend {
    const OP: i32 = 33;

    fn write_into<W: Write>(self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_tagged_tuple3(Self::OP, self.from_pid, self.alias)?;
        writer.write_term(self.message)?;
        Ok(())
    }

    fn read_from<R: Read>(reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (from_pid, alias) = eetf_ext::try_from_tagged_tuple3(ctrl_msg)?;
        let message = reader.read_term()?;
        Ok(Self {
            from_pid,
            alias,
            message,
        })
    }
}

/// [`AliasSend`] with a trace token.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub struct AliasSendTt {
    pub from_pid: Pid,
    pub alias: Reference,
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

    fn read_from<R: Read>(reader: &mut R, ctrl_msg: Tuple) -> Result<Self, DecodeError> {
        let (from_pid, alias, trace_token) = eetf_ext::try_from_tagged_tuple4(ctrl_msg)?;
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
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
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

    /// Tick message used for keeping alive a connection.
    ///
    /// See also: [`net_ticktime` parameter](https://www.erlang.org/doc/man/kernel_app.html#net_ticktime)
    Tick,
}

impl Message {
    /// Makes a [`Link`] message.
    pub fn link(from_pid: Pid, to_pid: Pid) -> Self {
        Self::Link(Link { from_pid, to_pid })
    }

    /// Makes a [`Send`] message.
    pub fn send(to_pid: Pid, message: Term) -> Self {
        Self::Send(Send { to_pid, message })
    }

    /// Makes a [`Exit`] message.
    pub fn exit(from_pid: Pid, to_pid: Pid, reason: Term) -> Self {
        Self::Exit(Exit {
            from_pid,
            to_pid,
            reason,
        })
    }

    /// Makes a [`Unlink`] message.
    pub fn unlink(from_pid: Pid, to_pid: Pid) -> Self {
        Self::Unlink(Unlink { from_pid, to_pid })
    }

    /// Makes a [`NodeLink`] message.
    pub fn node_link() -> Self {
        Self::NodeLink(NodeLink)
    }

    /// Makes a [`RegSend`] message.
    pub fn reg_send(from_pid: Pid, to_name: Atom, message: Term) -> Self {
        Self::RegSend(RegSend {
            from_pid,
            to_name,
            message,
        })
    }

    /// Makes a [`GroupLeader`] message.
    pub fn group_leader(from_pid: Pid, to_pid: Pid) -> Self {
        Self::GroupLeader(GroupLeader { from_pid, to_pid })
    }

    /// Makes a [`Exit2`] message.
    pub fn exit2(from_pid: Pid, to_pid: Pid, reason: Term) -> Self {
        Self::Exit2(Exit2 {
            from_pid,
            to_pid,
            reason,
        })
    }

    /// Makes a [`SendTt`] message.
    pub fn send_tt(to_pid: Pid, message: Term, trace_token: Term) -> Self {
        Self::SendTt(SendTt {
            to_pid,
            trace_token,
            message,
        })
    }

    /// Makes a [`ExitTt`] message.
    pub fn exit_tt(from_pid: Pid, to_pid: Pid, reason: Term, trace_token: Term) -> Self {
        Self::ExitTt(ExitTt {
            from_pid,
            to_pid,
            trace_token,
            reason,
        })
    }

    /// Makes a [`RegSendTt`] message.
    pub fn reg_send_tt(from_pid: Pid, to_name: Atom, message: Term, trace_token: Term) -> Self {
        Self::RegSendTt(RegSendTt {
            from_pid,
            to_name,
            trace_token,
            message,
        })
    }

    /// Makes a [`Exit2Tt`] message.
    pub fn exit2_tt(from_pid: Pid, to_pid: Pid, reason: Term, trace_token: Term) -> Self {
        Self::Exit2Tt(Exit2Tt {
            from_pid,
            to_pid,
            trace_token,
            reason,
        })
    }

    /// Makes as [`MonitorP`] message.
    pub fn monitor_p(from_pid: Pid, to_proc: PidOrAtom, reference: Reference) -> Self {
        Self::MonitorP(MonitorP {
            from_pid,
            to_proc,
            reference,
        })
    }

    /// Makes as [`DemonitorP`] message.
    pub fn demonitor_p(from_pid: Pid, to_proc: PidOrAtom, reference: Reference) -> Self {
        Self::DemonitorP(DemonitorP {
            from_pid,
            to_proc,
            reference,
        })
    }

    /// Makes as [`MonitorPExit`] message.
    pub fn monitor_p_exit(
        from_pid: Pid,
        to_proc: PidOrAtom,
        reference: Reference,
        reason: Term,
    ) -> Self {
        Self::MonitorPExit(MonitorPExit {
            from_pid,
            to_proc,
            reference,
            reason,
        })
    }

    /// Makes a [`SendSender`] message.
    pub fn send_sender(from_pid: Pid, to_pid: Pid, message: Term) -> Self {
        Self::SendSender(SendSender {
            from_pid,
            to_pid,
            message,
        })
    }

    /// Makes a [`SendSenderTt`] message.
    pub fn send_sender_tt(from_pid: Pid, to_pid: Pid, message: Term, trace_token: Term) -> Self {
        Self::SendSenderTt(SendSenderTt {
            from_pid,
            to_pid,
            message,
            trace_token,
        })
    }

    /// Makes a [`PayloadExit`] message.
    pub fn payload_exit(from_pid: Pid, to_pid: Pid, reason: Term) -> Self {
        Self::PayloadExit(PayloadExit {
            from_pid,
            to_pid,
            reason,
        })
    }

    /// Makes a [`PayloadExitTt`] message.
    pub fn payload_exit_tt(from_pid: Pid, to_pid: Pid, reason: Term, trace_token: Term) -> Self {
        Self::PayloadExitTt(PayloadExitTt {
            from_pid,
            to_pid,
            reason,
            trace_token,
        })
    }

    /// Makes a [`PayloadExit2`] message.
    pub fn payload_exit2(from_pid: Pid, to_pid: Pid, reason: Term) -> Self {
        Self::PayloadExit2(PayloadExit2 {
            from_pid,
            to_pid,
            reason,
        })
    }

    /// Makes a [`PayloadExit2Tt`] message.
    pub fn payload_exit2_tt(from_pid: Pid, to_pid: Pid, reason: Term, trace_token: Term) -> Self {
        Self::PayloadExit2Tt(PayloadExit2Tt {
            from_pid,
            to_pid,
            reason,
            trace_token,
        })
    }

    /// Makes a [`PayloadMonitorPExit`] message.
    pub fn payload_monitor_p_exit(
        from_pid: Pid,
        to_proc: PidOrAtom,
        reference: Reference,
        reason: Term,
    ) -> Self {
        Self::PayloadMonitorPExit(PayloadMonitorPExit {
            from_pid,
            to_proc,
            reference,
            reason,
        })
    }

    /// Makes a [`SpawnRequest`] message.
    pub fn spawn_request(
        req_id: Reference,
        from_pid: Pid,
        group_leader: Pid,
        mfa: Mfa,
        opt_list: List,
        arg_list: List,
    ) -> Self {
        Self::SpawnRequest(SpawnRequest {
            req_id,
            from_pid,
            group_leader,
            mfa,
            opt_list,
            arg_list,
        })
    }

    /// Makes a [`SpawnRequestTt`] message.
    pub fn spawn_request_tt(
        req_id: Reference,
        from_pid: Pid,
        group_leader: Pid,
        mfa: Mfa,
        opt_list: List,
        arg_list: List,
        trace_token: Term,
    ) -> Self {
        Self::SpawnRequestTt(SpawnRequestTt {
            req_id,
            from_pid,
            group_leader,
            mfa,
            opt_list,
            arg_list,
            trace_token,
        })
    }

    /// Makes a [`SpawnReply`] message.
    pub fn spawn_reply(
        req_id: Reference,
        to_pid: Pid,
        flags: FixInteger,
        result: PidOrAtom,
    ) -> Self {
        Self::SpawnReply(SpawnReply {
            req_id,
            to_pid,
            flags,
            result,
        })
    }

    /// Makes a [`SpawnReplyTt`] message.
    pub fn spawn_reply_tt(
        req_id: Reference,
        to_pid: Pid,
        flags: FixInteger,
        result: PidOrAtom,
        trace_token: Term,
    ) -> Self {
        Self::SpawnReplyTt(SpawnReplyTt {
            req_id,
            to_pid,
            flags,
            result,
            trace_token,
        })
    }

    /// Makes a [`UnlinkId`] message.
    pub fn unlink_id(id: Term, from_pid: Pid, to_pid: Pid) -> Self {
        Self::UnlinkId(UnlinkId {
            id,
            from_pid,
            to_pid,
        })
    }

    /// Makes a [`UnlinkIdAck`] message.
    pub fn unlink_id_ack(id: Term, from_pid: Pid, to_pid: Pid) -> Self {
        Self::UnlinkIdAck(UnlinkIdAck {
            id,
            from_pid,
            to_pid,
        })
    }

    /// Makes a [`AliasSend`] message.
    pub fn alias_send(from_pid: Pid, alias: Reference, message: Term) -> Self {
        Self::AliasSend(AliasSend {
            from_pid,
            alias,
            message,
        })
    }

    /// Makes a [`AliasSendTt`] message.
    pub fn alias_send_tt(
        from_pid: Pid,
        alias: Reference,
        message: Term,
        trace_token: Term,
    ) -> Self {
        Self::AliasSendTt(AliasSendTt {
            from_pid,
            alias,
            message,
            trace_token,
        })
    }

    pub(crate) fn write_into<W: Write>(
        self,
        writer: &mut W,
    ) -> Result<(), crate::channel::SendError> {
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
            Self::Tick => unreachable!(),
        }
        Ok(())
    }

    pub(crate) fn read_from<R: Read>(reader: &mut R) -> Result<Self, crate::channel::RecvError> {
        let mut ctrl_msg = reader.read_tuple()?;
        if ctrl_msg.elements.is_empty() {
            return Err(DecodeError::UnexpectedType {
                value: ctrl_msg.into(),
                expected: "non empty tuple".to_owned(),
            }
            .into());
        }
        let op: FixInteger = eetf_ext::try_from_term(
            std::mem::replace(&mut ctrl_msg.elements[0], eetf_ext::nil()),
            "integer",
        )?;
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
