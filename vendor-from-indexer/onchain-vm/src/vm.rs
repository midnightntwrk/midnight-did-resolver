use crate::base_crypto::fab::{AlignedValue, Alignment, InvalidBuiltinDecode, Value};
use crate::cost_model::CostModel;
use crate::error::OnchainProgramError;
use crate::ops::*;
use crate::result_mode::ResultMode;
use crate::runtime_state::state::*;
use crate::serialize::Serializable;
use crate::state_value_ext::*;
use crate::storage::storage::HashMap;
use crate::vm_value::*;
use rpds::{HashTrieMap, Vector};
use runtime_state::storage::db::DB;
use runtime_state::transient_crypto::merkle_tree::MerkleTree;
use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use ValueStrength::*;

// Maps initial stack positions to which parts of those stack objects are currently cached.
struct Cache(Vec<InnerCache>);

impl Cache {
    fn visit(&mut self, key: &CacheKey) -> bool {
        match &key.0 {
            None => true,
            Some((n, path)) => {
                if *n > self.0.len() {
                    false
                } else {
                    path.iter()
                        .fold((&mut self.0[*n], true), |(cache, found), key| match cache {
                            InnerCache(None) => (cache, found),
                            InnerCache(Some(map)) => {
                                if map.contains_key(key) {
                                    (map.get_mut(key).unwrap(), found)
                                } else {
                                    map.insert_mut(
                                        key.clone(),
                                        InnerCache(Some(HashTrieMap::new())),
                                    );
                                    (map.get_mut(key).unwrap(), false)
                                }
                            }
                        })
                        .1
                }
            }
        }
    }
}

// For a given data structure, either:
//  - If None, indicates the entire data structure is cached.
//  - If Some, maps keys in the data structure to the parts of those objects
//    that are currently cached.
// Note: If a value is copied *into another data structure*, it is no longer
// linked to its source in the cache structure. This is _not_ the case if it is
// copied through the stack.
#[derive(Clone)]
struct InnerCache(Option<HashTrieMap<AlignedValue, InnerCache>>);

#[derive(Clone)]
struct CacheKey(Option<(usize, Vector<AlignedValue>)>);

impl CacheKey {
    fn push_key(&self, key: &AlignedValue) -> Self {
        CacheKey(
            self.0
                .as_ref()
                .map(|(idx, path)| (*idx, path.push_back(key.clone()))),
        )
    }
}

impl Debug for CacheKey {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        match &self.0 {
            None => write!(fmt, "-"),
            Some((idx, path)) => {
                write!(fmt, "{}", idx)?;
                for key in path.iter() {
                    write!(fmt, "/{:?}", key)?;
                }
                Ok(())
            }
        }
    }
}

pub const MAX_STACK_HEIGHT: u32 = 1 << 16;

fn add<A: AsRef<Value>, E, B: TryInto<u64, Error = E>, D: DB>(
    a: A,
    b: B,
) -> Result<AlignedValue, OnchainProgramError<D>>
where
    OnchainProgramError<D>: From<E>,
{
    let a_tmp: Result<u64, InvalidBuiltinDecode> = (&**a.as_ref()).try_into();
    let a = a_tmp?;
    let b: u64 = b.try_into()?;
    a.checked_add(b)
        .ok_or(OnchainProgramError::ArithmeticOverflow)
        .map(Into::into)
}

fn sub<A: AsRef<Value>, E, B: TryInto<u64, Error = E>, D: DB>(
    a: A,
    b: B,
) -> Result<AlignedValue, OnchainProgramError<D>>
where
    OnchainProgramError<D>: From<E>,
{
    let a: Result<u64, InvalidBuiltinDecode> = (&**a.as_ref()).try_into();
    let b: u64 = b.try_into()?;
    a?.checked_sub(b)
        .ok_or(OnchainProgramError::ArithmeticOverflow)
        .map(Into::into)
}

fn lt<A: AsRef<Value>, B: AsRef<Value>, D: DB>(
    a: A,
    b: B,
) -> Result<AlignedValue, OnchainProgramError<D>> {
    let a: u64 = (&**a.as_ref()).try_into()?;
    let b: u64 = (&**b.as_ref()).try_into()?;
    Ok((a < b).into())
}

fn concat<A: AsRef<AlignedValue>, B: AsRef<AlignedValue>, D: DB>(
    a: A,
    b: B,
    bound: u32,
) -> Result<AlignedValue, OnchainProgramError<D>> {
    if Serializable::serialized_size(a.as_ref()) + Serializable::serialized_size(b.as_ref())
        <= bound as usize
    {
        AlignedValue::new(
            Value::concat([
                AsRef::<Value>::as_ref(a.as_ref()),
                AsRef::<Value>::as_ref(b.as_ref()),
            ]),
            Alignment::concat([
                AsRef::<Alignment>::as_ref(a.as_ref()),
                AsRef::<Alignment>::as_ref(b.as_ref()),
            ]),
        )
        .ok_or(OnchainProgramError::TypeError)
    } else {
        Err(OnchainProgramError::BoundsExceeded)
    }
}

fn eq_valid_input(x: &AlignedValue) -> bool {
    Serializable::serialized_size(x) <= 64
}

fn eq<A: AsRef<AlignedValue>, B: AsRef<AlignedValue>, D: DB>(
    a: A,
    b: B,
) -> Result<AlignedValue, OnchainProgramError<D>> {
    if !eq_valid_input(a.as_ref()) || !eq_valid_input(b.as_ref()) {
        Err(OnchainProgramError::TooLongForEqual)
    } else {
        Ok((a.as_ref() == b.as_ref()).into())
    }
}

fn idx<D: DB>(
    value: &(VmValue<D>, CacheKey),
    key: &AlignedValue,
    cache: &mut Cache,
    cached: bool,
) -> Result<(VmValue<D>, CacheKey), OnchainProgramError<D>> {
    let cache_key = value.1.push_key(key);
    let cache_miss = !cache.visit(&cache_key);
    match &value.0.value {
        StateValue::Array(arr) => {
            let idx: u8 = (&**AsRef::<Value>::as_ref(&key)).try_into()?;
            if idx as usize >= arr.len() {
                Err(OnchainProgramError::TypeError)
            } else {
                let res = &arr[idx as usize];
                if cached && cache_miss {
                    Err(OnchainProgramError::CacheMiss)
                } else {
                    Ok((VmValue::new(value.0.strength, res.clone()), cache_key))
                }
            }
        }
        StateValue::Map(map) => {
            let res = map
                .get(key)
                .map(|sp| (*sp).clone())
                .unwrap_or(StateValue::Null);
            if cached && cache_miss {
                Err(OnchainProgramError::CacheMiss)
            } else {
                Ok((VmValue::new(value.0.strength, res), cache_key))
            }
        }
        StateValue::BoundedMerkleTree(tree) => {
            let key = (&**AsRef::<Value>::as_ref(&key)).try_into()?;
            if key >= (1u64 << tree.height() as u64) {
                Err(OnchainProgramError::MissingKey)
            } else {
                Ok((
                    match tree.index(key) {
                        Some((hash, ())) => {
                            VmValue::new(value.0.strength, StateValue::Cell(Arc::new(hash.into())))
                        }
                        None => VmValue::new(value.0.strength, StateValue::Null),
                    },
                    cache_key,
                ))
            }
        }
        _ => Err(OnchainProgramError::TypeError),
    }
}

#[derive(Debug)]
pub struct VmResults<M: ResultMode<D>, D: DB> {
    pub stack: Vec<VmValue<D>>,
    pub events: Vec<M::Event>,
    pub gas_cost: u64,
}

/// Run a VM program.
///
/// The starting stack `initial` is consumed from right to left, i.e. highest to
/// lowest index position; popping the stack returns the last element.
///
/// The op-code sequence `program` is evaluated from left to right, i.e. from
/// lowest to highest index position.
pub fn run_program<M: ResultMode<D>, D: DB>(
    initial: &[VmValue<D>],
    program: &[Op<M, D>],
    gas_limit: Option<u64>,
    cost_model: &CostModel,
) -> Result<VmResults<M, D>, OnchainProgramError<D>> {
    let initial_annot = initial
        .iter()
        .enumerate()
        .map(|(i, val)| (val.clone(), CacheKey(Some((i, Vector::new())))))
        .collect();
    let mut cache = Cache(
        initial
            .iter()
            .map(|val| {
                if val.strength == ValueStrength::Weak {
                    InnerCache(None)
                } else {
                    InnerCache(Some(HashTrieMap::new()))
                }
            })
            .collect(),
    );
    run_program_internal(
        initial_annot,
        Vec::new(),
        &mut cache,
        program,
        gas_limit,
        cost_model,
    )
}

fn run_program_internal<M: ResultMode<D>, D: DB>(
    mut stack: Vec<(VmValue<D>, CacheKey)>,
    mut events: Vec<M::Event>,
    cache: &mut Cache,
    mut program: &[Op<M, D>],
    gas_limit: Option<u64>,
    cost_model: &CostModel,
) -> Result<VmResults<M, D>, OnchainProgramError<D>> {
    use Op::*;
    let vnew = VmValue::new;
    let mut gas: u64 = 0;
    let incr_gas = |mut gas: u64, by: u64| {
        gas += by;
        match gas_limit {
            Some(limit) if gas > limit => Err(OnchainProgramError::OutOfGas),
            _ => Ok(gas),
        }
    };
    while !program.is_empty() {
        let op = &program[0];
        // dbg!(&op, &stack);
        let stack_req = match op {
            Noop { .. } | Push { .. } | Branch { .. } | Jmp { .. } | Ckpt => 0,
            Type
            | Size
            | New
            | Neg
            | Log
            | Root
            | Pop
            | Popeq { .. }
            | Addi { .. }
            | Subi { .. } => 1,
            Lt | Eq | And | Or | Add | Sub | Concat { .. } | Member | Rem { .. } => 2,
            Dup { n } => *n as usize + 1,
            Swap { n } => *n as usize + 2,
            Idx { path, .. } => path.iter().filter(|key| key == &&Key::Stack).count() + 1,
            Ins { n, .. } => *n as usize * 2 + 1,
        };
        let stack_len = stack.len();
        if stack_len < stack_req {
            return Err(OnchainProgramError::RanOffStack);
        }
        match op {
            // Branches get handled later
            Noop { n } => {
                gas = incr_gas(
                    gas,
                    cost_model.noop_constant + cost_model.noop_linear * (*n as u64),
                )?;
            }
            Branch { .. } => {
                gas = incr_gas(gas, cost_model.branch)?;
            }
            Jmp { .. } => {
                gas = incr_gas(gas, cost_model.jmp)?;
            }
            Ckpt => {
                gas = incr_gas(gas, cost_model.ckpt)?;
            }
            Lt => {
                gas = incr_gas(gas, cost_model.lt)?;
                let b = stack.pop().unwrap().0.as_cell()?;
                let a = stack.pop().unwrap().0.as_cell()?;
                stack.push((vnew(Strong, lt(a, b)?.into()), CacheKey(None)));
            }
            Eq => {
                gas = incr_gas(gas, cost_model.eq)?;
                let a = stack.pop().unwrap().0.as_cell()?;
                let b = stack.pop().unwrap().0.as_cell()?;
                stack.push((vnew(Strong, eq(a, b)?.into()), CacheKey(None)));
            }
            Type => {
                gas = incr_gas(gas, cost_model.type_)?;
                let val = stack.pop().unwrap().0.value;
                stack.push((
                    vnew(
                        Strong,
                        AlignedValue::from(match &val {
                            StateValue::Cell(_) => 0,
                            StateValue::Null => 1,
                            StateValue::Map(_) => 2,
                            StateValue::Array(a) => 3 + a.len() as u8 * 8,
                            StateValue::BoundedMerkleTree(t) => 4 + t.height() * 8,
                            _ => return Err(OnchainProgramError::TypeError),
                        })
                        .into(),
                    ),
                    CacheKey(None),
                ));
            }
            Size => {
                gas = incr_gas(gas, cost_model.size)?;
                let val = stack.pop().unwrap().0.value;
                stack.push((
                    vnew(
                        Strong,
                        AlignedValue::from(match &val {
                            StateValue::Map(m) => m.size() as u64,
                            StateValue::Array(a) => a.len() as u64,
                            StateValue::BoundedMerkleTree(t) => t.height() as u64,
                            _ => return Err(OnchainProgramError::TypeError),
                        })
                        .into(),
                    ),
                    CacheKey(None),
                ));
            }
            New => {
                gas = incr_gas(gas, cost_model.new)?;
                let a = stack.pop().unwrap();
                let val: u8 = (&**AsRef::<Value>::as_ref(&a.0.value.as_cell()?)).try_into()?;
                stack.push((
                    vnew(
                        Strong,
                        match val & 0b111 {
                            0u8 => StateValue::Cell(Arc::new(().into())),
                            1u8 => StateValue::Null,
                            2u8 => StateValue::Map(HashMap::new()),
                            3u8 => StateValue::Array(
                                vec![StateValue::Null; (val >> 3) as usize].into(),
                            ),
                            4u8 => StateValue::BoundedMerkleTree(MerkleTree::blank(val >> 3)),
                            _ => return Err(OnchainProgramError::InvalidArgs),
                        },
                    ),
                    CacheKey(None),
                ));
            }
            And => {
                gas = incr_gas(gas, cost_model.and)?;
                let a = bool::try_from(&**AsRef::<Value>::as_ref(
                    &stack.pop().unwrap().0.as_cell()?,
                ))?;
                let b = bool::try_from(&**AsRef::<Value>::as_ref(
                    &stack.pop().unwrap().0.as_cell()?,
                ))?;
                stack.push((
                    vnew(Strong, AlignedValue::from(a && b).into()),
                    CacheKey(None),
                ));
            }
            Or => {
                gas = incr_gas(gas, cost_model.or)?;
                let a = bool::try_from(&**AsRef::<Value>::as_ref(
                    &stack.pop().unwrap().0.as_cell()?,
                ))?;
                let b = bool::try_from(&**AsRef::<Value>::as_ref(
                    &stack.pop().unwrap().0.as_cell()?,
                ))?;
                stack.push((
                    vnew(Strong, AlignedValue::from(a || b).into()),
                    CacheKey(None),
                ));
            }
            Neg => {
                gas = incr_gas(gas, cost_model.neg)?;
                let a = bool::try_from(&**AsRef::<Value>::as_ref(
                    &stack.pop().unwrap().0.as_cell()?,
                ))?;
                stack.push((vnew(Strong, AlignedValue::from(!a).into()), CacheKey(None)));
            }
            Log => {
                gas = incr_gas(gas, cost_model.log)?;
                if let Some(event) = M::process_log(&stack.pop().unwrap().0.value) {
                    events.push(event);
                }
            }
            Root => {
                gas = incr_gas(gas, cost_model.root)?;
                let a = stack.pop().unwrap().0.value;
                stack.push((
                    vnew(
                        Strong,
                        AlignedValue::from(match &a {
                            StateValue::BoundedMerkleTree(tree) => tree.root(),
                            _ => return Err(OnchainProgramError::TypeError),
                        })
                        .into(),
                    ),
                    CacheKey(None),
                ));
            }
            Pop => {
                gas = incr_gas(gas, cost_model.pop)?;
                drop(stack.pop().unwrap())
            }
            Popeq { cached, result } => {
                gas = incr_gas(
                    gas,
                    if *cached {
                        cost_model.popeqc_constant
                    } else {
                        cost_model.popeq_constant
                    },
                )?;
                // TODO: use `cached`!
                let value = &stack.pop().unwrap().0.value.as_cell()?;
                gas = incr_gas(
                    gas,
                    if *cached {
                        cost_model.popeqc_linear
                    } else {
                        cost_model.popeq_linear
                    } * <AlignedValue as Serializable>::serialized_size(value.as_ref()) as u64,
                )?;
                if let Some(event) = M::process_read(result, value)? {
                    events.push(event);
                }
            }
            Addi { immediate } => {
                gas = incr_gas(gas, cost_model.addi)?;
                let a = stack.pop().unwrap().0.as_cell()?;
                stack.push((vnew(Strong, add(a, *immediate)?.into()), CacheKey(None)));
            }
            Subi { immediate } => {
                gas = incr_gas(gas, cost_model.subi)?;
                let a = stack.pop().unwrap().0.as_cell()?;
                stack.push((vnew(Strong, sub(a, *immediate)?.into()), CacheKey(None)));
            }
            Push { storage, value } => {
                gas = incr_gas(
                    gas,
                    if *storage {
                        cost_model.pushs_constant
                            + cost_model.pushs_linear * Serializable::serialized_size(value) as u64
                    } else {
                        cost_model.push_constant
                            + cost_model.push_linear * Serializable::serialized_size(value) as u64
                    },
                )?;
                stack.push((
                    vnew(if *storage { Strong } else { Weak }, value.clone()),
                    CacheKey(None),
                ))
            }
            Add => {
                gas = incr_gas(gas, cost_model.add)?;
                let a = stack.pop().unwrap().0.as_cell()?;
                let b = stack.pop().unwrap().0.as_cell()?;
                stack.push((vnew(Strong, add(a, &*b.as_slice())?.into()), CacheKey(None)));
            }
            Sub => {
                gas = incr_gas(gas, cost_model.sub)?;
                let a = stack.pop().unwrap().0.as_cell()?;
                let b = stack.pop().unwrap().0.as_cell()?;
                stack.push((vnew(Strong, sub(b, &*a.as_slice())?.into()), CacheKey(None)));
            }
            Concat { cached, n } => {
                // TODO: used cached
                let a = stack.pop().unwrap().0.as_cell()?;
                let b = stack.pop().unwrap().0.as_cell()?;
                let total_len = <AlignedValue as Serializable>::serialized_size(a.as_ref())
                    + <AlignedValue as Serializable>::serialized_size(b.as_ref());
                gas = incr_gas(
                    gas,
                    if *cached {
                        cost_model.concatc_constant + cost_model.concatc_linear * total_len as u64
                    } else {
                        cost_model.concat_constant + cost_model.concat_linear * total_len as u64
                    },
                )?;
                stack.push((vnew(Strong, concat(b, a, *n)?.into()), CacheKey(None)));
            }
            Member => {
                let key = stack.pop().unwrap().0.as_cell()?;
                let map = stack.pop().unwrap().0.value;
                gas = incr_gas(
                    gas,
                    cost_model.member_constant
                        + cost_model.member_linear
                            * <AlignedValue as Serializable>::serialized_size(key.as_ref()) as u64
                        + cost_model.member_logarithmic * map.log_size() as u64,
                )?;
                stack.push((
                    vnew(
                        Strong,
                        AlignedValue::from(match &map {
                            StateValue::Map(map) => map.contains_key(&key),
                            _ => return Err(OnchainProgramError::TypeError),
                        })
                        .into(),
                    ),
                    CacheKey(None),
                ));
            }
            Rem { cached } => {
                let key = stack.pop().unwrap().0.as_cell()?;
                let container = stack.pop().unwrap();
                gas = incr_gas(
                    gas,
                    if *cached {
                        cost_model.remc_constant
                            + cost_model.remc_linear
                                * <AlignedValue as Serializable>::serialized_size(key.as_ref())
                                    as u64
                            + cost_model.remc_logarithmic * container.0.value.log_size() as u64
                    } else {
                        cost_model.rem_constant
                            + cost_model.rem_linear
                                * <AlignedValue as Serializable>::serialized_size(key.as_ref())
                                    as u64
                            + cost_model.rem_logarithmic * container.0.value.log_size() as u64
                    },
                )?;
                let cache_hit = cache.visit(&container.1.push_key(&key));
                if *cached && !cache_hit {
                    return Err(OnchainProgramError::CacheMiss);
                }
                let container_nxt = match &container.0.value {
                    StateValue::Map(m) => StateValue::Map(m.remove(&key)),
                    StateValue::BoundedMerkleTree(t) => {
                        StateValue::BoundedMerkleTree(t.update_hash(
                            (&**AsRef::<Value>::as_ref(&key)).try_into()?,
                            Default::default(),
                            (),
                        ))
                    }
                    _ => return Err(OnchainProgramError::TypeError),
                };
                stack.push((vnew(container.0.strength, container_nxt), container.1));
            }
            Dup { n } => {
                gas = incr_gas(gas, cost_model.dup)?;
                stack.push(stack[stack_len - *n as usize - 1].clone());
            }
            Swap { n } => {
                gas = incr_gas(gas, cost_model.swap)?;
                stack.swap(stack_len - 1, stack_len - 2 - *n as usize);
            }
            Idx {
                cached,
                push_path,
                path,
            } => {
                gas = incr_gas(
                    gas,
                    match (*cached, *push_path) {
                        (true, true) => cost_model.idxpc_constant,
                        (true, false) => cost_model.idxc_constant,
                        (false, true) => cost_model.idxp_constant,
                        (false, false) => cost_model.idx_constant,
                    },
                )?;
                let mut stack_keys = path
                    .iter()
                    .filter(|key| key == &&Key::Stack)
                    .map(|_| stack.pop().unwrap().0)
                    .collect::<Vec<_>>()
                    .into_iter();
                let mut i = 0;
                let refined_path = path.iter().map(|key| match key {
                    Key::Stack => {
                        i += 1;
                        stack_keys.next().unwrap()
                    }
                    Key::Value(v) => vnew(Weak, StateValue::Cell(Arc::new(v.clone()))),
                });
                let mut curr = stack.pop().unwrap();
                for key in refined_path {
                    let (linear_cost, log_cost) = match (*cached, *push_path) {
                        (true, true) => (cost_model.idxpc_linear, cost_model.idxpc_logarithmic),
                        (true, false) => (cost_model.idxc_linear, cost_model.idxc_logarithmic),
                        (false, true) => (cost_model.idxp_linear, cost_model.idxp_logarithmic),
                        (false, false) => (cost_model.idx_linear, cost_model.idx_logarithmic),
                    };
                    gas = incr_gas(
                        gas,
                        linear_cost * Serializable::serialized_size(&key.value) as u64
                            + log_cost * curr.0.value.log_size() as u64,
                    )?;
                    if *push_path {
                        stack.push(curr.clone());
                        stack.push((key.clone(), CacheKey(None)));
                    }
                    let key_val = key.as_cell_ref()?;
                    curr = idx(&curr, key_val, cache, *cached)?;
                }
                stack.push(curr);
            }
            Ins { cached, n } => {
                gas = incr_gas(
                    gas,
                    if *cached {
                        cost_model.insc_constant
                    } else {
                        cost_model.ins_constant
                    },
                )?;
                let mut curr = stack.pop().unwrap();
                for _ in 0..*n {
                    let key = stack.pop().unwrap().0;
                    let container = stack.pop().unwrap();
                    let (linear_cost, log_cost) = if *cached {
                        (cost_model.insc_linear, cost_model.insc_logarithmic)
                    } else {
                        (cost_model.ins_linear, cost_model.ins_logarithmic)
                    };
                    gas = incr_gas(
                        gas,
                        linear_cost * Serializable::serialized_size(&key.value) as u64
                            + log_cost * container.0.value.log_size() as u64,
                    )?;
                    let key_cell = key.as_cell_ref()?;
                    let VmValue {
                        strength: container_str,
                        value: container_val,
                    } = container.0;
                    let cache_hit = cache.visit(&container.1.push_key(key_cell));
                    if *cached && !cache_hit {
                        if let StateValue::Array(_) = &container_val {
                            // The miss is okay. We're overwriting an array cell, we never need to read it.
                        } else {
                            return Err(OnchainProgramError::CacheMiss);
                        }
                    }
                    let next = match &container_val {
                        StateValue::Array(arr) => {
                            let idx: u8 = (&**AsRef::<Value>::as_ref(&key_cell)).try_into()?;
                            if idx as usize >= arr.len() {
                                return Err(OnchainProgramError::TypeError);
                            }
                            let arr = arr.clone().insert(idx as usize, curr.0.value);
                            StateValue::Array(arr)
                        }
                        StateValue::Map(m) => {
                            StateValue::Map(m.insert(key_cell.clone(), curr.0.value))
                        }
                        StateValue::BoundedMerkleTree(ref t) => {
                            let idx = (&**AsRef::<Value>::as_ref(&key_cell)).try_into()?;
                            if idx < (1u64 << t.height()) {
                                StateValue::BoundedMerkleTree(
                                    t.update_hash(
                                        idx,
                                        (&**AsRef::<Value>::as_ref(&curr.0.as_cell_ref()?))
                                            .try_into()?,
                                        (),
                                    ),
                                )
                            } else {
                                return Err(OnchainProgramError::BoundsExceeded);
                            }
                        }
                        _ => return Err(OnchainProgramError::TypeError),
                    };
                    curr.0 = vnew(curr.0.strength & container_str, next);
                    curr.1 = container.1;
                }
                stack.push(curr);
            }
        }
        let skip = 1 + match op {
            Branch { skip } => {
                let a = stack.pop().unwrap().0.as_cell()?;
                if AsRef::<Value>::as_ref(&a).0.len() == 1
                    && AsRef::<Value>::as_ref(&a).0[0].0.is_empty()
                {
                    0
                } else {
                    *skip as usize
                }
            }
            Jmp { skip } => *skip as usize,
            _ => 0,
        };
        if skip > program.len() {
            return Err(OnchainProgramError::RanPastProgramEnd);
        }
        program = &program[skip..];
    }
    Ok(VmResults {
        stack: stack.into_iter().map(|(a, _)| a).collect(),
        events,
        gas_cost: gas,
    })
}
