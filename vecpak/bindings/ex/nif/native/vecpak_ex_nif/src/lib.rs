use rustler::types::{Binary, OwnedBinary};
use rustler::{
    Encoder, Env, Error, Term
};
use vecpak_ex::{ encode_term, decode_term_from_slice };

#[rustler::nif]
fn encode<'a>(env: Env<'a>, map: Term<'a>) -> Result<Term<'a>, Error> {
    let mut buf = Vec::with_capacity(1024);
    encode_term(env, &mut buf, map)?;

    let mut ob = OwnedBinary::new(buf.len()).ok_or_else(|| Error::Term(Box::new("alloc failed")))?;
    ob.as_mut_slice().copy_from_slice(&buf);

    Ok(Binary::from_owned(ob, env).encode(env))
}

#[rustler::nif]
fn decode<'a>(env: Env<'a>, bin: Binary) -> Result<Term<'a>, Error> {
    let term = decode_term_from_slice(env, bin.as_slice())?;
    Ok(term.encode(env))
}

rustler::init!("Elixir.VecPak");
