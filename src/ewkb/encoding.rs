use crate::error::Error;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::io::Read;

pub fn read_u32<R: Read>(raw: &mut R, is_be: bool) -> Result<u32, Error> {
    Ok(if is_be {
        raw.read_u32::<BigEndian>()?
    } else {
        raw.read_u32::<LittleEndian>()?
    })
}

pub fn read_i32<R: Read>(raw: &mut R, is_be: bool) -> Result<i32, Error> {
    Ok(if is_be {
        raw.read_i32::<BigEndian>()?
    } else {
        raw.read_i32::<LittleEndian>()?
    })
}

pub fn read_f64<R: Read>(raw: &mut R, is_be: bool) -> Result<f64, Error> {
    Ok(if is_be {
        raw.read_f64::<BigEndian>()?
    } else {
        raw.read_f64::<LittleEndian>()?
    })
}
