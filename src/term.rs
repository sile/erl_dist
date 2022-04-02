//! Erlang terms.
pub use eetf::{
    Atom, BigInteger, Binary, BitBinary, ExternalFun, FixInteger, Float, ImproperList, InternalFun,
    List, Map, Pid, Port, Reference, Term, Tuple,
};

/// [`Pid`] or [`Atom`]
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub enum PidOrAtom {
    Pid(Pid),
    Atom(Atom),
}

impl From<PidOrAtom> for Term {
    fn from(v: PidOrAtom) -> Self {
        match v {
            PidOrAtom::Pid(v) => v.into(),
            PidOrAtom::Atom(v) => v.into(),
        }
    }
}

/// { [`Atom`], [`Atom`], [`FixInteger`] }
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub struct Mfa {
    pub module: Atom,
    pub function: Atom,
    pub arity: FixInteger,
}

impl From<Mfa> for Term {
    fn from(v: Mfa) -> Self {
        Tuple::from(vec![v.module.into(), v.function.into(), v.arity.into()]).into()
    }
}
