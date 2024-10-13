use core::str;
use std::fmt::Debug;

use bytes::Buf;
use pumpkin_protocol::{bytebuf::DeserializerError, FixedBitSet, VarInt, VarLongType};

pub mod client;
pub mod player;
pub mod resources;

const SEGMENT_BITS: u8 = 0x7F;
const CONTINUE_BIT: u8 = 0x80;

pub struct ReadByteBuffer<B: Buf> {
    buffer: B,
}

impl<B: Buf + Debug> Debug for ReadByteBuffer<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReadByteBuffer").field("buffer", &self.buffer).finish()
    }
}

impl<B: Buf + Clone> Clone for ReadByteBuffer<B> {
    fn clone(&self) -> Self {
        Self { buffer: self.buffer.clone() }
    }
}

impl<B: Buf> ReadByteBuffer<B> {
    pub fn new(buffer: B) -> Self {
        Self { buffer }
    }

    pub fn get_var_int(&mut self) -> Result<VarInt, DeserializerError> {
        let mut value: i32 = 0;
        let mut position: i32 = 0;

        loop {
            let read = self.get_u8()?;

            value |= ((read & SEGMENT_BITS) as i32) << position;

            if read & CONTINUE_BIT == 0 {
                break;
            }

            position += 7;

            if position >= 32 {
                return Err(DeserializerError::Message("VarInt is too big".to_string()));
            }
        }

        Ok(VarInt(value))
    }

    pub fn get_var_long(&mut self) -> Result<VarLongType, DeserializerError> {
        let mut value: i64 = 0;
        let mut position: i64 = 0;

        loop {
            let read = self.get_u8()?;

            value |= ((read & SEGMENT_BITS) as i64) << position;

            if read & CONTINUE_BIT == 0 {
                break;
            }

            position += 7;

            if position >= 64 {
                return Err(DeserializerError::Message("VarLong is too big".to_string()));
            }
        }

        Ok(value)
    }

    pub fn get_string(&mut self) -> Result<String, DeserializerError> {
        self.get_string_len(i16::MAX as i32)
    }

    pub fn get_string_len(&mut self, max_size: i32) -> Result<String, DeserializerError> {
        let size = self.get_var_int()?.0;
        if size > max_size {
            return Err(DeserializerError::Message(
                "String length is bigger than max size".to_string(),
            ));
        }

        let data = self.copy_to_bytes(size as usize)?;
        if data.len() as i32 > max_size {
            return Err(DeserializerError::Message(
                "String is bigger than max size".to_string(),
            ));
        }
        match str::from_utf8(&data) {
            Ok(string_result) => Ok(string_result.to_string()),
            Err(e) => Err(DeserializerError::Message(e.to_string())),
        }
    }

    pub fn get_bool(&mut self) -> Result<bool, DeserializerError> {
        Ok(self.get_u8()? != 0)
    }

    pub fn get_uuid(&mut self) -> Result<uuid::Uuid, DeserializerError> {
        let mut bytes = [0u8; 16];
        self.copy_to_slice(&mut bytes)?;
        Ok(uuid::Uuid::from_slice(&bytes).expect("Failed to parse UUID"))
    }

    pub fn get_fixed_bitset(&mut self, bits: usize) -> Result<FixedBitSet, DeserializerError> {
        self.copy_to_bytes(bits.div_ceil(8))
    }

    /// Reads a boolean. If true, the closure is called, and the returned value is
    /// wrapped in Some. Otherwise, this returns None.
    pub fn get_option<T>(
        &mut self,
        val: impl FnOnce(&mut Self) -> Result<T, DeserializerError>,
    ) -> Result<Option<T>, DeserializerError> {
        if self.get_bool()? {
            Ok(Some(val(self)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_list<T>(
        &mut self,
        val: impl Fn(&mut Self) -> Result<T, DeserializerError>,
    ) -> Result<Vec<T>, DeserializerError> {
        let len = self.get_var_int()?.0 as usize;
        let mut list = Vec::with_capacity(len);
        for _ in 0..len {
            list.push(val(self)?);
        }
        Ok(list)
    }
    
    pub fn buf(&mut self) -> &mut B {
        &mut self.buffer
    }

    // Trait equivalents
    pub fn get_u8(&mut self) -> Result<u8, DeserializerError> {
        if self.buffer.has_remaining() {
            Ok(self.buffer.get_u8())
        } else {
            Err(DeserializerError::Message(
                "No bytes left to consume".to_string(),
            ))
        }
    }

    pub fn get_i8(&mut self) -> Result<i8, DeserializerError> {
        if self.buffer.has_remaining() {
            Ok(self.buffer.get_i8())
        } else {
            Err(DeserializerError::Message(
                "No bytes left to consume".to_string(),
            ))
        }
    }

    pub fn get_u16(&mut self) -> Result<u16, DeserializerError> {
        if self.buffer.remaining() >= 2 {
            Ok(self.buffer.get_u16())
        } else {
            Err(DeserializerError::Message(
                "Less than 2 bytes left to consume".to_string(),
            ))
        }
    }

    pub fn get_i16(&mut self) -> Result<i16, DeserializerError> {
        if self.buffer.remaining() >= 2 {
            Ok(self.buffer.get_i16())
        } else {
            Err(DeserializerError::Message(
                "Less than 2 bytes left to consume".to_string(),
            ))
        }
    }

    pub fn get_u32(&mut self) -> Result<u32, DeserializerError> {
        if self.buffer.remaining() >= 4 {
            Ok(self.buffer.get_u32())
        } else {
            Err(DeserializerError::Message(
                "Less than 4 bytes left to consume".to_string(),
            ))
        }
    }

    pub fn get_i32(&mut self) -> Result<i32, DeserializerError> {
        if self.buffer.remaining() >= 4 {
            Ok(self.buffer.get_i32())
        } else {
            Err(DeserializerError::Message(
                "Less than 4 bytes left to consume".to_string(),
            ))
        }
    }

    pub fn get_u64(&mut self) -> Result<u64, DeserializerError> {
        if self.buffer.remaining() >= 8 {
            Ok(self.buffer.get_u64())
        } else {
            Err(DeserializerError::Message(
                "Less than 8 bytes left to consume".to_string(),
            ))
        }
    }

    pub fn get_i64(&mut self) -> Result<i64, DeserializerError> {
        if self.buffer.remaining() >= 8 {
            Ok(self.buffer.get_i64())
        } else {
            Err(DeserializerError::Message(
                "Less than 8 bytes left to consume".to_string(),
            ))
        }
    }

    pub fn get_f32(&mut self) -> Result<f32, DeserializerError> {
        if self.buffer.remaining() >= 4 {
            Ok(self.buffer.get_f32())
        } else {
            Err(DeserializerError::Message(
                "Less than 4 bytes left to consume".to_string(),
            ))
        }
    }

    pub fn get_f64(&mut self) -> Result<f64, DeserializerError> {
        if self.buffer.remaining() >= 8 {
            Ok(self.buffer.get_f64())
        } else {
            Err(DeserializerError::Message(
                "Less than 8 bytes left to consume".to_string(),
            ))
        }
    }

    pub fn copy_to_bytes(&mut self, len: usize) -> Result<bytes::Bytes, DeserializerError> {
        if self.buffer.remaining() >= len {
            Ok(self.buffer.copy_to_bytes(len))
        } else {
            Err(DeserializerError::Message(
                "Unable to copy bytes".to_string(),
            ))
        }
    }

    pub fn copy_to_slice(&mut self, dst: &mut [u8]) -> Result<(), DeserializerError> {
        if self.buffer.remaining() >= dst.len() {
            self.buffer.copy_to_slice(dst);
            Ok(())
        } else {
            Err(DeserializerError::Message(
                "Unable to copy slice".to_string(),
            ))
        }
    }

}
