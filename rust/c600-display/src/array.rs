//! Array descriptors of RM.6: element type, shape, byte length and content digest. Numeric arrays are tightly packed
//! and little-endian with the last index contiguous; `f32` is IEEE binary32. Shape products are checked before any
//! use, and a descriptor that does not describe its bytes is `EV3:input.display_plan_invalid` (RM.12).

use std::sync::Arc;

use c600_digest::sha256_hex;

use crate::error::Error;

/// The element type of an array.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Elem {
    F32,
    U32,
    I32,
    U8,
}

impl Elem {
    pub fn size(self) -> u64 {
        match self {
            Elem::F32 | Elem::U32 | Elem::I32 => 4,
            Elem::U8 => 1,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Elem::F32 => "f32",
            Elem::U32 => "u32",
            Elem::I32 => "i32",
            Elem::U8 => "u8",
        }
    }
}

/// The descriptor of RM.6.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArrayDesc {
    pub elem: Elem,
    pub shape: Vec<u64>,
    pub byte_len: u64,
    /// The SHA-256 of the bytes.
    pub digest: String,
}

/// An immutable array: its descriptor and the bytes it describes, shared as an immutable lease (RM.18).
#[derive(Clone, Debug, PartialEq)]
pub struct Array {
    pub desc: ArrayDesc,
    pub bytes: Arc<Vec<u8>>,
}

impl Array {
    /// An array whose descriptor is computed from its bytes.
    pub fn from_bytes(elem: Elem, shape: Vec<u64>, bytes: Vec<u8>) -> Array {
        let desc = ArrayDesc { elem, shape, byte_len: bytes.len() as u64, digest: sha256_hex(&bytes) };
        Array { desc, bytes: Arc::new(bytes) }
    }

    pub fn u32s(shape: Vec<u64>, values: &[u32]) -> Array {
        Array::from_bytes(Elem::U32, shape, values.iter().flat_map(|v| v.to_le_bytes()).collect())
    }

    pub fn i32s(shape: Vec<u64>, values: &[i32]) -> Array {
        Array::from_bytes(Elem::I32, shape, values.iter().flat_map(|v| v.to_le_bytes()).collect())
    }

    pub fn f32s(shape: Vec<u64>, values: &[f32]) -> Array {
        Array::from_bytes(Elem::F32, shape, values.iter().flat_map(|v| v.to_le_bytes()).collect())
    }

    pub fn u8s(shape: Vec<u64>, values: &[u8]) -> Array {
        Array::from_bytes(Elem::U8, shape, values.to_vec())
    }

    /// Checks the descriptor against the bytes: the shape product, the byte length and the digest. Returns the
    /// element count.
    pub fn check(&self, what: &str) -> Result<u64, Error> {
        let elements = self
            .desc
            .shape
            .iter()
            .try_fold(1u64, |acc, &dim| acc.checked_mul(dim))
            .ok_or_else(|| Error::invalid(format!("{what}: the shape product overflows")))?;
        let bytes = elements
            .checked_mul(self.desc.elem.size())
            .ok_or_else(|| Error::invalid(format!("{what}: the byte length overflows")))?;
        if bytes != self.desc.byte_len || self.desc.byte_len != self.bytes.len() as u64 {
            return Err(Error::invalid(format!("{what}: the descriptor does not describe its bytes")));
        }
        if sha256_hex(&self.bytes) != self.desc.digest {
            return Err(Error::invalid(format!("{what}: the content digest does not match")));
        }
        Ok(elements)
    }

    /// Checks the descriptor, the element type and the shape; `None` leaves a dimension free.
    pub fn expect(&self, what: &str, elem: Elem, shape: &[Option<u64>]) -> Result<u64, Error> {
        let elements = self.check(what)?;
        if self.desc.elem != elem {
            return Err(Error::invalid(format!(
                "{what}: element type {} is not {}",
                self.desc.elem.name(),
                elem.name()
            )));
        }
        let fits = self.desc.shape.len() == shape.len()
            && self.desc.shape.iter().zip(shape).all(|(&have, want)| want.is_none_or(|want| want == have));
        if !fits {
            return Err(Error::invalid(format!("{what}: shape {:?} does not fit {shape:?}", self.desc.shape)));
        }
        Ok(elements)
    }

    /// The first dimension, or 0 for a scalar.
    pub fn rows(&self) -> u64 {
        self.desc.shape.first().copied().unwrap_or(0)
    }

    pub fn as_u32(&self) -> Vec<u32> {
        self.bytes.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
    }

    pub fn as_i32(&self) -> Vec<i32> {
        self.bytes.chunks_exact(4).map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
    }

    pub fn as_f32(&self) -> Vec<f32> {
        self.bytes.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
    }
}
