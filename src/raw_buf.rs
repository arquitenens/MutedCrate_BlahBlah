use std::fmt::{Binary, Debug, Display, Formatter};
use std::marker::PhantomData;
use core::default::Default;
use std::{fmt, slice, vec};
use std::any::{type_name, type_name_of_val, TypeId};

#[derive(Debug, Eq, PartialEq)]
pub struct RawBuf {
    pub data: Vec<u8>,
    pub last_index: u32,
    pub len: u32,
}


#[derive(Debug)]
pub enum BIT {
    One,
    Zero,
}

pub struct BYTE(u64);

pub enum offset{
    Bit(u64),
    Byte(u64),
}

#[derive(Debug)]
pub enum PrivateTypes {
    U32(u32),
    U64(u64),
    U16(u16),
    U8(u8),
    I32(i32),
    I64(i64),
    I16(i16),
    I8(i8),
}

impl Binary for RawBuf {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut bytes: Vec<u8> = Vec::with_capacity((self.len * 8) as usize);

        for byte in self.data.iter() {
            for bit in 0..8 {
                let mask = 1 << (7 - bit);
                let bin = if (byte & mask) != 0 { 1 } else { 0 };
                bytes.push(bin);
            }
        }
        if f.alternate() {
            let mut s = String::new();
            for (i, &b) in bytes.iter().enumerate() {
                s.push(if b == 1 { '1' } else { '0' });
                if (i + 1) % 8 == 0 {
                    s.push('\n');
                }
            }
            return write!(f, "{}", s);
        }

        write!(f, "{:?}", bytes)
    }
}

impl RawBuf {
    pub fn read_bit(&self, bit_offset: u32) -> u8 {
        let offset = (bit_offset % 8) as u8;
        let byte_index = (bit_offset / 8) as usize;
        let bit_in_byte = 7 - offset;
        return (self.data[byte_index] >> bit_in_byte) & 1;
    }

    pub fn write_bit(&mut self, bit_offset: offset, bit: BIT, append_mode: bool) {
        let bit_offset = match bit_offset {
            offset::Bit(x) => x,
            _ => panic!("only bits"),
        } as u32;

        let bit_offset = if append_mode { self.last_index } else { bit_offset };
        //println!("bit offset: {}, len: {}", bit_offset, self.len);
        if bit_offset > self.len {
            panic!("bit offset out of bounds");
        }
        let offset = (bit_offset % 8) as u8;
        let byte_index = (bit_offset / 8) as usize;
        let bit_in_byte = 7 - offset;
        let mask = 1 << bit_in_byte;
        //println!("mask {}", mask);
        match bit {
            BIT::One => self.data[byte_index] |= mask,
            BIT::Zero => self.data[byte_index] &= !mask,
        }
        //println!("bit: {:?}, bit_offset: {}, byte_offset: {}", bit, bit_in_byte, byte_index);
        self.last_index += 1;
        //println!("last_index: {}", self.last_index);
    }

    pub fn read_bits<T>(&self, from: u32, until: u32, last: bool, read_as_type: T) -> Result<PrivateTypes, String> {
        let from = if last { self.last_index } else { from };

        //println!("from {:?}", from);
        //println!("until {:?}", until);
        let mut num: Vec<u8> = Vec::with_capacity(((until - from) * 8) as usize);
        for i in from..until {
            num.push(self.read_bit(i));
        }
        let s = num.iter().map(|&b| if b == 1 { '1' } else { '0' }).collect::<String>();
        let t = match type_name_of_val(&read_as_type) {
            "u32" => u32::from_str_radix(s.as_str(), 2).map(PrivateTypes::U32),
            "u64" => u64::from_str_radix(s.as_str(), 2).map(PrivateTypes::U64),
            "u16" => u16::from_str_radix(s.as_str(), 2).map(PrivateTypes::U16),
            "u8" => u8::from_str_radix(s.as_str(), 2).map(PrivateTypes::U8),

            "i32" => i32::from_str_radix(s.as_str(), 2).map(PrivateTypes::I32),
            "i64" => i64::from_str_radix(s.as_str(), 2).map(PrivateTypes::I64),
            "i16" => i16::from_str_radix(s.as_str(), 2).map(PrivateTypes::I16),
            "i8" => i8::from_str_radix(s.as_str(), 2).map(PrivateTypes::I8),
            _ => panic!("not supported type!")
        };
        return Ok(t.unwrap())
    }

    pub fn write_bits(&mut self, bit_offset: offset, write: u64, bit_count: u32, append_mode: bool) {

        let bit_offset = match bit_offset {
            offset::Bit(x) => x,
            offset::Byte(x) => x * 8,
        } as u32;

        let bit_offset = if append_mode { self.last_index } else { bit_offset };
        if !(bit_offset <= self.len) {
            panic!("bit offset out of bounds");
        }
        let binary_digits = (0..bit_count).map(|n| ((write >> (bit_count - 1 - n)) & 1) as u8).collect::<Vec<u8>>();
        //println!("binary digits: {:?}", binary_digits);
        for i in 0..bit_count {
            self.write_bit(offset::Bit((bit_offset + i) as u64), match binary_digits[i as usize]{
                1 => BIT::One,
                0 => BIT::Zero,
                _ => unreachable!()
            }, append_mode);
        }
        let total_bits = bit_count + bit_offset;
        self.last_index = total_bits;
        //println!("Writing: {} to {} in append_mode = {}, last_index: {}", write, bit_offset, append_mode, self.last_index);

    }
    
    pub fn extend_by(&mut self, bytes: usize) {
        let new_size = self.data.len() + bytes;
        self.data.resize(new_size, 0);
    }

    pub fn new<'a>(byte_size: u32) -> RawBuf{
        return RawBuf{data: vec![0; byte_size as usize], len: byte_size * 8, last_index: 0};
    }

}
