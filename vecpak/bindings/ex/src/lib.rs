use rustler::types::{tuple, map::{MapIterator}, BigInt, Binary, OwnedBinary};
use rustler::{
    Atom, Decoder, Encoder, Env, Error, Term
};
use num_traits::ToPrimitive;
use vecpak::{ encode_varint, decode_varint };

#[inline(always)]
fn decode_varint_ex(buf: &[u8], i: &mut usize) -> Result<i128, Error> {
    decode_varint(buf, i).map_err(|e| Error::Atom(e))
}

//pub fn encode_term(env: Env, buf: &mut Vec<u8>, term: Term) -> NifResult<()> {
pub fn encode_term(env: Env, buf: &mut Vec<u8>, term: Term) -> Result<(), Error> {
    // ---- nil (tag 0) ----
    if rustler::types::atom::nil().eq(&term) {
        buf.push(0);
        return Ok(())
    }
    // ---- bool (tag 1 | 2) ----
    if let Ok(b) = term.decode::<bool>() {
        if b { buf.push(1); } else { buf.push(2); };
        return Ok(())
    }
    // ---- VarInt (tag 3) ----
    if let Ok(i) = term.decode::<i64>() {
        buf.push(3);
        encode_varint(buf, i as i128);
        return Ok(())
    }
    if let Ok(i) = term.decode::<u64>() {
        buf.push(3);
        encode_varint(buf, i as i128);
        return Ok(())
    }
    if let Ok(bi) = BigInt::decode(term) {
        if let Some(i) = bi.to_i128() {
            buf.push(3);
            encode_varint(buf, i);
            return Ok(())
        } else {
            return Err(Error::BadArg);
        }
    }
    // ---- Binary (tag 5) OR Atom (tag 5) ----
    if let Ok(bin) = Binary::from_term(term) {
        buf.push(5);
        encode_varint(buf, bin.len() as i128);
        buf.extend_from_slice(bin.as_slice());
        return Ok(());
    }
    if let Ok(atom_string) = term.atom_to_string() {
        buf.push(5);
        encode_varint(buf, atom_string.len() as i128);
        buf.extend_from_slice(atom_string.as_bytes());
        return Ok(());
    }

    // ---- Map (tag 7) ----
    if let Ok(iter) = term.decode::<MapIterator>() {
        buf.push(7);
        encode_varint(buf, term.map_size()? as i128);

        let mut keyed: Vec<(Vec<u8>, Term)> = Vec::with_capacity(iter.size_hint().0);
        for (k, v) in iter {
            let mut kbytes = Vec::with_capacity(32);
            encode_term(env, &mut kbytes, k)?;
            keyed.push((kbytes, v));
        }
        keyed.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        for (kbytes, v) in keyed {
            buf.extend_from_slice(&kbytes);
            encode_term(env, buf, v)?;
        }
        return Ok(());
    }

    // ---- PropList (tag 7) ----
    if let Ok(mut it) = term.into_list_iterator() {
        // Peek first element to decide if it's a proplist
        let mut tmp_pairs: Vec<(Term, Term)> = Vec::new();
        let mut is_proplist = term.list_length()? > 0;

        while let Some(elem) = it.next() {
            if let Ok((k, v)) = elem.decode::<(Term, Term)>() {
                tmp_pairs.push((k, v));
            } else {
                is_proplist = false;
                break;
            }
        }

        if is_proplist {
            buf.push(7);
            encode_varint(buf, term.list_length()? as i128);

            let mut keyed: Vec<(Vec<u8>, Term)> = Vec::with_capacity(tmp_pairs.len());
            for (k, v) in tmp_pairs {
                let mut kbytes = Vec::with_capacity(32);
                encode_term(env, &mut kbytes, k)?;
                keyed.push((kbytes, v));
            }
            keyed.sort_unstable_by(|a, b| a.0.cmp(&b.0));
            for (kbytes, v) in keyed {
                buf.extend_from_slice(&kbytes);
                encode_term(env, buf, v)?;
            }
            return Ok(());
        } else {
            // Not a proplist (tag 6)
            buf.push(6);
            encode_varint(buf, term.list_length()? as i128);
            let it = term.into_list_iterator().expect("list_iterator");
            for v in it {
                encode_term(env, buf, v)?;
            }
            return Ok(());
        }
    }

    // ---- Tuple (encode as list 6) ----
    if let Ok(tpl) = tuple::get_tuple(term) {
        buf.push(6);
        encode_varint(buf, tpl.len() as i128);
        for v in tpl {
            encode_term(env, buf, v)?;
        }
        return Ok(());
    }

    Err(Error::BadArg)
}

#[inline]
fn decode_varint_gt_zero(buf: &[u8], i: &mut usize) -> Result<usize, Error> {
    let n = decode_varint_ex(buf, i)?;
    if n < 0 { return Err(Error::Atom("length_is_negative")); }
    usize::try_from(n).map_err(|_| Error::Atom("length_overflow"))
}

#[inline]
fn read_u8(buf: &[u8], i: &mut usize) -> Result<u8, Error> {
    if *i >= buf.len() { return Err(Error::Atom("eof")); }
    let b = buf[*i];
    *i += 1;
    Ok(b)
}

#[inline]
fn read_exact<'a>(buf: &'a [u8], i: &mut usize, n: usize) -> Result<&'a [u8], Error> {
    if buf.len().saturating_sub(*i) < n { return Err(Error::Atom("eof")); }
    let s = &buf[*i..*i + n];
    *i += n;
    Ok(s)
}

//RDB.vecpak_decode(<<7, 1, 3, 5, 1, 2, 122, 97, 3, 1, 4, 5, 1, 3, 97, 102, 97, 5, 1, 4, 116, 101, 115, 116, 5, 1, 3, 98, 122, 122, 5, 1, 4, 98, 101, 115, 116>>)
pub fn decode_term<'a>(env: Env<'a>, buf: &[u8], i: &mut usize) -> Result<Term<'a>, Error> {
    let tag = read_u8(buf, i)?;
    match tag {
        0 => { Ok(rustler::types::atom::nil().encode(env)) }
        1 => { Ok(true.encode(env)) }
        2 => { Ok(false.encode(env)) }
        3 => {
            let v = decode_varint_ex(buf, i)?;
            let term = v.encode(env);
            Ok(term)
        }
        5 => {
            let len = decode_varint_gt_zero(buf, i)?;
            let bytes = read_exact(buf, i, len as usize)?;
            let mut ob = OwnedBinary::new(len).ok_or(Error::Atom("alloc_failed"))?;
            ob.as_mut_slice().copy_from_slice(&bytes);
            let bin = ob.release(env);
            Ok(bin.encode(env))
        }
        6 => {
            let count = decode_varint_gt_zero(buf, i)?;
            let mut items: Vec<Term> = Vec::with_capacity(count);
            for _ in 0..count {
                items.push(decode_term(env, buf, i)?);
            }
            Ok(items.encode(env))
        }
        7 => {
            let count = decode_varint_gt_zero(buf, i)?;
            let mut map = rustler::types::map::map_new(env);

            //Canonical check
            let mut prev_key_bytes: Option<&[u8]> = None;

            for _ in 0..count {
                let k_start = *i;
                let k = decode_term(env, buf, i)?;
                let k_bytes = &buf[k_start..*i];

                if let Some(prev) = prev_key_bytes {
                    if k_bytes <= prev { return Err(Error::Atom("map_not_canonical")); }
                }
                prev_key_bytes = Some(k_bytes);

                let v = decode_term(env, buf, i)?;

                //Put keys as atom if they exist in atom table
                if let Ok(bin) = Binary::from_term(k) {
                    if let Ok(Some(atom)) = Atom::try_from_bytes(env, bin.as_slice()) {
                        map = map.map_put(atom, v)?;
                    } else {
                        map = map.map_put(k, v)?;
                    }
                } else {
                    map = map.map_put(k, v)?;
                }
            }
            Ok(map)
        }
        _ => Err(Error::Atom("unknown_tag")),
    }
}

pub fn decode_term_from_slice<'a>(env: Env<'a>, buf: &[u8]) -> Result<Term<'a>, Error> {
    let mut i = 0;
    let term = decode_term(env, buf, &mut i)?;
    if i != buf.len() { return Err(Error::Atom("trailing_bytes")); }
    Ok(term)
}
