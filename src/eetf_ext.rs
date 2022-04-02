use eetf::{Atom, DecodeError, FixInteger, Pid, Reference, Term, Tuple};

pub fn nil() -> Term {
    eetf::List::nil().into()
}

pub fn check_tuple_len(tuple: &Tuple, n: usize) -> Result<(), DecodeError> {
    if tuple.elements.len() != n {
        return Err(DecodeError::UnexpectedType {
            value: tuple.clone().into(),
            expected: format!("{} elements tuple", n),
        });
    } else {
        Ok(())
    }
}

pub fn try_from_tagged_tuple3<T0, T1>(mut tuple: Tuple) -> Result<(T0, T1), DecodeError>
where
    T0: TryFromTerm,
    T1: TryFromTerm,
{
    check_tuple_len(&tuple, 3)?;
    Ok((
        T0::try_from_term(std::mem::replace(&mut tuple.elements[1], nil()))?,
        T1::try_from_term(std::mem::replace(&mut tuple.elements[2], nil()))?,
    ))
}

pub fn try_from_tagged_tuple4<T0, T1, T2>(mut tuple: Tuple) -> Result<(T0, T1, T2), DecodeError>
where
    T0: TryFromTerm,
    T1: TryFromTerm,
    T2: TryFromTerm,
{
    check_tuple_len(&tuple, 4)?;
    Ok((
        T0::try_from_term(std::mem::replace(&mut tuple.elements[1], nil()))?,
        T1::try_from_term(std::mem::replace(&mut tuple.elements[2], nil()))?,
        T2::try_from_term(std::mem::replace(&mut tuple.elements[3], nil()))?,
    ))
}

pub fn try_from_tagged_tuple5<T0, T1, T2, T3>(
    mut tuple: Tuple,
) -> Result<(T0, T1, T2, T3), DecodeError>
where
    T0: TryFromTerm,
    T1: TryFromTerm,
    T2: TryFromTerm,
    T3: TryFromTerm,
{
    check_tuple_len(&tuple, 5)?;
    Ok((
        T0::try_from_term(std::mem::replace(&mut tuple.elements[1], nil()))?,
        T1::try_from_term(std::mem::replace(&mut tuple.elements[2], nil()))?,
        T2::try_from_term(std::mem::replace(&mut tuple.elements[3], nil()))?,
        T3::try_from_term(std::mem::replace(&mut tuple.elements[4], nil()))?,
    ))
}

pub fn try_from_tagged_tuple6<T0, T1, T2, T3, T4>(
    mut tuple: Tuple,
) -> Result<(T0, T1, T2, T3, T4), DecodeError>
where
    T0: TryFromTerm,
    T1: TryFromTerm,
    T2: TryFromTerm,
    T3: TryFromTerm,
    T4: TryFromTerm,
{
    check_tuple_len(&tuple, 6)?;
    Ok((
        T0::try_from_term(std::mem::replace(&mut tuple.elements[1], nil()))?,
        T1::try_from_term(std::mem::replace(&mut tuple.elements[2], nil()))?,
        T2::try_from_term(std::mem::replace(&mut tuple.elements[3], nil()))?,
        T3::try_from_term(std::mem::replace(&mut tuple.elements[4], nil()))?,
        T4::try_from_term(std::mem::replace(&mut tuple.elements[5], nil()))?,
    ))
}

pub fn try_from_tagged_tuple7<T0, T1, T2, T3, T4, T5>(
    mut tuple: Tuple,
) -> Result<(T0, T1, T2, T3, T4, T5), DecodeError>
where
    T0: TryFromTerm,
    T1: TryFromTerm,
    T2: TryFromTerm,
    T3: TryFromTerm,
    T4: TryFromTerm,
    T5: TryFromTerm,
{
    check_tuple_len(&tuple, 7)?;
    Ok((
        T0::try_from_term(std::mem::replace(&mut tuple.elements[1], nil()))?,
        T1::try_from_term(std::mem::replace(&mut tuple.elements[2], nil()))?,
        T2::try_from_term(std::mem::replace(&mut tuple.elements[3], nil()))?,
        T3::try_from_term(std::mem::replace(&mut tuple.elements[4], nil()))?,
        T4::try_from_term(std::mem::replace(&mut tuple.elements[5], nil()))?,
        T5::try_from_term(std::mem::replace(&mut tuple.elements[6], nil()))?,
    ))
}

pub trait TryFromTerm: Sized {
    fn try_from_term(term: Term) -> Result<Self, DecodeError>;
}

impl TryFromTerm for Term {
    fn try_from_term(term: Term) -> Result<Self, DecodeError> {
        Ok(term)
    }
}

impl TryFromTerm for Pid {
    fn try_from_term(term: Term) -> Result<Self, DecodeError> {
        try_from_term(term, "pid")
    }
}

impl TryFromTerm for Atom {
    fn try_from_term(term: Term) -> Result<Self, DecodeError> {
        try_from_term(term, "atom")
    }
}

impl TryFromTerm for Reference {
    fn try_from_term(term: Term) -> Result<Self, DecodeError> {
        try_from_term(term, "reference")
    }
}

impl TryFromTerm for FixInteger {
    fn try_from_term(term: Term) -> Result<Self, DecodeError> {
        try_from_term(term, "integer")
    }
}

pub fn try_from_term<T>(term: Term, expected: &str) -> Result<T, DecodeError>
where
    Term: TryInto<T, Error = Term>,
{
    term.try_into()
        .map_err(|value| DecodeError::UnexpectedType {
            value,
            expected: expected.to_owned(),
        })
}
