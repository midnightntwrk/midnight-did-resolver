use std::marker::PhantomData;

use midnight_ledger_v4::base_crypto::fab::{Value, ValueAtom};
use midnight_ledger_v4::onchain_runtime::state::StateValue;
use midnight_ledger_v4::storage::db::DB;
use midnight_ledger_v4::transient_crypto::curve;

#[derive(Debug, derive_more::From, derive_more::Display, derive_more::Error)]
#[display("{message}")]
pub struct CompactError {
    #[from]
    pub message: String,
}

pub trait AdtType: Sized {
    fn from_state<D: DB>(value: &StateValue<D>) -> Result<Self, CompactError>;
}

pub trait CompactType: Sized {
    fn from_value(value: &mut Value) -> Result<Self, CompactError>;
}

pub trait StateValuePath: Sized {
    fn state_value_path() -> &'static [u8];
}

pub struct EmptyPath;

impl StateValuePath for EmptyPath {
    fn state_value_path() -> &'static [u8] {
        &[]
    }
}

pub struct AdtTypeCell<T: CompactType, P: StateValuePath = EmptyPath>(pub T, PhantomData<P>);
pub struct AdtTypeSet<T: CompactType, P: StateValuePath = EmptyPath>(pub Vec<T>, PhantomData<P>);
pub struct AdtTypeMap<K: CompactType, V: AdtType, P: StateValuePath = EmptyPath>(pub Vec<(K, V)>, PhantomData<P>);

pub struct BigInt(pub Vec<u8>);
pub struct CompactTypeBoolean(pub bool);
pub struct CompactTypeBytes<const N: usize>(pub [u8; N]);
pub struct CompactTypeOpaqueString(pub String);
pub struct CompactTypeUnsignedInteger(pub BigInt);
pub struct CompactTypeField(pub BigInt);
pub struct CompactTypeEnum(pub u8);
pub struct CompactTypeVector<const N: usize, T>(pub [T; N]);

fn state_value_getter<'a, 'b, D: DB>(
    value: &'a StateValue<D>,
    path: &'b [u8],
) -> Result<&'a StateValue<D>, CompactError> {
    let mut current = value;
    for idx in path {
        let maybe_child = match value {
            StateValue::Array(array) => array.get(usize::from(*idx)),
            _ => Err(format!("expected StateValue::Array on path {:?}", path))?,
        };
        match maybe_child {
            Some(child) => current = child,
            None => Err(format!("expected StateValue on path {:?}", path))?,
        }
    }
    Ok(current)
}

impl<T: CompactType> AdtType for T {
    fn from_state<D: DB>(value: &StateValue<D>) -> Result<Self, CompactError> {
        match value {
            StateValue::Cell(aligned_value) => Ok(T::from_value(&mut aligned_value.value.clone())?),
            _ => Err(format!("expected State of to be cell"))?,
        }
    }
}

impl<T: CompactType, P: StateValuePath> AdtType for AdtTypeCell<T, P> {
    fn from_state<D: DB>(value: &StateValue<D>) -> Result<Self, CompactError> {
        let path = P::state_value_path();
        let state_value = state_value_getter(value, &path)?;
        match state_value {
            StateValue::Cell(aligned_value) => {
                let mut value = aligned_value.value.clone();
                return Ok(Self(T::from_value(&mut value)?, PhantomData));
            }
            _ => Err(format!("expected StateValue on path {:?} to be cell", path))?,
        }
    }
}

impl<T: CompactType, P: StateValuePath> AdtType for AdtTypeSet<T, P> {
    fn from_state<D: DB>(value: &StateValue<D>) -> Result<Self, CompactError> {
        let path = P::state_value_path();
        let state_value = state_value_getter(value, &path)?;
        match state_value {
            StateValue::Map(map) => {
                let keys = map
                    .iter()
                    .map(|i| T::from_value(&mut i.0.value.clone()))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Self(keys, PhantomData))
            }
            _ => Err(format!("expected StateValue on path {:?} to be cell", path))?,
        }
    }
}

impl<K: CompactType, V: AdtType, P: StateValuePath> AdtType for AdtTypeMap<K, V, P> {
    fn from_state<D: DB>(value: &StateValue<D>) -> Result<Self, CompactError> {
        let path = P::state_value_path();
        let state_value = state_value_getter(value, &path)?;
        match state_value {
            StateValue::Map(map) => {
                let keys = map
                    .iter()
                    .map(|i| {
                        let key = K::from_value(&mut i.0.value.clone());
                        let value = V::from_state(&&i.1);
                        key.and_then(|k| value.map(|v| (k, v)))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Self(keys, PhantomData))
            }
            _ => Err(format!("expected StateValue on path {:?} to be cell", path))?,
        }
    }
}

impl CompactType for CompactTypeBoolean {
    fn from_value(value: &mut Value) -> Result<Self, CompactError> {
        let maybe_val = value.0.pop().map(|i| i.0);
        let Some(val) = maybe_val else {
            Err("expected Boolean".to_string())?
        };
        if val.len() > 1 || (val.len() == 1 && val[0] != 1) {
            Err("expected Boolean".to_string())?
        }
        Ok(Self(val.len() == 1))
    }
}

impl<const N: usize> CompactType for CompactTypeBytes<N> {
    fn from_value(value: &mut Value) -> Result<Self, CompactError> {
        let maybe_val = value.0.pop().map(|i| i.0);
        let Some(val) = maybe_val else {
            Err(format!("expected Bytes[{N}]"))?
        };
        let Ok(array) = val.try_into() else {
            Err(format!("expected Bytes[{N}]"))?
        };
        Ok(Self(array))
    }
}

impl CompactType for CompactTypeOpaqueString {
    fn from_value(value: &mut Value) -> Result<Self, CompactError> {
        let val = value.0.pop().unwrap_or_default().0;
        String::from_utf8(val)
            .map(Self)
            .map_err(|_| "expected String".to_string().into())
    }
}

impl CompactType for CompactTypeUnsignedInteger {
    fn from_value(value: &mut Value) -> Result<Self, CompactError> {
        let maybe_val = value.0.pop();
        let Some(val) = maybe_val else {
            Err(format!("expected UnsignedInteger[<=?]"))?
        };
        value_to_bigint(&val).map(Self)
    }
}

impl CompactType for CompactTypeField {
    fn from_value(value: &mut Value) -> Result<Self, CompactError> {
        let maybe_val = value.0.pop();
        let Some(val) = maybe_val else {
            Err(format!("expected Field"))?
        };
        value_to_bigint(&val).map(Self)
    }
}

impl CompactType for CompactTypeEnum {
    fn from_value(value: &mut Value) -> Result<Self, CompactError> {
        let maybe_val = value.0.pop().map(|i| i.0);
        let Some(mut val) = maybe_val else {
            Err(format!("exptected Enum[<=?]"))?
        };
        let byte = val.pop().unwrap_or_default();
        Ok(Self(byte))
    }
}

impl<const N: usize, T> CompactType for CompactTypeVector<N, T>
where
    T: CompactType,
{
    fn from_value(value: &mut Value) -> Result<Self, CompactError> {
        let mut res = Vec::with_capacity(N);
        for _ in 0..N {
            let val = T::from_value(value)?;
            res.push(val);
        }
        res.try_into()
            .map(Self)
            .map_err(|_| format!("expected {N}-element-array").into())
    }
}

fn value_to_bigint(x: &ValueAtom) -> Result<BigInt, CompactError> {
    let mut bytes = curve::Fr::try_from(&*x).map_err(|e| e.to_string())?.as_le_bytes();
    bytes.reverse();
    Ok(BigInt(bytes))
}

macro_rules! compact_enum {
    ($name:ident { $($field:ident),+ }) => {
        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone, derive_more::Display)]
        pub enum $name {
            $(
                #[display("{}", stringify!($field))]
                $field
            ),+
        }

        impl CompactType for $name {
            fn from_value(value: &mut Value) -> Result<Self, CompactError> {
                let variants = [
                    $(Self::$field),+
                ];
                let idx = CompactTypeEnum::from_value(value)?.0;
                variants.get(idx as usize).cloned().ok_or(
                    format!("exptected Enum[<=?] for type {}", stringify!($name)).into(),
                )
            }
        }
    };
}

macro_rules! compact_struct {
    ($name:ident {
        $($fields:ident: $tys:ty),+
    }) => {
        pub struct $name {
            $($fields: $tys),+
        }

        impl CompactType for $name {
            fn from_value(value: &mut Value) -> Result<Self, CompactError> {
                $(let $fields = <$tys as CompactType>::from_value(value)?;)+
                Ok(Self {
                    $($fields),*
                })
            }
        }
    };
}

macro_rules! compact_ledger {
    ($name:ident {
        $($field:ident: $adt:tt <$ty:ty $(, $ty2:ty)?> [$($path:literal),*]),+
    }) => {
        paste::paste! {
            $(
                #[allow(non_camel_case_types)]
                struct [<$name _ $field:camel>];

                impl StateValuePath for [<$name _ $field:camel>]{
                    fn state_value_path() -> &'static [u8] {
                        &[$($path),*]
                    }
                }
            )+

            pub struct $name {
                $($field: compact_ledger!(@internal $adt <$ty $(, $ty2)?>, [<$name _ $field:camel>])),+
            }

            impl $name {
                pub fn from_state_value<D: midnight_ledger_v4::storage::db::DB>(value: &midnight_ledger_v4::onchain_runtime::state::StateValue<D>) -> Result<$name, CompactError> {
                    Ok(Self {
                        $($field: <compact_ledger!(@internal $adt <$ty $(, $ty2)?>, [<$name _ $field:camel>]) as AdtType>::from_state(value)?),+
                    })
                }
            }
        }
    };
    (@internal cell <$ty:ty>, $pty:ident) => {
        AdtTypeCell<$ty, $pty>
    };
    (@internal set <$ty:ty>, $pty:ident) => {
        AdtTypeSet<$ty, $pty>
    };
    (@internal map <$ty:ty, $ty2:ty>, $pty:ident) => {
        AdtTypeMap<$ty, $ty2, $pty>
    }
}

pub(crate) use {compact_enum, compact_ledger, compact_struct};
