#![deny(unreachable_pub)]
#![deny(warnings)]
// Proptest derive triggers this.
#![allow(non_local_definitions)]

pub mod coin;
pub mod contract;
mod fab;
pub mod transfer;

macro_rules! hash_serde {
    ($($ty:ident),*) => {
        $(
            #[cfg(feature = "serde")]
            impl serde::Serialize for $ty {
                fn serialize<S: serde::ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    serializer.serialize_bytes(&self.0.0)
                }
            }

            #[cfg(feature = "serde")]
            impl<'de> serde::Deserialize<'de> for $ty {
                fn deserialize<D: serde::de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                    deserializer.deserialize_bytes(crate::HashVisitor).map($ty)
                }
            }
        )*
    }
}
pub(crate) use hash_serde;

#[cfg(feature = "serde")]
pub(crate) struct HashVisitor;

#[cfg(feature = "serde")]
impl serde::de::Visitor<'_> for HashVisitor {
    type Value = base_crypto::hash::HashOutput;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a hash value")
    }

    fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
        let mut res = [0u8; base_crypto::hash::PERSISTENT_HASH_BYTES];
        if v.len() == res.len() {
            res.copy_from_slice(v);
            Ok(base_crypto::hash::HashOutput(res))
        } else {
            Err(E::invalid_length(v.len(), &self))
        }
    }
}

/// Re-export of `storage` lib.
pub use storage;
/// Re-export of `base_crypto` lib.
pub use storage::base_crypto;
/// Re-export of `serialize` lib.
pub use storage::serialize;
/// Re-export of `transient_crypto` lib.
pub use transient_crypto;
