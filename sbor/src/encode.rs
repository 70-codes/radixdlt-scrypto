use crate::rust::borrow::Cow;
use crate::rust::borrow::ToOwned;
use crate::rust::boxed::Box;
use crate::rust::cell::RefCell;
use crate::rust::collections::*;
use crate::rust::hash::Hash;
use crate::rust::ptr::copy;
use crate::rust::string::String;
use crate::rust::vec::Vec;
use crate::type_id::*;

/// A data structure that can be serialized into a byte array using SBOR.
pub trait Encode {
    fn encode(&self, encoder: &mut Encoder) {
        Self::encode_type_id(encoder);
        self.encode_value(encoder);
    }

    fn encode_type_id(encoder: &mut Encoder);

    fn encode_value(&self, encoder: &mut Encoder);
}

/// An `Encoder` abstracts the logic for writing core types into a byte buffer.
pub struct Encoder<'a> {
    buf: &'a mut Vec<u8>,
    with_static_info: bool,
}

impl<'a> Encoder<'a> {
    pub fn new(buf: &'a mut Vec<u8>, with_static_info: bool) -> Self {
        Self {
            buf,
            with_static_info,
        }
    }

    pub fn with_static_info(buf: &'a mut Vec<u8>) -> Self {
        Self::new(buf, true)
    }

    pub fn no_static_info(buf: &'a mut Vec<u8>) -> Self {
        Self::new(buf, false)
    }

    pub fn write_type_id(&mut self, ty: u8) {
        // May use compile-time feature flag, instead of runtime check, for performance.
        if self.with_static_info {
            self.buf.push(ty);
        }
    }

    pub fn write_variant_index(&mut self, index: u8) {
        self.write_byte(index);
    }

    pub fn write_variant_label(&mut self, label: &str) {
        self.write_dynamic_size(label.len());
        self.write_slice(label.as_bytes());
    }

    pub fn write_static_size(&mut self, len: usize) {
        // May use compile-time feature flag, instead of runtime check, for performance.
        if self.with_static_info {
            self.buf.extend(&(len as u32).to_le_bytes());
        }
    }

    pub fn write_dynamic_size(&mut self, len: usize) {
        self.buf.extend(&(len as u32).to_le_bytes());
    }

    pub fn write_byte(&mut self, n: u8) {
        self.buf.push(n);
    }

    pub fn write_slice(&mut self, slice: &[u8]) {
        self.buf.extend(slice);
    }

    pub fn encode<T: Encode + ?Sized>(&mut self, value: &T) {
        value.encode(self)
    }
}

impl Encode for () {
    #[inline]
    fn encode_type_id(encoder: &mut Encoder) {
        encoder.write_type_id(Self::type_id());
    }
    #[inline]
    fn encode_value(&self, encoder: &mut Encoder) {
        encoder.write_byte(0);
    }
}

impl Encode for bool {
    #[inline]
    fn encode_type_id(encoder: &mut Encoder) {
        encoder.write_type_id(Self::type_id());
    }
    #[inline]
    fn encode_value(&self, encoder: &mut Encoder) {
        encoder.write_byte(if *self { 1u8 } else { 0u8 });
    }
}

impl Encode for i8 {
    #[inline]
    fn encode_type_id(encoder: &mut Encoder) {
        encoder.write_type_id(Self::type_id());
    }
    #[inline]
    fn encode_value(&self, encoder: &mut Encoder) {
        encoder.write_byte(*self as u8);
    }
}

impl Encode for u8 {
    #[inline]
    fn encode_type_id(encoder: &mut Encoder) {
        encoder.write_type_id(Self::type_id());
    }
    #[inline]
    fn encode_value(&self, encoder: &mut Encoder) {
        encoder.write_byte(*self);
    }
}

macro_rules! encode_int {
    ($type:ident, $type_id:ident) => {
        impl Encode for $type {
            #[inline]
            fn encode_type_id(encoder: &mut Encoder) {
                encoder.write_type_id(Self::type_id());
            }
            #[inline]
            fn encode_value(&self, encoder: &mut Encoder) {
                encoder.write_slice(&(*self).to_le_bytes());
            }
        }
    };
}

encode_int!(i16, TYPE_I16);
encode_int!(i32, TYPE_I32);
encode_int!(i64, TYPE_I64);
encode_int!(i128, TYPE_I128);
encode_int!(u16, TYPE_U16);
encode_int!(u32, TYPE_U32);
encode_int!(u64, TYPE_U64);
encode_int!(u128, TYPE_U128);

impl Encode for isize {
    #[inline]
    fn encode_type_id(encoder: &mut Encoder) {
        encoder.write_type_id(Self::type_id());
    }
    #[inline]
    fn encode_value(&self, encoder: &mut Encoder) {
        (*self as i64).encode_value(encoder);
    }
}

impl Encode for usize {
    #[inline]
    fn encode_type_id(encoder: &mut Encoder) {
        encoder.write_type_id(Self::type_id());
    }
    #[inline]
    fn encode_value(&self, encoder: &mut Encoder) {
        (*self as u64).encode_value(encoder);
    }
}

impl Encode for str {
    #[inline]
    fn encode_type_id(encoder: &mut Encoder) {
        encoder.write_type_id(Self::type_id());
    }
    #[inline]
    fn encode_value(&self, encoder: &mut Encoder) {
        encoder.write_dynamic_size(self.len());
        encoder.write_slice(self.as_bytes());
    }
}

impl Encode for &str {
    #[inline]
    fn encode_type_id(encoder: &mut Encoder) {
        encoder.write_type_id(Self::type_id());
    }
    #[inline]
    fn encode_value(&self, encoder: &mut Encoder) {
        encoder.write_dynamic_size(self.len());
        encoder.write_slice(self.as_bytes());
    }
}

impl Encode for String {
    #[inline]
    fn encode_type_id(encoder: &mut Encoder) {
        encoder.write_type_id(Self::type_id());
    }
    #[inline]
    fn encode_value(&self, encoder: &mut Encoder) {
        self.as_str().encode_value(encoder);
    }
}

impl<T: Encode + TypeId> Encode for Option<T> {
    #[inline]
    fn encode_type_id(encoder: &mut Encoder) {
        encoder.write_type_id(Self::type_id());
    }
    #[inline]
    fn encode_value(&self, encoder: &mut Encoder) {
        match self {
            Some(v) => {
                encoder.write_variant_index(OPTION_VARIANT_SOME);
                v.encode(encoder);
            }
            None => {
                encoder.write_variant_index(OPTION_VARIANT_NONE);
            }
        }
    }
}

impl<'a, B: ?Sized + 'a + ToOwned + Encode> Encode for Cow<'a, B> {
    #[inline]
    fn encode_type_id(encoder: &mut Encoder) {
        B::encode_type_id(encoder)
    }
    #[inline]
    fn encode_value(&self, encoder: &mut Encoder) {
        self.as_ref().encode_value(encoder);
    }
}

impl<T: Encode> Encode for Box<T> {
    #[inline]
    fn encode_type_id(encoder: &mut Encoder) {
        T::encode_type_id(encoder)
    }
    #[inline]
    fn encode_value(&self, encoder: &mut Encoder) {
        self.as_ref().encode_value(encoder);
    }
}

impl<T: Encode> Encode for RefCell<T> {
    #[inline]
    fn encode_type_id(encoder: &mut Encoder) {
        T::encode_type_id(encoder)
    }
    #[inline]
    fn encode_value(&self, encoder: &mut Encoder) {
        self.borrow().encode_value(encoder);
    }
}

impl<T: Encode + TypeId, const N: usize> Encode for [T; N] {
    #[inline]
    fn encode_type_id(encoder: &mut Encoder) {
        encoder.write_type_id(Self::type_id());
    }
    #[inline]
    fn encode_value(&self, encoder: &mut Encoder) {
        encoder.write_type_id(T::type_id());
        encoder.write_static_size(self.len());
        for v in self {
            v.encode_value(encoder);
        }
    }
}

macro_rules! encode_tuple {
    ($n:tt $($idx:tt $name:ident)+) => {
        impl<$($name: Encode),+> Encode for ($($name,)+) {
            #[inline]
            fn encode_type_id(encoder: &mut Encoder) {
                encoder.write_type_id(Self::type_id());
            }
            #[inline]
            fn encode_value(&self, encoder: &mut Encoder) {
                encoder.write_static_size($n);

                $(self.$idx.encode(encoder);)+
            }
        }
    };
}

encode_tuple! { 2 0 A 1 B }
encode_tuple! { 3 0 A 1 B 2 C }
encode_tuple! { 4 0 A 1 B 2 C 3 D }
encode_tuple! { 5 0 A 1 B 2 C 3 D 4 E }
encode_tuple! { 6 0 A 1 B 2 C 3 D 4 E 5 F }
encode_tuple! { 7 0 A 1 B 2 C 3 D 4 E 5 F 6 G }
encode_tuple! { 8 0 A 1 B 2 C 3 D 4 E 5 F 6 G 7 H }
encode_tuple! { 9 0 A 1 B 2 C 3 D 4 E 5 F 6 G 7 H 8 I }
encode_tuple! { 10 0 A 1 B 2 C 3 D 4 E 5 F 6 G 7 H 8 I 9 J }

impl<T: Encode, E: Encode> Encode for Result<T, E> {
    #[inline]
    fn encode_type_id(encoder: &mut Encoder) {
        encoder.write_type_id(Self::type_id());
    }
    #[inline]
    fn encode_value(&self, encoder: &mut Encoder) {
        match self {
            Ok(o) => {
                encoder.write_variant_index(RESULT_VARIANT_OK);
                o.encode(encoder);
            }
            Err(e) => {
                encoder.write_variant_index(RESULT_VARIANT_ERR);
                e.encode(encoder);
            }
        }
    }
}

impl<T: Encode + TypeId> Encode for Vec<T> {
    #[inline]
    fn encode_type_id(encoder: &mut Encoder) {
        encoder.write_type_id(Self::type_id());
    }
    #[inline]
    fn encode_value(&self, encoder: &mut Encoder) {
        self.as_slice().encode_value(encoder);
    }
}

impl<T: Encode + TypeId> Encode for [T] {
    #[inline]
    fn encode_type_id(encoder: &mut Encoder) {
        encoder.write_type_id(Self::type_id());
    }
    #[inline]
    fn encode_value(&self, encoder: &mut Encoder) {
        encoder.write_type_id(T::type_id());
        encoder.write_dynamic_size(self.len());
        if T::type_id() == TYPE_U8 || T::type_id() == TYPE_I8 {
            let mut buf = Vec::<u8>::with_capacity(self.len());
            unsafe {
                copy(self.as_ptr() as *mut u8, buf.as_mut_ptr(), self.len());
                buf.set_len(self.len());
            }
            encoder.write_slice(&buf);
        } else {
            for v in self {
                v.encode_value(encoder);
            }
        }
    }
}

impl<T: Encode + TypeId> Encode for BTreeSet<T> {
    #[inline]
    fn encode_type_id(encoder: &mut Encoder) {
        encoder.write_type_id(Self::type_id());
    }
    #[inline]
    fn encode_value(&self, encoder: &mut Encoder) {
        encoder.write_type_id(T::type_id());
        encoder.write_dynamic_size(self.len());
        for v in self {
            v.encode_value(encoder);
        }
    }
}

impl<K: Encode + TypeId, V: Encode + TypeId> Encode for BTreeMap<K, V> {
    #[inline]
    fn encode_type_id(encoder: &mut Encoder) {
        encoder.write_type_id(Self::type_id());
    }
    #[inline]
    fn encode_value(&self, encoder: &mut Encoder) {
        encoder.write_type_id(K::type_id());
        encoder.write_type_id(V::type_id());
        encoder.write_dynamic_size(self.len());
        for (k, v) in self {
            k.encode_value(encoder);
            v.encode_value(encoder);
        }
    }
}

impl<T: Encode + TypeId + Ord + Hash> Encode for HashSet<T> {
    #[inline]
    fn encode_type_id(encoder: &mut Encoder) {
        encoder.write_type_id(Self::type_id());
    }
    #[inline]
    fn encode_value(&self, encoder: &mut Encoder) {
        encoder.write_type_id(T::type_id());
        encoder.write_dynamic_size(self.len());
        // Encode elements based on the order defined on the key type to generate deterministic bytes.
        let values: BTreeSet<&T> = self.iter().collect();
        for v in values {
            v.encode_value(encoder);
        }
    }
}

impl<K: Encode + TypeId + Ord + Hash, V: Encode + TypeId> Encode for HashMap<K, V> {
    #[inline]
    fn encode_type_id(encoder: &mut Encoder) {
        encoder.write_type_id(Self::type_id());
    }
    #[inline]
    fn encode_value(&self, encoder: &mut Encoder) {
        encoder.write_type_id(K::type_id());
        encoder.write_type_id(V::type_id());
        encoder.write_dynamic_size(self.len());
        // Encode elements based on the order defined on the key type to generate deterministic bytes.
        let keys: BTreeSet<&K> = self.keys().collect();
        for key in keys {
            key.encode_value(encoder);
            self.get(key).unwrap().encode_value(encoder);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rust::borrow::ToOwned;
    use crate::rust::vec;

    fn do_encoding(enc: &mut Encoder) {
        ().encode(enc);
        true.encode(enc);
        1i8.encode(enc);
        1i16.encode(enc);
        1i32.encode(enc);
        1i64.encode(enc);
        1i128.encode(enc);
        1u8.encode(enc);
        1u16.encode(enc);
        1u32.encode(enc);
        1u64.encode(enc);
        1u128.encode(enc);
        "hello".encode(enc);

        Some(1u32).encode(enc);
        Option::<u32>::None.encode(enc);
        Result::<u32, String>::Ok(1u32).encode(enc);
        Result::<u32, String>::Err("hello".to_owned()).encode(enc);

        [1u32, 2u32, 3u32].encode(enc);
        (1u32, 2u32).encode(enc);

        vec![1u32, 2u32, 3u32].encode(enc);
        let mut set = BTreeSet::<u8>::new();
        set.insert(1);
        set.insert(2);
        set.encode(enc);
        let mut map = BTreeMap::<u8, u8>::new();
        map.insert(1, 2);
        map.insert(3, 4);
        map.encode(enc);
    }

    #[test]
    pub fn test_encoding() {
        let mut bytes = Vec::with_capacity(512);
        let mut enc = Encoder::with_static_info(&mut bytes);
        do_encoding(&mut enc);

        assert_eq!(
            vec![
                0, 0, // unit
                1, 1, // bool
                2, 1, // i8
                3, 1, 0, // i16
                4, 1, 0, 0, 0, // i32
                5, 1, 0, 0, 0, 0, 0, 0, 0, // i64
                6, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // i128
                7, 1, // u8
                8, 1, 0, // u16
                9, 1, 0, 0, 0, // u32
                10, 1, 0, 0, 0, 0, 0, 0, 0, // u64
                11, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // u128
                12, 5, 0, 0, 0, 104, 101, 108, 108, 111, // string
                18, 0, 9, 1, 0, 0, 0, // option
                18, 1, // option
                19, 0, 9, 1, 0, 0, 0, // result
                19, 1, 12, 5, 0, 0, 0, 104, 101, 108, 108, 111, // result
                32, 9, 3, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, // array
                33, 2, 0, 0, 0, 9, 1, 0, 0, 0, 9, 2, 0, 0, 0, // tuple
                48, 9, 3, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, // list
                49, 7, 2, 0, 0, 0, 1, 2, // set
                50, 7, 7, 2, 0, 0, 0, 1, 2, 3, 4 // map
            ],
            bytes
        );
    }

    #[test]
    pub fn test_encoding_no_static_info() {
        let mut bytes = Vec::with_capacity(512);
        let mut enc = Encoder::no_static_info(&mut bytes);
        do_encoding(&mut enc);

        assert_eq!(
            vec![
                0, // unit
                1, // bool
                1, // i8
                1, 0, // i16
                1, 0, 0, 0, // i32
                1, 0, 0, 0, 0, 0, 0, 0, // i64
                1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // i128
                1, // u8
                1, 0, // u16
                1, 0, 0, 0, // u32
                1, 0, 0, 0, 0, 0, 0, 0, // u64
                1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // u128
                5, 0, 0, 0, 104, 101, 108, 108, 111, // string
                0, 1, 0, 0, 0, // option
                1, // option
                0, 1, 0, 0, 0, // result
                1, 5, 0, 0, 0, 104, 101, 108, 108, 111, // result
                1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, // array
                1, 0, 0, 0, 2, 0, 0, 0, // tuple
                3, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, // list
                2, 0, 0, 0, 1, 2, // set
                2, 0, 0, 0, 1, 2, 3, 4 // map
            ],
            bytes
        );
    }

    #[test]
    pub fn test_encode_box() {
        let x = Box::new(5u8);
        let mut bytes = Vec::with_capacity(512);
        let mut enc = Encoder::with_static_info(&mut bytes);
        x.encode(&mut enc);
        assert_eq!(bytes, vec![7, 5])
    }

    #[test]
    pub fn test_encode_rc() {
        let x = crate::rust::rc::Rc::new(5u8);
        let mut bytes = Vec::with_capacity(512);
        let mut enc = Encoder::with_static_info(&mut bytes);
        x.encode(&mut enc);
        assert_eq!(bytes, vec![7, 5])
    }

    #[test]
    pub fn test_encode_ref_cell() {
        let x = crate::rust::cell::RefCell::new(5u8);
        let mut bytes = Vec::with_capacity(512);
        let mut enc = Encoder::with_static_info(&mut bytes);
        x.encode(&mut enc);
        assert_eq!(bytes, vec![7, 5])
    }
}
