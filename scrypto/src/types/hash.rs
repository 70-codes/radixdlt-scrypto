extern crate alloc;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use core::convert::TryInto;
use core::fmt;

use sbor::{Decode, Encode};

use crate::utils::*;

/// Represents a 32-byte hash digest.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Encode, Decode)]
pub struct Hash {
    raw: [u8; 32],
}

impl Hash {
    /// Decode a hash from its hex representation.
    pub fn from_hex(hex: &str) -> Result<Self, String> {
        let data = hex_decode(hex)?;
        Ok(Self {
            raw: data
                .try_into()
                .map_err(|_| format!("Unable to parse hash from hex: {}", hex))?,
        })
    }

    /// Create hash struct from a slice.
    pub fn from_slice(slice: &[u8]) -> Result<Self, String> {
        Ok(Self {
            raw: slice
                .try_into()
                .map_err(|_| "Unable to parse hash from slice".to_string())?,
        })
    }

    /// Returns the first 26 bytes.
    pub fn lower_26_bytes(&self) -> [u8; 26] {
        let mut result = [0u8; 26];
        result.copy_from_slice(&self.raw[6..32]);
        result
    }
}

impl<T: AsRef<str>> From<T> for Hash {
    fn from(s: T) -> Self {
        Hash::from_hex(s.as_ref()).unwrap()
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self.raw
    }
}

impl ToString for Hash {
    fn to_string(&self) -> String {
        hex_encode(self.as_ref())
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use alloc::string::ToString;

    use crate::types::Hash;

    #[test]
    fn test_from_to_string() {
        let s = "b177968c9c68877dc8d33e25759183c556379daa45a4d78a2b91c70133c873ca";
        let h: Hash = s.into();
        assert_eq!(h.to_string(), s);
    }
}
