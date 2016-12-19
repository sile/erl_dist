use std::io::{self, Read, Write, Error, ErrorKind};
use eetf::{Pid, Term, Reference, Atom, FixInteger, Tuple};
use eetf::convert::{TryAsRef, TryInto};

pub const CTRL_TYPE_LINK: u8 = 1;
pub const CTRL_TYPE_SEND: u8 = 2;
pub const CTRL_TYPE_EXIT: u8 = 3;
pub const CTRL_TYPE_UNLINK: u8 = 4;
pub const CTRL_TYPE_NODE_LINK: u8 = 5;
pub const CTRL_TYPE_REG_SEND: u8 = 6;
pub const CTRL_TYPE_GROUP_LEADER: u8 = 7;
pub const CTRL_TYPE_EXIT2: u8 = 8;
pub const CTRL_TYPE_SEND_TT: u8 = 12;
pub const CTRL_TYPE_EXIT_TT: u8 = 13;
pub const CTRL_TYPE_REG_SEND_TT: u8 = 16;
pub const CTRL_TYPE_EXIT2_TT: u8 = 18;
pub const CTRL_TYPE_MONITOR_P: u8 = 19;
pub const CTRL_TYPE_DEMONITOR_P: u8 = 20;
pub const CTRL_TYPE_MONITOR_P_EXIT: u8 = 21;

pub const TAG_PASS_THROUGH: u8 = 112;

#[derive(Debug, Clone)]
pub struct DistributionHeader;

#[derive(Debug, Clone)]
pub struct Message {
    distribution_header: Option<DistributionHeader>,
    body: Body,
}
impl Message {
    fn new(body: Body) -> Self {
        Message {
            distribution_header: None,
            body: body,
        }
    }
    pub fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
        // TODO
        assert_eq!(bytes[0], TAG_PASS_THROUGH);
        let body = Body::read_from(&mut &bytes[1..])?;
        Ok(Message {
            distribution_header: None,
            body: body,
        })
    }
    pub fn into_bytes(self) -> io::Result<Vec<u8>> {
        assert!(self.distribution_header.is_none(), "Unimpelemented");
        let mut buf = vec![0; 4];
        buf.push(TAG_PASS_THROUGH);
        self.body.write_into(&mut buf)?;
        let message_len = buf.len() - 4;
        buf[0] = (message_len >> 24) as u8;
        buf[1] = (message_len >> 16) as u8;
        buf[2] = (message_len >> 8) as u8;
        buf[3] = message_len as u8;
        Ok(buf)
    }
}
impl From<Body> for Message {
    fn from(f: Body) -> Self {
        Message::new(f)
    }
}
macro_rules! impl_from_for_message {
    ($t:ident) => {
        impl From<$t> for Message {
            fn from(f: $t) -> Self {
                Message::new(Body::$t(f))
            }
        }
    }
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

#[derive(Debug, Clone)]
pub enum Body {
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
}
impl Body {
    pub fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
        match self {
            Body::Link(x) => x.write_into(writer),
            Body::Send(x) => x.write_into(writer),
            Body::Exit(x) => x.write_into(writer),
            Body::Unlink(x) => x.write_into(writer),
            Body::NodeLink(x) => x.write_into(writer),
            Body::RegSend(x) => x.write_into(writer),
            Body::GroupLeader(x) => x.write_into(writer),
            Body::Exit2(x) => x.write_into(writer),
            Body::SendTt(x) => x.write_into(writer),
            Body::ExitTt(x) => x.write_into(writer),
            Body::RegSendTt(x) => x.write_into(writer),
            Body::Exit2Tt(x) => x.write_into(writer),
            Body::MonitorP(x) => x.write_into(writer),
            Body::DemonitorP(x) => x.write_into(writer),
            Body::MonitorPExit(x) => x.write_into(writer),
        }
    }
    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        // TODO: error handlings
        let ctrl: Tuple = read_term(reader)?;
        let tag = {
            let tag: &FixInteger = ctrl.elements[0].try_as_ref().unwrap();
            tag.value as u8
        };
        Ok(match tag {
            CTRL_TYPE_LINK => Body::from(Link::read_from(ctrl, reader)?),
            CTRL_TYPE_SEND => Body::from(Send::read_from(ctrl, reader)?),
            CTRL_TYPE_EXIT => Body::from(Exit::read_from(ctrl, reader)?),
            CTRL_TYPE_UNLINK => Body::from(Unlink::read_from(ctrl, reader)?),
            CTRL_TYPE_NODE_LINK => Body::from(NodeLink::read_from(ctrl, reader)?),
            CTRL_TYPE_REG_SEND => Body::from(RegSend::read_from(ctrl, reader)?),
            CTRL_TYPE_GROUP_LEADER => Body::from(GroupLeader::read_from(ctrl, reader)?),
            CTRL_TYPE_EXIT2 => Body::from(Exit2::read_from(ctrl, reader)?),
            CTRL_TYPE_SEND_TT => Body::from(SendTt::read_from(ctrl, reader)?),
            CTRL_TYPE_EXIT_TT => Body::from(ExitTt::read_from(ctrl, reader)?),
            CTRL_TYPE_REG_SEND_TT => Body::from(RegSendTt::read_from(ctrl, reader)?),
            CTRL_TYPE_EXIT2_TT => Body::from(Exit2Tt::read_from(ctrl, reader)?),
            CTRL_TYPE_MONITOR_P => Body::from(MonitorP::read_from(ctrl, reader)?),
            CTRL_TYPE_DEMONITOR_P => Body::from(DemonitorP::read_from(ctrl, reader)?),
            CTRL_TYPE_MONITOR_P_EXIT => Body::from(MonitorPExit::read_from(ctrl, reader)?),
            _ => {
                return Err(Error::new(ErrorKind::InvalidData,
                                      format!("Unknown control message: type={}", tag)));
            }
        })
    }
}
macro_rules! impl_from_for_body {
    ($t:ident) => {
        impl From<$t> for Body {
            fn from(f: $t) -> Self {
                Body::$t(f)
            }
        }
    }
}
impl_from_for_body!(Link);
impl_from_for_body!(Send);
impl_from_for_body!(Exit);
impl_from_for_body!(Unlink);
impl_from_for_body!(NodeLink);
impl_from_for_body!(RegSend);
impl_from_for_body!(GroupLeader);
impl_from_for_body!(Exit2);
impl_from_for_body!(SendTt);
impl_from_for_body!(ExitTt);
impl_from_for_body!(RegSendTt);
impl_from_for_body!(Exit2Tt);
impl_from_for_body!(MonitorP);
impl_from_for_body!(DemonitorP);
impl_from_for_body!(MonitorPExit);

#[derive(Debug, Clone)]
pub struct Link {
    pub from_pid: Pid,
    pub to_pid: Pid,
}
impl Link {
    pub fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
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

#[derive(Debug, Clone)]
pub struct Send {
    pub to_pid: Pid,
    pub message: Term,
}
impl Send {
    pub fn new<T>(to_pid: Pid, message: T) -> Self
        where Term: From<T>
    {
        Send {
            to_pid: to_pid,
            message: Term::from(message),
        }
    }
    pub fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
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

#[derive(Debug, Clone)]
pub struct Exit {
    pub from_pid: Pid,
    pub to_pid: Pid,
    pub reason: Term,
}
impl Exit {
    pub fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
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

#[derive(Debug, Clone)]
pub struct Unlink {
    pub from_pid: Pid,
    pub to_pid: Pid,
}
impl Unlink {
    pub fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
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

#[derive(Debug, Clone)]
pub struct NodeLink;
impl NodeLink {
    pub fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
        let ctrl = tagged_tuple1(CTRL_TYPE_NODE_LINK);
        write_term(writer, ctrl)
    }
    fn read_from<R: Read>(_ctrl: Tuple, _reader: &mut R) -> io::Result<Self> {
        Ok(NodeLink)
    }
}

#[derive(Debug, Clone)]
pub struct RegSend {
    pub from_pid: Pid,
    pub to_name: Atom,
    pub message: Term,
}
impl RegSend {
    pub fn new<F, T, M>(from_pid: F, to_name: T, message: M) -> Self
        where Pid: From<F>,
              Atom: From<T>,
              Term: From<M>
    {
        RegSend {
            from_pid: From::from(from_pid),
            to_name: From::from(to_name),
            message: From::from(message),
        }
    }
    pub fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
        let ctrl = tagged_tuple4(CTRL_TYPE_REG_SEND,
                                 self.from_pid,
                                 Tuple::nil(),
                                 self.to_name);
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

#[derive(Debug, Clone)]
pub struct GroupLeader {
    pub from_pid: Pid,
    pub to_pid: Pid,
}
impl GroupLeader {
    pub fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
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

#[derive(Debug, Clone)]
pub struct Exit2 {
    pub from_pid: Pid,
    pub to_pid: Pid,
    pub reason: Term,
}
impl Exit2 {
    pub fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
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

#[derive(Debug, Clone)]
pub struct SendTt {
    pub to_pid: Pid,
    pub trace_token: Term,
    pub message: Term,
}
impl SendTt {
    pub fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
        let ctrl = tagged_tuple4(CTRL_TYPE_SEND_TT,
                                 Tuple::nil(),
                                 self.to_pid,
                                 self.trace_token);
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

#[derive(Debug, Clone)]
pub struct ExitTt {
    pub from_pid: Pid,
    pub to_pid: Pid,
    pub trace_token: Term,
    pub reason: Term,
}
impl ExitTt {
    pub fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
        let ctrl = tagged_tuple5(CTRL_TYPE_EXIT_TT,
                                 self.from_pid,
                                 self.to_pid,
                                 self.trace_token,
                                 self.reason);
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

#[derive(Debug, Clone)]
pub struct RegSendTt {
    pub from_pid: Pid,
    pub to_name: Atom,
    pub trace_token: Term,
    pub message: Term,
}
impl RegSendTt {
    pub fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
        let ctrl = tagged_tuple5(CTRL_TYPE_REG_SEND_TT,
                                 self.from_pid,
                                 Tuple::nil(),
                                 self.to_name,
                                 self.trace_token);
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

#[derive(Debug, Clone)]
pub struct Exit2Tt {
    pub from_pid: Pid,
    pub to_pid: Pid,
    pub trace_token: Term,
    pub reason: Term,
}
impl Exit2Tt {
    pub fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
        let ctrl = tagged_tuple5(CTRL_TYPE_EXIT2_TT,
                                 self.from_pid,
                                 self.to_pid,
                                 self.trace_token,
                                 self.reason);
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

#[derive(Debug, Clone)]
pub struct MonitorP {
    pub from_pid: Pid,
    pub to_proc: ProcessRef,
    pub reference: Reference,
}
impl MonitorP {
    pub fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
        let ctrl = tagged_tuple4(CTRL_TYPE_MONITOR_P,
                                 self.from_pid,
                                 self.to_proc,
                                 self.reference);
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

#[derive(Debug, Clone)]
pub struct DemonitorP {
    pub from_pid: Pid,
    pub to_proc: ProcessRef,
    pub reference: Reference,
}
impl DemonitorP {
    pub fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
        let ctrl = tagged_tuple4(CTRL_TYPE_DEMONITOR_P,
                                 self.from_pid,
                                 self.to_proc,
                                 self.reference);
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

#[derive(Debug, Clone)]
pub struct MonitorPExit {
    pub from_pid: Pid,
    pub to_pid: Pid,
    pub reference: Reference,
    pub reason: Term,
}
impl MonitorPExit {
    pub fn write_into<W: Write>(self, writer: &mut W) -> io::Result<()> {
        let ctrl = tagged_tuple5(CTRL_TYPE_DEMONITOR_P,
                                 self.from_pid,
                                 self.to_pid,
                                 self.reference,
                                 self.reason);
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

#[derive(Debug, Clone)]
pub enum ProcessRef {
    Pid(Pid),
    Name(Atom),
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
    where Term: From<T>
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
    where Term: TryInto<T>
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
                .map_err(|t| Error::new(ErrorKind::InvalidData, format!("Unexpected term: {}", t)))
        })
}

fn tagged_tuple1(tag: u8) -> Tuple {
    Tuple { elements: vec![Term::from(FixInteger { value: tag as i32 })] }
}

fn tagged_tuple3<T1, T2>(tag: u8, t1: T1, t2: T2) -> Tuple
    where Term: From<T1>,
          Term: From<T2>
{
    Tuple {
        elements: vec![Term::from(FixInteger { value: tag as i32 }),
                       Term::from(t1),
                       Term::from(t2)],
    }
}
fn tagged_tuple4<T1, T2, T3>(tag: u8, t1: T1, t2: T2, t3: T3) -> Tuple
    where Term: From<T1>,
          Term: From<T2>,
          Term: From<T3>
{
    Tuple {
        elements: vec![Term::from(FixInteger { value: tag as i32 }),
                       Term::from(t1),
                       Term::from(t2),
                       Term::from(t3)],
    }
}
fn tagged_tuple5<T1, T2, T3, T4>(tag: u8, t1: T1, t2: T2, t3: T3, t4: T4) -> Tuple
    where Term: From<T1>,
          Term: From<T2>,
          Term: From<T3>,
          Term: From<T4>
{
    Tuple {
        elements: vec![Term::from(FixInteger { value: tag as i32 }),
                       Term::from(t1),
                       Term::from(t2),
                       Term::from(t3),
                       Term::from(t4)],
    }
}
