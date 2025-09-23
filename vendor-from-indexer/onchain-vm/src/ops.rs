use crate::base_crypto::fab::AlignedValue;
use crate::base_crypto::repr::MemWrite;
use crate::result_mode::{ResultMode, ResultModeVerify};
use crate::runtime_state::state::{StateValue, int_size, read_int, write_int};
use crate::serialize::{
    self, Deserializable, Serializable, Version, Versioned, check_injected_version,
};
#[cfg(all(feature = "proptest", test))]
use crate::serialize::{NetworkId, deserialize, serialize, serialized_size};
#[cfg(feature = "proptest")]
use crate::storage::serialize::randomised_serialization_test;
use crate::transient_crypto::curve::Fr;
use crate::transient_crypto::repr::FieldRepr;
use derive_where::derive_where;
#[cfg(all(test, feature = "proptest"))]
use proptest::prelude::*;
#[cfg(feature = "proptest")]
use proptest_derive::Arbitrary;
use runtime_state::storage::DefaultDB;
use runtime_state::storage::db::DB;
#[cfg(all(test, feature = "proptest"))]
use runtime_state::storage::db::InMemoryDB;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::io::{self, Read, Write};

#[derive(Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "proptest", derive(Arbitrary))]
#[cfg_attr(
    feature = "serde",
    serde(tag = "tag", content = "value", rename_all = "camelCase")
)]
pub enum Key {
    Value(AlignedValue),
    Stack,
}

impl Debug for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Key::Value(v) => v.fmt(f),
            Key::Stack => write!(f, "STK"),
        }
    }
}

impl TryFrom<Key> for AlignedValue {
    type Error = ();
    fn try_from(value: Key) -> Result<Self, Self::Error> {
        match value {
            Key::Value(v) => Ok(v),
            Key::Stack => Err(()),
        }
    }
}

impl FieldRepr for Key {
    fn field_repr<W: MemWrite<Fr>>(&self, writer: &mut W) {
        match self {
            Key::Stack => writer.write(&[(-1).into()]),
            Key::Value(v) => v.field_repr(writer),
        }
    }

    fn field_size(&self) -> usize {
        match self {
            Key::Stack => 1,
            Key::Value(v) => v.field_size(),
        }
    }
}

#[non_exhaustive]
#[derive_where(Clone, Eq, PartialEq; M)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "proptest", derive(Arbitrary))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "M::ReadResult : Serialize",
        deserialize = "M::ReadResult : Deserialize<'de>"
    ))
)]
#[cfg_attr(
    feature = "serde",
    serde(rename_all = "lowercase", expecting = "operation")
)]
pub enum Op<M: ResultMode<D>, D: DB> {
    Noop {
        #[cfg_attr(
            all(test, feature = "proptest"),
            proptest(strategy = "any::<u32>().prop_map(|x| x % 0x1FFFFF)")
        )]
        n: u32,
    },
    Lt,
    Eq,
    Type,
    Size,
    New,
    And,
    Or,
    Neg,
    Log,
    Root,
    Pop,
    Popeq {
        cached: bool,
        result: M::ReadResult,
    },
    Addi {
        #[cfg_attr(
            all(test, feature = "proptest"),
            proptest(strategy = "any::<u32>().prop_map(|x| x % 0x1FFFFF)")
        )]
        immediate: u32,
    },
    Subi {
        #[cfg_attr(
            all(test, feature = "proptest"),
            proptest(strategy = "any::<u32>().prop_map(|x| x % 0x1FFFFF)")
        )]
        immediate: u32,
    },
    Push {
        storage: bool,
        value: StateValue<D>,
    },
    Branch {
        #[cfg_attr(
            all(test, feature = "proptest"),
            proptest(strategy = "any::<u32>().prop_map(|x| x % 0x1FFFFF)")
        )]
        skip: u32,
    },
    Jmp {
        #[cfg_attr(
            all(test, feature = "proptest"),
            proptest(strategy = "any::<u32>().prop_map(|x| x % 0x1FFFFF)")
        )]
        skip: u32,
    },
    Add,
    Sub,
    Concat {
        cached: bool,
        #[cfg_attr(
            all(test, feature = "proptest"),
            proptest(strategy = "any::<u32>().prop_map(|x| x % 0x1FFFFF)")
        )]
        n: u32,
    },
    Member,
    Rem {
        cached: bool,
    },
    Dup {
        #[cfg_attr(
            all(test, feature = "proptest"),
            proptest(strategy = "any::<u8>().prop_map(|x| x % 16)")
        )]
        n: u8,
    },
    Swap {
        #[cfg_attr(
            all(test, feature = "proptest"),
            proptest(strategy = "any::<u8>().prop_map(|x| x % 16)")
        )]
        n: u8,
    },
    Idx {
        cached: bool,
        #[cfg_attr(feature = "serde", serde(rename = "pushPath"))]
        push_path: bool,
        #[cfg_attr(
            all(test, feature = "proptest"),
            proptest(strategy = "proptest::collection::vec(Key::arbitrary(), 1..2)")
        )]
        path: Vec<Key>,
    },
    Ins {
        cached: bool,
        #[cfg_attr(
            all(test, feature = "proptest"),
            proptest(strategy = "any::<u8>().prop_map(|x| x % 16)", filter = "|v| *v != 0")
        )]
        n: u8,
    },
    Ckpt,
}

#[macro_export]
macro_rules! key {
    (stack) => {
        Key::Stack
    };
    ($val:expr) => {
        Key::Value($val.into())
    };
}

#[macro_export]
macro_rules! op {
    (noop $val:expr) => { Op::Noop { n: $val } };
    (lt) => { Op::Lt };
    (eq) => { Op::Eq };
    (type) => { Op::Type };
    (size) => { Op::Size };
    (new) => { Op::New };
    (and) => { Op::And };
    (or) => { Op::Or };
    (neg) => { Op::Neg };
    (log) => { Op::Log };
    (root) => { Op::Root };
    (pop) => { Op::Pop };
    (popeq $res:expr) => { Op::Popeq { cached: false, result: $res } };
    (popeqc $res:expr) => { Op::Popeq { cached: true, result: $res } };
    (addi $imm:expr) => { Op::Addi { immediate: $imm } };
    (subi $imm:expr) => { Op::Subi { immediate: $imm } };
    (push $val:tt) => { Op::Push { storage: false, value: stval!($val) } };
    (pushs $val:tt) => { Op::Push { storage: true, value: stval!($val) } };
    (branch $skip:expr) => { Op::Branch { skip: $skip } };
    (jmp $skip:expr) => { Op::Jmp { skip: $skip } };
    (add) => { Op::Add };
    (sub) => { Op::Sub };
    (concat $n:expr) => { Op::Concat { cached: false, n: $n } };
    (concatc $n:expr) => { Op::Concat { cached: true, n: $n } };
    (member) => { Op::Member };
    (rem) => { Op::Rem { cached: false } };
    (remc) => { Op::Rem { cached: true } };
    (dup $n:expr) => { Op::Dup { n: $n } };
    (swap $n:expr) => { Op::Swap { n: $n } };
    (idx [$($key:tt),*]) => { Op::Idx { cached: false, push_path: false, path: vec![$(key!($key)),*] }};
    (idxc [$($key:tt),*]) => { Op::Idx { cached: true, push_path: false, path: vec![$(key!($key)),*] }};
    (idxp [$($key:tt),*]) => { Op::Idx { cached: false, push_path: true, path: vec![$(key!($key)),*] }};
    (idxpc [$($key:tt),*]) => { Op::Idx { cached: true, push_path: true, path: vec![$(key!($key)),*] }};
    (ins $n:expr) => { Op::Ins { cached: false, n: $n } };
    (insc $n:expr) => { Op::Ins { cached: true, n: $n } };
    (ckpt) => { Op::Ckpt };
}

#[macro_export]
macro_rules! ops_int {
    [] => { std::iter::empty() };
    [;] => { std::iter::empty() };
    [$op0:tt ; $($ops:tt)*] => { std::iter::once(op!($op0)).chain(ops_int!($($ops)*)) };
    [$op0:tt $op1:tt ; $($ops:tt)*] => { std::iter::once(op!($op0 $op1)).chain(ops_int!($($ops)*)) };
    [$op0:tt $op1:tt $op2:tt ; $($ops:tt)*] => { std::iter::once(op!($op0 $op1 $op2)).chain(ops_int!($($ops)*)) };
    [$op0:tt $op1:tt $op2:tt $op3:tt ; $($ops:tt)*] => { std::iter::once(op!($op0 $op1 $op2 $op3)).chain(ops_int!($($ops)*)) };
    [$($ops:tt)*] => { std::iter::once(op!($($ops)*)) };
}

#[macro_export]
macro_rules! ops {
    [$($tts:tt)*] => { ops_int!($($tts)*).collect::<Vec<_>>() };
}

impl<M: ResultMode<D>, D: DB> Debug for Op<M, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Op::*;
        match self {
            Noop { n } => write!(f, "noop {n}"),
            Lt => write!(f, "lt"),
            Eq => write!(f, "eq"),
            Type => write!(f, "type"),
            Size => write!(f, "size"),
            New => write!(f, "new"),
            And => write!(f, "and"),
            Or => write!(f, "or"),
            Neg => write!(f, "neg"),
            Log => write!(f, "log"),
            Root => write!(f, "root"),
            Pop => write!(f, "pop"),
            Popeq {
                cached: false,
                result,
            } => write!(f, "popeq {result:?}"),
            Popeq {
                cached: true,
                result,
            } => write!(f, "popeqc {result:?}"),
            Addi { immediate } => write!(f, "addi {immediate:?}"),
            Subi { immediate } => write!(f, "subi {immediate:?}"),
            Push {
                storage: false,
                value,
            } => write!(f, "push {value:?}"),
            Push {
                storage: true,
                value,
            } => write!(f, "pushs {value:?}"),
            Branch { skip } => write!(f, "branch {skip}"),
            Jmp { skip } => write!(f, "jmp {skip}"),
            Add => write!(f, "add"),
            Sub => write!(f, "sub"),
            Concat { cached: false, n } => write!(f, "concat {n}"),
            Concat { cached: true, n } => write!(f, "concatc {n}"),
            Member => write!(f, "member"),
            Rem { cached: false } => write!(f, "rem"),
            Rem { cached: true } => write!(f, "remc"),
            Dup { n } => write!(f, "dup {n}"),
            Swap { n } => write!(f, "swap {n}"),
            Idx {
                cached,
                push_path,
                path,
            } => {
                write!(f, "idx")?;
                if *push_path {
                    write!(f, "p")?;
                }
                if *cached {
                    write!(f, "c")?;
                }
                write!(f, " [")?;
                let mut is_first = true;
                for key in path.iter() {
                    if is_first {
                        is_first = false;
                    } else {
                        write!(f, ", ")?;
                    }
                    write!(f, "{key:?}")?;
                }
                write!(f, "]")
            }
            Ins { cached: false, n } => write!(f, "ins {n}"),
            Ins { cached: true, n } => write!(f, "insc {n}"),
            Ckpt => write!(f, "ckpt"),
        }
    }
}

fn limit_val(val: u8, limit: u8) -> io::Result<u8> {
    if val < limit {
        Ok(val)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{val} exceeds limit of {limit}"),
        ))
    }
}

fn write_path<W: Write>(writer: &mut W, path: &[Key]) -> io::Result<()> {
    for key in path {
        match key {
            Key::Value(val) => Serializable::serialize(val, writer)?,
            Key::Stack => writer.write_all(&[0xff][..])?,
        }
    }
    Ok(())
}

fn read_path<R: Read>(mut reader: &mut R, n: usize) -> io::Result<Vec<Key>> {
    (0..n)
        .map(|_| {
            let first = <u8 as Deserializable>::deserialize(reader, 0)?;
            match first {
                0x00..=0xbf => Ok(Key::Value(<AlignedValue as Deserializable>::deserialize(
                    &mut [first].chain(&mut reader),
                    0,
                )?)),
                0xff => Ok(Key::Stack),
                _ => Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid path key start byte: {first:02x}"),
                )),
            }
        })
        .collect()
}

fn path_size(path: &[Key]) -> usize {
    path.iter()
        .map(|key| match key {
            Key::Value(val) => Serializable::serialized_size(&val),
            Key::Stack => 1,
        })
        .sum()
}

impl<M: ResultMode<D>, D: DB> Op<M, D> {
    pub fn translate<M2: ResultMode<D>, F: FnOnce(M::ReadResult) -> M2::ReadResult>(
        self,
        f: F,
    ) -> Op<M2, D> {
        match self {
            Op::Noop { n } => Op::Noop { n },
            Op::Lt => Op::Lt,
            Op::Eq => Op::Eq,
            Op::Type => Op::Type,
            Op::Size => Op::Size,
            Op::New => Op::New,
            Op::And => Op::And,
            Op::Or => Op::Or,
            Op::Neg => Op::Neg,
            Op::Log => Op::Log,
            Op::Root => Op::Root,
            Op::Pop => Op::Pop,
            Op::Popeq { cached, result } => Op::Popeq {
                cached,
                result: f(result),
            },
            Op::Addi { immediate } => Op::Addi { immediate },
            Op::Subi { immediate } => Op::Subi { immediate },
            Op::Push { storage, value } => Op::Push { storage, value },
            Op::Branch { skip } => Op::Branch { skip },
            Op::Jmp { skip } => Op::Jmp { skip },
            Op::Add => Op::Add,
            Op::Sub => Op::Sub,
            Op::Concat { cached, n } => Op::Concat { cached, n },
            Op::Member => Op::Member,
            Op::Rem { cached } => Op::Rem { cached },
            Op::Dup { n } => Op::Dup { n },
            Op::Swap { n } => Op::Swap { n },
            Op::Idx {
                cached,
                push_path,
                path,
            } => Op::Idx {
                cached,
                push_path,
                path,
            },
            Op::Ins { cached, n } => Op::Ins { cached, n },
            Op::Ckpt => Op::Ckpt,
        }
    }
}

impl<M: ResultMode<D>, D: DB> Versioned for Op<M, D> {
    const VERSION: Option<Version> = Some(Version { major: 2, minor: 2 });
    const NETWORK_SPECIFIC: bool = true;
}

impl<M: ResultMode<D>, D: DB> Serializable for Op<M, D> {
    fn unversioned_serialize<W: Write>(value: &Self, writer: &mut W) -> Result<(), std::io::Error> {
        use Op::*;
        let limit_val_nonzero = |val, limit| {
            if val == 0 {
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "expected non-zero size",
                ))
            } else {
                limit_val(val, limit)
            }
        };
        match value {
            Noop { n } => {
                writer.write_all(&[0x00][..])?;
                write_int(writer, *n as u64)
            }
            Lt => writer.write_all(&[0x01][..]),
            Eq => writer.write_all(&[0x02][..]),
            Type => writer.write_all(&[0x03][..]),
            Size => writer.write_all(&[0x04][..]),
            New => writer.write_all(&[0x05][..]),
            And => writer.write_all(&[0x06][..]),
            Or => writer.write_all(&[0x07][..]),
            Neg => writer.write_all(&[0x08][..]),
            Log => writer.write_all(&[0x09][..]),
            Root => writer.write_all(&[0x0a][..]),
            Pop => writer.write_all(&[0x0b][..]),
            Popeq { cached, result } => {
                writer.write_all(&[if *cached { 0x0d } else { 0x0c }][..])?;
                Serializable::serialize(result, writer)
            }
            Addi { immediate } => {
                writer.write_all(&[0x0e][..])?;
                write_int(writer, *immediate as u64)
            }
            Subi { immediate } => {
                writer.write_all(&[0x0f][..])?;
                write_int(writer, *immediate as u64)
            }
            Push { storage, value } => {
                writer.write_all(&[if *storage { 0x11 } else { 0x10 }][..])?;
                Serializable::unversioned_serialize(value, writer)
            }
            Branch { skip } => {
                writer.write_all(&[0x12][..])?;
                write_int(writer, *skip as u64)
            }
            Jmp { skip } => {
                writer.write_all(&[0x13][..])?;
                write_int(writer, *skip as u64)
            }
            Add => writer.write_all(&[0x14][..]),
            Sub => writer.write_all(&[0x15][..]),
            Concat { cached, n } => {
                writer.write_all(&[if *cached { 0x17 } else { 0x16 }][..])?;
                write_int(writer, *n as u64)
            }
            Member => writer.write_all(&[0x18][..]),
            Rem { cached: false } => writer.write_all(&[0x19][..]),
            Rem { cached: true } => writer.write_all(&[0x1a][..]),
            Dup { n } => writer.write_all(&[0x30 | limit_val(*n, 16)?][..]),
            Swap { n } => writer.write_all(&[0x40 | limit_val(*n, 16)?][..]),
            Idx {
                cached,
                push_path,
                path,
            } => {
                let msb = match (push_path, cached) {
                    (false, false) => 0x50,
                    (false, true) => 0x60,
                    (true, false) => 0x70,
                    (true, true) => 0x80,
                };
                writer.write_all(&[msb | (limit_val_nonzero(path.len() as u8, 17)? - 1)][..])?;
                write_path(writer, path)
            }
            Ins { cached, n } => writer
                .write_all(&[if *cached { 0xa0 } else { 0x90 } | (limit_val_nonzero(*n, 16)?)][..]),
            Ckpt => writer.write_all(&[0xff][..]),
        }
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        use Op::*;
        match value {
            Lt
            | Eq
            | Type
            | Size
            | New
            | And
            | Or
            | Neg
            | Log
            | Root
            | Pop
            | Dup { .. }
            | Add
            | Sub
            | Swap { .. }
            | Member
            | Rem { .. }
            | Ins { .. }
            | Ckpt => 1,
            Noop { n } | Addi { immediate: n } | Subi { immediate: n } => 1 + int_size(*n as u64),
            Popeq { result, .. } => 1 + Serializable::serialized_size(result),
            Push { value, .. } => 1 + Serializable::unversioned_serialized_size(value),
            Branch { skip } | Jmp { skip } => 1 + int_size(*skip as u64),
            Concat { n, .. } => 1 + int_size(*n as u64),
            Idx { path, .. } => 1 + path_size(path),
        }
    }
}

#[cfg(all(test, feature = "proptest"))]
type SimpleOp = Op<ResultModeVerify, InMemoryDB>;
#[cfg(feature = "proptest")]
randomised_serialization_test!(SimpleOp);

impl<M: ResultMode<D>, D: DB> Deserializable for Op<M, D> {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&serialize::Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        use Op::*;
        let opcode = <u8 as Deserializable>::deserialize(reader, recursion_depth)?;
        match version {
            Some(Version { major: 2, minor: 0 })
            | Some(Version { major: 2, minor: 1 })
            | Some(Version { major: 2, minor: 2 }) => {
                Ok(match opcode {
                    0x00 => Noop {
                        n: read_int(reader)? as u32,
                    },
                    0x01 => Lt,
                    0x02 => Eq,
                    0x03 => Type,
                    0x04 => Size,
                    0x05 => New,
                    0x06 => And,
                    0x07 => Or,
                    0x08 => Neg,
                    0x09 => Log,
                    0x0a => Root,
                    0x0b => Pop,
                    0x0c => Popeq {
                        cached: false,
                        result: Deserializable::deserialize(reader, recursion_depth)?,
                    },
                    0x0d => Popeq {
                        cached: true,
                        result: Deserializable::deserialize(reader, recursion_depth)?,
                    },
                    0x0e => Addi {
                        immediate: read_int(reader)? as u32,
                    },
                    0x0f => Subi {
                        immediate: read_int(reader)? as u32,
                    },
                    0x10 | 0x11 => Push {
                        storage: opcode == 0x11,
                        value: {
                            /// Compile-time check that we're remembering to inject version information into
                            /// `StateValue` correctly
                            const _: () = check_injected_version(
                                Some(Version { major: 2, minor: 2 }),
                                StateValue::<DefaultDB>::VERSION,
                            );

                            match version {
                                Some(Version { major: 2, minor: 0 }) => {
                                    Deserializable::versioned_deserialize(
                                        reader,
                                        Some(&Version { major: 2, minor: 0 }),
                                        recursion_depth,
                                    )?
                                }
                                Some(Version { major: 2, minor: 1 }) => {
                                    Deserializable::versioned_deserialize(
                                        reader,
                                        Some(&Version { major: 2, minor: 1 }),
                                        recursion_depth,
                                    )?
                                }
                                Some(Version { major: 2, minor: 2 }) => {
                                    Deserializable::versioned_deserialize(
                                        reader,
                                        Some(&Version { major: 2, minor: 2 }),
                                        recursion_depth,
                                    )?
                                }
                                _ => unreachable!(),
                            }
                        },
                    },
                    0x12 => Branch {
                        skip: read_int(reader)? as u32,
                    },
                    0x13 => Jmp {
                        skip: read_int(reader)? as u32,
                    },
                    0x14 => Add,
                    0x15 => Sub,
                    0x16 => Concat {
                        cached: false,
                        n: read_int(reader)? as u32,
                    },
                    0x17 => Concat {
                        cached: true,
                        n: read_int(reader)? as u32,
                    },
                    0x18 => Member,
                    0x19 => Rem { cached: false },
                    0x1a => Rem { cached: true },
                    0x30..=0x3f => Dup { n: opcode & 0x0f },
                    0x40..=0x4f => Swap { n: opcode & 0x0f },
                    0x50..=0x8f => {
                        let msb = opcode & 0xf0;
                        let cached = msb == 0x60 || msb == 0x80;
                        let push_path = msb == 0x70 || msb == 0x80;
                        Idx {
                            cached,
                            push_path,
                            path: read_path(reader, (opcode as usize & 0x0f) + 1)?,
                        }
                    }
                    0x91..=0x9f => Ins {
                        cached: false,
                        n: opcode & 0x0f,
                    },
                    0xa1..=0xaf => Ins {
                        cached: true,
                        n: opcode & 0x0f,
                    },
                    0xff => Ckpt,
                    _ => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("invalid opcode: {opcode:02x}"),
                        ));
                    }
                })
            }
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version.".to_string(),
            )),
        }
    }
}

impl<D: DB> FieldRepr for Op<ResultModeVerify, D> {
    fn field_repr<W: MemWrite<Fr>>(&self, writer: &mut W) {
        use Op::*;
        match self {
            Noop { n } => writer.write(&vec![0x00.into(); *n as usize]),
            Lt => writer.write(&[0x01.into()]),
            Eq => writer.write(&[0x02.into()]),
            Type => writer.write(&[0x03.into()]),
            Size => writer.write(&[0x04.into()]),
            New => writer.write(&[0x05.into()]),
            And => writer.write(&[0x06.into()]),
            Or => writer.write(&[0x07.into()]),
            Neg => writer.write(&[0x08.into()]),
            Log => writer.write(&[0x09.into()]),
            Root => writer.write(&[0x0a.into()]),
            Pop => writer.write(&[0x0b.into()]),
            Popeq { cached, result } => {
                writer.write(&[(0x0c + *cached as u8).into()]);
                result.field_repr(writer);
            }
            Addi { immediate } => {
                writer.write(&[0x0e.into()]);
                immediate.field_repr(writer);
            }
            Subi { immediate } => {
                writer.write(&[0x0f.into()]);
                immediate.field_repr(writer);
            }
            Push { storage, value } => {
                writer.write(&[(0x10 + *storage as u8).into()]);
                value.field_repr(writer);
            }
            Branch { skip } => writer.write(&[0x12.into(), (*skip).into()]),
            Jmp { skip } => writer.write(&[0x13.into(), (*skip).into()]),
            Add => writer.write(&[0x14.into()]),
            Sub => writer.write(&[0x15.into()]),
            Concat { cached: false, n } => writer.write(&[0x16.into(), (*n).into()]),
            Concat { cached: true, n } => writer.write(&[0x17.into(), (*n).into()]),
            Member => writer.write(&[0x18.into()]),
            Rem { cached: false } => writer.write(&[0x19.into()]),
            Rem { cached: true } => writer.write(&[0x1a.into()]),
            Dup { n } => writer.write(&[(0x30 | *n).into()]),
            Swap { n } => writer.write(&[(0x40 | *n).into()]),
            Idx {
                cached,
                push_path,
                path,
            } => {
                if !path.is_empty() {
                    let opcode = match (*cached, *push_path) {
                        (false, false) => 0x50,
                        (true, false) => 0x60,
                        (false, true) => 0x70,
                        (true, true) => 0x80,
                    } | (path.len() as u8 - 1);
                    writer.write(&[opcode.into()]);
                    for entry in path.iter() {
                        entry.field_repr(writer);
                    }
                }
            }
            Ins { cached: false, n } => writer.write(&[(0x90 | *n).into()]),
            Ins { cached: true, n } => writer.write(&[(0xa0 | *n).into()]),
            Ckpt => writer.write(&[0xff.into()]),
        }
    }

    fn field_size(&self) -> usize {
        use Op::*;
        match self {
            Lt
            | Eq
            | Type
            | Size
            | New
            | And
            | Or
            | Neg
            | Log
            | Root
            | Pop
            | Add
            | Sub
            | Member
            | Rem { .. }
            | Dup { .. }
            | Swap { .. }
            | Ins { .. }
            | Ckpt => 1,
            Noop { n } => *n as usize,
            Branch { .. } | Jmp { .. } | Concat { .. } => 2,
            Addi { immediate } | Subi { immediate } => 1 + immediate.field_size(),
            Popeq { result, .. } => 1 + result.field_size(),
            Push { value, .. } => 1 + value.field_size(),
            Idx { path, .. } => 1 + path.iter().map(FieldRepr::field_size).sum::<usize>(),
        }
    }
}

pub use {key, op, ops, ops_int};

#[cfg(test)]
mod tests {
    use coin_structure::serialize::NetworkId;
    use runtime_state::storage::DefaultDB;

    use super::Op;
    use crate::result_mode::ResultModeGather;
    use crate::runtime_state::state::StateValue;
    use crate::storage::storage::HashMap;

    #[test]
    fn diagnostic_test_map_serialization_stability() {
        let op: Op<ResultModeGather, DefaultDB> = Op::Push {
            storage: false,
            value: StateValue::Map(HashMap::new()),
        };
        let mut ser = Vec::new();
        crate::serialize::serialize(&op, &mut ser, NetworkId::Undeployed).unwrap();
        let op2: Op<ResultModeGather, DefaultDB> =
            crate::serialize::deserialize(&ser[..], NetworkId::Undeployed).unwrap();
        assert_eq!(op, op2);
    }
}
