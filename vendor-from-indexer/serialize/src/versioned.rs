use borsh::{BorshDeserialize, BorshSerialize};
use konst::{
    cmp::{ConstCmp, IsNotStdKind},
    const_cmp, const_eq, try_equal,
};
#[cfg(feature = "proptest")]
use proptest_derive::Arbitrary;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    time::{Duration, SystemTime},
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "proptest", derive(Arbitrary))]
// TODO WG needing all the serialization, deserialization, storage and versioned instances for this seems...weird. Maybe it's fine. Think about it later.
pub struct Version {
    pub major: u8,
    pub minor: u8,
}

pub trait Versioned {
    const VERSION: Option<Version>;
    const NETWORK_SPECIFIC: bool = Self::VERSION.is_some();
    const LIMIT_RECURSION: bool = false;
}

impl Versioned for Version {
    const VERSION: Option<Version> = None;
    const NETWORK_SPECIFIC: bool = false;
}

#[cfg(feature = "proptest")]
impl rand::distributions::Distribution<Version> for rand::distributions::Standard {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Version {
        Version {
            major: rng.gen(),
            minor: rng.gen(),
        }
    }
}

impl<T: Versioned> Versioned for Vec<T> {
    const VERSION: Option<Version> = None;
    const NETWORK_SPECIFIC: bool = T::NETWORK_SPECIFIC;
}

impl<K: Versioned, V: Versioned> Versioned for HashMap<K, V> {
    const VERSION: Option<Version> = None;
    const NETWORK_SPECIFIC: bool = K::NETWORK_SPECIFIC || V::NETWORK_SPECIFIC;
}

impl<T: Versioned> Versioned for HashSet<T> {
    const VERSION: Option<Version> = None;
    const NETWORK_SPECIFIC: bool = T::NETWORK_SPECIFIC;
}

impl<'a, T> Versioned for &'a T
where
    T: Versioned + 'a,
{
    const VERSION: Option<Version> = None;
    const NETWORK_SPECIFIC: bool = T::NETWORK_SPECIFIC;
}

impl<T: Versioned> Versioned for Option<T> {
    const VERSION: Option<Version> = None;
    const NETWORK_SPECIFIC: bool = T::NETWORK_SPECIFIC;
}

impl Versioned for &str {
    const VERSION: Option<Version> = None;
}

impl Versioned for String {
    const VERSION: Option<Version> = None;
}

impl<T: Versioned> Versioned for Arc<T> {
    const VERSION: Option<Version> = None;
    const NETWORK_SPECIFIC: bool = T::NETWORK_SPECIFIC;
}

impl Versioned for SystemTime {
    const VERSION: Option<Version> = None;
}

impl Versioned for Duration {
    const VERSION: Option<Version> = None;
}

impl ConstCmp for Version {
    type Kind = IsNotStdKind;
}

impl Version {
    pub const fn const_eq(&self, other: &Self) -> bool {
        const_eq!(self.major, other.major) && const_eq!(self.minor, other.minor)
    }

    pub const fn const_cmp(&self, other: &Self) -> Ordering {
        try_equal!(const_cmp!(self.major, other.major));
        try_equal!(const_cmp!(self.minor, other.minor))
    }
}
