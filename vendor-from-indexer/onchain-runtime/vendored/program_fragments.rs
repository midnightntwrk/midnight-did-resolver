#[macro_export]
macro_rules! kernel_claim_zswap_nullifier {
  ($f:expr, $fcached:expr, $nul:expr) => {
    [
      Op::Swap { n: 0.try_into().unwrap() },
      Op::Idx { cached: true.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(0 as u8).into())].try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($nul.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Null.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: 2.try_into().unwrap() },
      Op::Swap { n: 0.try_into().unwrap() },
    ]
  };
}
pub use kernel_claim_zswap_nullifier;
#[macro_export]
macro_rules! kernel_claim_zswap_coin_spend {
  ($f:expr, $fcached:expr, $note:expr) => {
    [
      Op::Swap { n: 0.try_into().unwrap() },
      Op::Idx { cached: true.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(2 as u8).into())].try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($note.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Null.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: 2.try_into().unwrap() },
      Op::Swap { n: 0.try_into().unwrap() },
    ]
  };
}
pub use kernel_claim_zswap_coin_spend;
#[macro_export]
macro_rules! kernel_claim_zswap_coin_receive {
  ($f:expr, $fcached:expr, $note:expr) => {
    [
      Op::Swap { n: 0.try_into().unwrap() },
      Op::Idx { cached: true.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(1 as u8).into())].try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($note.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Null.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: 2.try_into().unwrap() },
      Op::Swap { n: 0.try_into().unwrap() },
    ]
  };
}
pub use kernel_claim_zswap_coin_receive;
#[macro_export]
macro_rules! kernel_claim_contract_call {
  ($f:expr, $fcached:expr, $addr:expr, $entry_point:expr, $comm:expr) => {
    [
      Op::Swap { n: 0.try_into().unwrap() },
      Op::Idx { cached: true.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(3 as u8).into())].try_into().unwrap() },
      Op::Dup { n: 0.try_into().unwrap() },
      Op::Size,
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new(AlignedValue::concat([AlignedValue::from($addr.clone()), AlignedValue::from($entry_point.clone()), AlignedValue::from($comm.clone())].iter()).try_into().unwrap())).try_into().unwrap() },
      Op::Concat { cached: true.try_into().unwrap(), n: 160.try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Null.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: 2.try_into().unwrap() },
      Op::Swap { n: 0.try_into().unwrap() },
    ]
  };
}
pub use kernel_claim_contract_call;
#[macro_export]
macro_rules! kernel_checkpoint {
  ($f:expr, $fcached:expr) => {
    [
      Op::Ckpt,
    ]
  };
}
pub use kernel_checkpoint;
#[macro_export]
macro_rules! kernel_mint {
  ($f:expr, $fcached:expr, $domain_sep:expr, $amount:expr) => {
    [
      Op::Swap { n: 0.try_into().unwrap() },
      Op::Idx { cached: true.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(4 as u8).into())].try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($domain_sep.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Dup { n: 1.try_into().unwrap() },
      Op::Dup { n: 1.try_into().unwrap() },
      Op::Member,
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($amount.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Swap { n: 0.try_into().unwrap() },
      Op::Neg,
      Op::Branch { skip: 4.try_into().unwrap() },
      Op::Dup { n: 2.try_into().unwrap() },
      Op::Dup { n: 2.try_into().unwrap() },
      Op::Idx { cached: true.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Stack].try_into().unwrap() },
      Op::Add,
      Op::Ins { cached: true.try_into().unwrap(), n: 2.try_into().unwrap() },
      Op::Swap { n: 0.try_into().unwrap() },
    ]
  };
}
pub use kernel_mint;
#[macro_export]
macro_rules! kernel_self {
  ($f:expr, $fcached:expr) => {
    [
      Op::Dup { n: 2.try_into().unwrap() },
      Op::Idx { cached: true.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(0 as u8).into())].try_into().unwrap() },
      Op::Popeq { cached: true.try_into().unwrap(), result: ().try_into().unwrap() },
    ]
  };
}
pub use kernel_self;
#[macro_export]
macro_rules! Counter_read {
  ($f:expr, $fcached:expr) => {
    [
      Op::Dup { n: 0.try_into().unwrap() },
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: false.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Popeq { cached: true.try_into().unwrap(), result: ().try_into().unwrap() },
    ]
  };
}
pub use Counter_read;
#[macro_export]
macro_rules! Counter_less_than {
  ($f:expr, $fcached:expr, $threshold:expr) => {
    [
      Op::Dup { n: 0.try_into().unwrap() },
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: false.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($threshold.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Lt,
      Op::Popeq { cached: true.try_into().unwrap(), result: ().try_into().unwrap() },
    ]
  };
}
pub use Counter_less_than;
#[macro_export]
macro_rules! Counter_increment {
  ($f:expr, $fcached:expr, $amount:expr) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Addi { immediate: u32::try_from($amount.clone()).unwrap().try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: ($f.clone().len() as u8).try_into().unwrap() },
    ]
  };
}
pub use Counter_increment;
#[macro_export]
macro_rules! Counter_decrement {
  ($f:expr, $fcached:expr, $amount:expr) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Subi { immediate: u32::try_from($amount.clone()).unwrap().try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: ($f.clone().len() as u8).try_into().unwrap() },
    ]
  };
}
pub use Counter_decrement;
#[macro_export]
macro_rules! Counter_reset_to_default {
  ($f:expr, $fcached:expr) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().iter().cloned().rev().collect::<Vec<_>>().iter().cloned().skip(1).collect::<Vec<_>>().iter().cloned().rev().collect::<Vec<_>>().try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($f.clone().iter().cloned().rev().collect::<Vec<_>>()[0].clone().try_into().unwrap())).try_into().unwrap() },
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Cell(Arc::new(AlignedValue::from(0 as u64).try_into().unwrap())).try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: (($f.clone().len() as u8) - 1).try_into().unwrap() },
    ]
  };
}
pub use Counter_reset_to_default;
#[macro_export]
macro_rules! Cell_read {
  ($f:expr, $fcached:expr, $value_type:ty) => {
    [
      Op::Dup { n: 0.try_into().unwrap() },
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: false.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Popeq { cached: $fcached.try_into().unwrap(), result: ().try_into().unwrap() },
    ]
  };
}
pub use Cell_read;
#[macro_export]
macro_rules! Cell_write {
  ($f:expr, $fcached:expr, $value_type:ty, $value:expr) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().iter().cloned().rev().collect::<Vec<_>>().iter().cloned().skip(1).collect::<Vec<_>>().iter().cloned().rev().collect::<Vec<_>>().try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($f.clone().iter().cloned().rev().collect::<Vec<_>>()[0].clone().try_into().unwrap())).try_into().unwrap() },
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Cell(Arc::new($value.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: (($f.clone().len() as u8) - 1).try_into().unwrap() },
    ]
  };
}
pub use Cell_write;
#[macro_export]
macro_rules! Cell_reset_to_default {
  ($f:expr, $fcached:expr, $value_type:ty) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().iter().cloned().rev().collect::<Vec<_>>().iter().cloned().skip(1).collect::<Vec<_>>().iter().cloned().rev().collect::<Vec<_>>().try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($f.clone().iter().cloned().rev().collect::<Vec<_>>()[0].clone().try_into().unwrap())).try_into().unwrap() },
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Cell(Arc::new(AlignedValue::from(<$value_type>::default()).try_into().unwrap())).try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: (($f.clone().len() as u8) - 1).try_into().unwrap() },
    ]
  };
}
pub use Cell_reset_to_default;
#[macro_export]
macro_rules! Cell_write_coin {
  ($f:expr, $fcached:expr, $value_type:ty, $coin:expr, $recipient:expr) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().iter().cloned().rev().collect::<Vec<_>>().iter().cloned().skip(1).collect::<Vec<_>>().iter().cloned().rev().collect::<Vec<_>>().try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($f.clone().iter().cloned().rev().collect::<Vec<_>>()[0].clone().try_into().unwrap())).try_into().unwrap() },
      Op::Dup { n: (3 + ((($f.clone().len() as u8) - 1) * 2)).try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($coin.clone().commitment(&$recipient.clone()).try_into().unwrap())).try_into().unwrap() },
      Op::Idx { cached: true.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(1 as u8).into()), Key::Stack].try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($coin.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Swap { n: 0.try_into().unwrap() },
      Op::Concat { cached: true.try_into().unwrap(), n: 91.try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: (($f.clone().len() as u8) - 1).try_into().unwrap() },
    ]
  };
}
pub use Cell_write_coin;
#[macro_export]
macro_rules! Set_reset_to_default {
  ($f:expr, $fcached:expr, $value_type:ty) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().iter().cloned().rev().collect::<Vec<_>>().iter().cloned().skip(1).collect::<Vec<_>>().iter().cloned().rev().collect::<Vec<_>>().try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($f.clone().iter().cloned().rev().collect::<Vec<_>>()[0].clone().try_into().unwrap())).try_into().unwrap() },
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Map([].iter().cloned().collect()).try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: (($f.clone().len() as u8) - 1).try_into().unwrap() },
    ]
  };
}
pub use Set_reset_to_default;
#[macro_export]
macro_rules! Set_is_empty {
  ($f:expr, $fcached:expr, $value_type:ty) => {
    [
      Op::Dup { n: 0.try_into().unwrap() },
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: false.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Size,
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new(AlignedValue::from(0 as u64).try_into().unwrap())).try_into().unwrap() },
      Op::Eq,
      Op::Popeq { cached: true.try_into().unwrap(), result: ().try_into().unwrap() },
    ]
  };
}
pub use Set_is_empty;
#[macro_export]
macro_rules! Set_size {
  ($f:expr, $fcached:expr, $value_type:ty) => {
    [
      Op::Dup { n: 0.try_into().unwrap() },
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: false.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Size,
      Op::Popeq { cached: true.try_into().unwrap(), result: ().try_into().unwrap() },
    ]
  };
}
pub use Set_size;
#[macro_export]
macro_rules! Set_member {
  ($f:expr, $fcached:expr, $value_type:ty, $elem:expr) => {
    [
      Op::Dup { n: 0.try_into().unwrap() },
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: false.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($elem.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Member,
      Op::Popeq { cached: true.try_into().unwrap(), result: ().try_into().unwrap() },
    ]
  };
}
pub use Set_member;
#[macro_export]
macro_rules! Set_insert {
  ($f:expr, $fcached:expr, $value_type:ty, $elem:expr) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($elem.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Null.try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: ($f.clone().len() as u8).try_into().unwrap() },
    ]
  };
}
pub use Set_insert;
#[macro_export]
macro_rules! Set_remove {
  ($f:expr, $fcached:expr, $value_type:ty, $elem:expr) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($elem.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Rem { cached: false.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: ($f.clone().len() as u8).try_into().unwrap() },
    ]
  };
}
pub use Set_remove;
#[macro_export]
macro_rules! Set_insert_coin {
  ($f:expr, $fcached:expr, $value_type:ty, $coin:expr, $recipient:expr) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Dup { n: (2 + (($f.clone().len() as u8) * 2)).try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($coin.clone().commitment(&$recipient.clone()).try_into().unwrap())).try_into().unwrap() },
      Op::Idx { cached: true.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(1 as u8).into()), Key::Stack].try_into().unwrap() },
      Op::Concat { cached: true.try_into().unwrap(), n: 91.try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($coin.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Swap { n: 0.try_into().unwrap() },
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Null.try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: ($f.clone().len() as u8).try_into().unwrap() },
    ]
  };
}
pub use Set_insert_coin;
#[macro_export]
macro_rules! Map_reset_to_default {
  ($f:expr, $fcached:expr, $key_type:ty, $value_type:ty) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().iter().cloned().rev().collect::<Vec<_>>().iter().cloned().skip(1).collect::<Vec<_>>().iter().cloned().rev().collect::<Vec<_>>().try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($f.clone().iter().cloned().rev().collect::<Vec<_>>()[0].clone().try_into().unwrap())).try_into().unwrap() },
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Map([].iter().cloned().collect()).try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: (($f.clone().len() as u8) - 1).try_into().unwrap() },
    ]
  };
}
pub use Map_reset_to_default;
#[macro_export]
macro_rules! Map_is_empty {
  ($f:expr, $fcached:expr, $key_type:ty, $value_type:ty) => {
    [
      Op::Dup { n: 0.try_into().unwrap() },
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: false.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Size,
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new(AlignedValue::from(0 as u64).try_into().unwrap())).try_into().unwrap() },
      Op::Eq,
      Op::Popeq { cached: true.try_into().unwrap(), result: ().try_into().unwrap() },
    ]
  };
}
pub use Map_is_empty;
#[macro_export]
macro_rules! Map_size {
  ($f:expr, $fcached:expr, $key_type:ty, $value_type:ty) => {
    [
      Op::Dup { n: 0.try_into().unwrap() },
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: false.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Size,
      Op::Popeq { cached: true.try_into().unwrap(), result: ().try_into().unwrap() },
    ]
  };
}
pub use Map_size;
#[macro_export]
macro_rules! Map_member {
  ($f:expr, $fcached:expr, $key_type:ty, $value_type:ty, $key:expr) => {
    [
      Op::Dup { n: 0.try_into().unwrap() },
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: false.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($key.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Member,
      Op::Popeq { cached: true.try_into().unwrap(), result: ().try_into().unwrap() },
    ]
  };
}
pub use Map_member;
#[macro_export]
macro_rules! Map_lookup {
  ($f:expr, $fcached:expr, $key_type:ty, $value_type:ty, $key:expr) => {
    [
      Op::Dup { n: 0.try_into().unwrap() },
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: false.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Value($key.clone().into())].try_into().unwrap() },
      Op::Popeq { cached: false.try_into().unwrap(), result: ().try_into().unwrap() },
    ]
  };
}
pub use Map_lookup;
#[macro_export]
macro_rules! Map_insert {
  ($f:expr, $fcached:expr, $key_type:ty, $value_type:ty, $key:expr, $value:expr) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($key.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Cell(Arc::new($value.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: ($f.clone().len() as u8).try_into().unwrap() },
    ]
  };
}
pub use Map_insert;
#[macro_export]
macro_rules! Map_insert_default {
  ($f:expr, $fcached:expr, $key_type:ty, $value_type:ty, $key:expr) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($key.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Cell(Arc::new(AlignedValue::from(<$value_type>::default()).try_into().unwrap())).try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: ($f.clone().len() as u8).try_into().unwrap() },
    ]
  };
}
pub use Map_insert_default;
#[macro_export]
macro_rules! Map_remove {
  ($f:expr, $fcached:expr, $key_type:ty, $value_type:ty, $key:expr) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($key.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Rem { cached: false.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: ($f.clone().len() as u8).try_into().unwrap() },
    ]
  };
}
pub use Map_remove;
#[macro_export]
macro_rules! Map_insert_coin {
  ($f:expr, $fcached:expr, $key_type:ty, $value_type:ty, $key:expr, $coin:expr, $recipient:expr) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($key.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Dup { n: (2 + (($f.clone().len() as u8) * 2)).try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($coin.clone().commitment(&$recipient.clone()).try_into().unwrap())).try_into().unwrap() },
      Op::Idx { cached: true.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(1 as u8).into()), Key::Stack].try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($coin.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Swap { n: 0.try_into().unwrap() },
      Op::Concat { cached: true.try_into().unwrap(), n: 91.try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: ($f.clone().len() as u8).try_into().unwrap() },
    ]
  };
}
pub use Map_insert_coin;
#[macro_export]
macro_rules! List_reset_to_default {
  ($f:expr, $fcached:expr, $value_type:ty) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().iter().cloned().rev().collect::<Vec<_>>().iter().cloned().skip(1).collect::<Vec<_>>().iter().cloned().rev().collect::<Vec<_>>().try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($f.clone().iter().cloned().rev().collect::<Vec<_>>()[0].clone().try_into().unwrap())).try_into().unwrap() },
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Array(vec![StateValue::Null, StateValue::Null, StateValue::Cell(Arc::new(AlignedValue::from(0 as u64).try_into().unwrap()))].into()).try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: (($f.clone().len() as u8) - 1).try_into().unwrap() },
    ]
  };
}
pub use List_reset_to_default;
#[macro_export]
macro_rules! List_is_empty {
  ($f:expr, $fcached:expr, $value_type:ty) => {
    [
      Op::Dup { n: 0.try_into().unwrap() },
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: false.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(1 as u8).into())].try_into().unwrap() },
      Op::Type,
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new(AlignedValue::from(1 as u8).try_into().unwrap())).try_into().unwrap() },
      Op::Eq,
      Op::Popeq { cached: true.try_into().unwrap(), result: ().try_into().unwrap() },
    ]
  };
}
pub use List_is_empty;
#[macro_export]
macro_rules! List_length {
  ($f:expr, $fcached:expr, $value_type:ty) => {
    [
      Op::Dup { n: 0.try_into().unwrap() },
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: false.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(2 as u8).into())].try_into().unwrap() },
      Op::Popeq { cached: true.try_into().unwrap(), result: ().try_into().unwrap() },
    ]
  };
}
pub use List_length;
#[macro_export]
macro_rules! List_head {
  ($f:expr, $fcached:expr, $value_type:ty) => {
    [
      Op::Dup { n: 0.try_into().unwrap() },
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: false.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(0 as u8).into())].try_into().unwrap() },
      Op::Dup { n: 0.try_into().unwrap() },
      Op::Type,
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new(AlignedValue::from(1 as u8).try_into().unwrap())).try_into().unwrap() },
      Op::Eq,
      Op::Branch { skip: 4.try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new(AlignedValue::from(1 as u8).try_into().unwrap())).try_into().unwrap() },
      Op::Swap { n: 0.try_into().unwrap() },
      Op::Concat { cached: false.try_into().unwrap(), n: (2 + (<$value_type>::alignment().max_aligned_size() as u32)).try_into().unwrap() },
      Op::Jmp { skip: 2.try_into().unwrap() },
      Op::Pop,
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new(AlignedValue::concat([AlignedValue::from(AlignedValue::from(0 as u8)), AlignedValue::from(AlignedValue::from(<$value_type>::default()))].iter()).try_into().unwrap())).try_into().unwrap() },
      Op::Popeq { cached: true.try_into().unwrap(), result: ().try_into().unwrap() },
    ]
  };
}
pub use List_head;
#[macro_export]
macro_rules! List_pop_front {
  ($f:expr, $fcached:expr, $value_type:ty) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(1 as u8).into())].try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: ($f.clone().len() as u8).try_into().unwrap() },
    ]
  };
}
pub use List_pop_front;
#[macro_export]
macro_rules! List_push_front {
  ($f:expr, $fcached:expr, $value_type:ty, $value:expr) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Dup { n: 0.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(2 as u8).into())].try_into().unwrap() },
      Op::Addi { immediate: 1.try_into().unwrap() },
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Array(vec![StateValue::Cell(Arc::new($value.clone().try_into().unwrap())), StateValue::Null, StateValue::Null].into()).try_into().unwrap() },
      Op::Swap { n: 0.try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new(AlignedValue::from(2 as u8).try_into().unwrap())).try_into().unwrap() },
      Op::Swap { n: 0.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Swap { n: 0.try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new(AlignedValue::from(1 as u8).try_into().unwrap())).try_into().unwrap() },
      Op::Swap { n: 0.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: (($f.clone().len() as u8) + 1).try_into().unwrap() },
    ]
  };
}
pub use List_push_front;
#[macro_export]
macro_rules! List_push_front_coin {
  ($f:expr, $fcached:expr, $value_type:ty, $coin:expr, $recipient:expr) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Dup { n: 0.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(2 as u8).into())].try_into().unwrap() },
      Op::Addi { immediate: 1.try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new(AlignedValue::from(0 as u8).try_into().unwrap())).try_into().unwrap() },
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Array(vec![StateValue::Null, StateValue::Null, StateValue::Null].into()).try_into().unwrap() },
      Op::Dup { n: (4 + (($f.clone().len() as u8) * 2)).try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($coin.clone().commitment(&$recipient.clone()).try_into().unwrap())).try_into().unwrap() },
      Op::Idx { cached: true.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(1 as u8).into()), Key::Stack].try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($coin.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Swap { n: 0.try_into().unwrap() },
      Op::Concat { cached: true.try_into().unwrap(), n: 91.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Swap { n: 0.try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new(AlignedValue::from(2 as u8).try_into().unwrap())).try_into().unwrap() },
      Op::Swap { n: 0.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Swap { n: 0.try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new(AlignedValue::from(1 as u8).try_into().unwrap())).try_into().unwrap() },
      Op::Swap { n: 0.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: (($f.clone().len() as u8) + 1).try_into().unwrap() },
    ]
  };
}
pub use List_push_front_coin;
#[macro_export]
macro_rules! MerkleTree_reset_to_default {
  ($f:expr, $fcached:expr, $nat:literal, $value_type:ty) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().iter().cloned().rev().collect::<Vec<_>>().iter().cloned().skip(1).collect::<Vec<_>>().iter().cloned().rev().collect::<Vec<_>>().try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($f.clone().iter().cloned().rev().collect::<Vec<_>>()[0].clone().try_into().unwrap())).try_into().unwrap() },
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Array(vec![StateValue::BoundedMerkleTree(MerkleTree::blank($nat)), StateValue::Cell(Arc::new(AlignedValue::from(0 as u64).try_into().unwrap()))].into()).try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: (($f.clone().len() as u8) - 1).try_into().unwrap() },
    ]
  };
}
pub use MerkleTree_reset_to_default;
#[macro_export]
macro_rules! MerkleTree_is_full {
  ($f:expr, $fcached:expr, $nat:literal, $value_type:ty) => {
    [
      Op::Dup { n: 0.try_into().unwrap() },
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: false.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(1 as u8).into())].try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new(AlignedValue::from((2 as u64).pow($nat) as u64).try_into().unwrap())).try_into().unwrap() },
      Op::Lt,
      Op::Neg,
      Op::Popeq { cached: true.try_into().unwrap(), result: ().try_into().unwrap() },
    ]
  };
}
pub use MerkleTree_is_full;
#[macro_export]
macro_rules! MerkleTree_check_root {
  ($f:expr, $fcached:expr, $nat:literal, $value_type:ty, $rt:expr) => {
    [
      Op::Dup { n: 0.try_into().unwrap() },
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: false.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(0 as u8).into())].try_into().unwrap() },
      Op::Root,
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($rt.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Eq,
      Op::Popeq { cached: true.try_into().unwrap(), result: ().try_into().unwrap() },
    ]
  };
}
pub use MerkleTree_check_root;
#[macro_export]
macro_rules! MerkleTree_insert {
  ($f:expr, $fcached:expr, $nat:literal, $value_type:ty, $item:expr) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(0 as u8).into())].try_into().unwrap() },
      Op::Dup { n: 2.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(1 as u8).into())].try_into().unwrap() },
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Cell(Arc::new(leaf_hash(&ValueReprAlignedValue(AlignedValue::from($item.clone()))).try_into().unwrap())).try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(1 as u8).into())].try_into().unwrap() },
      Op::Addi { immediate: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: (($f.clone().len() as u8) + 1).try_into().unwrap() },
    ]
  };
}
pub use MerkleTree_insert;
#[macro_export]
macro_rules! MerkleTree_insert_index {
  ($f:expr, $fcached:expr, $nat:literal, $value_type:ty, $item:expr, $index:expr) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(0 as u8).into())].try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($index.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Cell(Arc::new(leaf_hash(&ValueReprAlignedValue(AlignedValue::from($item.clone()))).try_into().unwrap())).try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 2.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(1 as u8).into())].try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($index.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Addi { immediate: 1.try_into().unwrap() },
      Op::Dup { n: 1.try_into().unwrap() },
      Op::Dup { n: 1.try_into().unwrap() },
      Op::Lt,
      Op::Branch { skip: 2.try_into().unwrap() },
      Op::Pop,
      Op::Jmp { skip: 2.try_into().unwrap() },
      Op::Swap { n: 0.try_into().unwrap() },
      Op::Pop,
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: ($f.clone().len() as u8).try_into().unwrap() },
    ]
  };
}
pub use MerkleTree_insert_index;
#[macro_export]
macro_rules! MerkleTree_insert_hash {
  ($f:expr, $fcached:expr, $nat:literal, $value_type:ty, $hash:expr) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(0 as u8).into())].try_into().unwrap() },
      Op::Dup { n: 2.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(1 as u8).into())].try_into().unwrap() },
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Cell(Arc::new($hash.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(1 as u8).into())].try_into().unwrap() },
      Op::Addi { immediate: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: (($f.clone().len() as u8) + 1).try_into().unwrap() },
    ]
  };
}
pub use MerkleTree_insert_hash;
#[macro_export]
macro_rules! MerkleTree_insert_hash_index {
  ($f:expr, $fcached:expr, $nat:literal, $value_type:ty, $hash:expr, $index:expr) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(0 as u8).into())].try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($index.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Cell(Arc::new($hash.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 2.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(1 as u8).into())].try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($index.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Addi { immediate: 1.try_into().unwrap() },
      Op::Dup { n: 1.try_into().unwrap() },
      Op::Dup { n: 1.try_into().unwrap() },
      Op::Lt,
      Op::Branch { skip: 2.try_into().unwrap() },
      Op::Pop,
      Op::Jmp { skip: 2.try_into().unwrap() },
      Op::Swap { n: 0.try_into().unwrap() },
      Op::Pop,
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: ($f.clone().len() as u8).try_into().unwrap() },
    ]
  };
}
pub use MerkleTree_insert_hash_index;
#[macro_export]
macro_rules! MerkleTree_insert_index_default {
  ($f:expr, $fcached:expr, $nat:literal, $value_type:ty, $index:expr) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($index.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Cell(Arc::new(leaf_hash(&ValueReprAlignedValue(AlignedValue::from(AlignedValue::from(<$value_type>::default())))).try_into().unwrap())).try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 2.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(1 as u8).into())].try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($index.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Addi { immediate: 1.try_into().unwrap() },
      Op::Dup { n: 1.try_into().unwrap() },
      Op::Dup { n: 1.try_into().unwrap() },
      Op::Lt,
      Op::Branch { skip: 2.try_into().unwrap() },
      Op::Pop,
      Op::Jmp { skip: 2.try_into().unwrap() },
      Op::Swap { n: 0.try_into().unwrap() },
      Op::Pop,
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: ($f.clone().len() as u8).try_into().unwrap() },
    ]
  };
}
pub use MerkleTree_insert_index_default;
#[macro_export]
macro_rules! HistoricMerkleTree_reset_to_default {
  ($f:expr, $fcached:expr, $nat:literal, $value_type:ty) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().iter().cloned().rev().collect::<Vec<_>>().iter().cloned().skip(1).collect::<Vec<_>>().iter().cloned().rev().collect::<Vec<_>>().try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($f.clone().iter().cloned().rev().collect::<Vec<_>>()[0].clone().try_into().unwrap())).try_into().unwrap() },
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Array(vec![StateValue::BoundedMerkleTree(MerkleTree::blank($nat)), StateValue::Cell(Arc::new(AlignedValue::from(0 as u64).try_into().unwrap())), StateValue::Map([].iter().cloned().collect())].into()).try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(2 as u8).into())].try_into().unwrap() },
      Op::Dup { n: 2.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(0 as u8).into())].try_into().unwrap() },
      Op::Root,
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Null.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: 2.try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: (($f.clone().len() as u8) - 1).try_into().unwrap() },
    ]
  };
}
pub use HistoricMerkleTree_reset_to_default;
#[macro_export]
macro_rules! HistoricMerkleTree_is_full {
  ($f:expr, $fcached:expr, $nat:literal, $value_type:ty) => {
    [
      Op::Dup { n: 0.try_into().unwrap() },
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: false.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(1 as u8).into())].try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new(AlignedValue::from((2 as u64).pow($nat) as u64).try_into().unwrap())).try_into().unwrap() },
      Op::Lt,
      Op::Neg,
      Op::Popeq { cached: true.try_into().unwrap(), result: ().try_into().unwrap() },
    ]
  };
}
pub use HistoricMerkleTree_is_full;
#[macro_export]
macro_rules! HistoricMerkleTree_check_root {
  ($f:expr, $fcached:expr, $nat:literal, $value_type:ty, $rt:expr) => {
    [
      Op::Dup { n: 0.try_into().unwrap() },
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: false.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(2 as u8).into())].try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($rt.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Member,
      Op::Popeq { cached: true.try_into().unwrap(), result: ().try_into().unwrap() },
    ]
  };
}
pub use HistoricMerkleTree_check_root;
#[macro_export]
macro_rules! HistoricMerkleTree_insert {
  ($f:expr, $fcached:expr, $nat:literal, $value_type:ty, $item:expr) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(0 as u8).into())].try_into().unwrap() },
      Op::Dup { n: 2.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(1 as u8).into())].try_into().unwrap() },
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Cell(Arc::new(leaf_hash(&ValueReprAlignedValue(AlignedValue::from($item.clone()))).try_into().unwrap())).try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(1 as u8).into())].try_into().unwrap() },
      Op::Addi { immediate: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(2 as u8).into())].try_into().unwrap() },
      Op::Dup { n: 2.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(0 as u8).into())].try_into().unwrap() },
      Op::Root,
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Null.try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: (($f.clone().len() as u8) + 1).try_into().unwrap() },
    ]
  };
}
pub use HistoricMerkleTree_insert;
#[macro_export]
macro_rules! HistoricMerkleTree_insert_index {
  ($f:expr, $fcached:expr, $nat:literal, $value_type:ty, $item:expr, $index:expr) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(0 as u8).into())].try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($index.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Cell(Arc::new(leaf_hash(&ValueReprAlignedValue(AlignedValue::from($item.clone()))).try_into().unwrap())).try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 2.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(1 as u8).into())].try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($index.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Addi { immediate: 1.try_into().unwrap() },
      Op::Dup { n: 1.try_into().unwrap() },
      Op::Dup { n: 1.try_into().unwrap() },
      Op::Lt,
      Op::Branch { skip: 2.try_into().unwrap() },
      Op::Pop,
      Op::Jmp { skip: 2.try_into().unwrap() },
      Op::Swap { n: 0.try_into().unwrap() },
      Op::Pop,
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(2 as u8).into())].try_into().unwrap() },
      Op::Dup { n: 2.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(0 as u8).into())].try_into().unwrap() },
      Op::Root,
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Null.try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: (($f.clone().len() as u8) + 1).try_into().unwrap() },
    ]
  };
}
pub use HistoricMerkleTree_insert_index;
#[macro_export]
macro_rules! HistoricMerkleTree_insert_hash {
  ($f:expr, $fcached:expr, $nat:literal, $value_type:ty, $hash:expr) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(0 as u8).into())].try_into().unwrap() },
      Op::Dup { n: 2.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(1 as u8).into())].try_into().unwrap() },
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Cell(Arc::new($hash.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(1 as u8).into())].try_into().unwrap() },
      Op::Addi { immediate: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(2 as u8).into())].try_into().unwrap() },
      Op::Dup { n: 2.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(0 as u8).into())].try_into().unwrap() },
      Op::Root,
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Null.try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: (($f.clone().len() as u8) + 1).try_into().unwrap() },
    ]
  };
}
pub use HistoricMerkleTree_insert_hash;
#[macro_export]
macro_rules! HistoricMerkleTree_insert_hash_index {
  ($f:expr, $fcached:expr, $nat:literal, $value_type:ty, $hash:expr, $index:expr) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(0 as u8).into())].try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($index.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Cell(Arc::new($hash.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 2.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(1 as u8).into())].try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($index.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Addi { immediate: 1.try_into().unwrap() },
      Op::Dup { n: 1.try_into().unwrap() },
      Op::Dup { n: 1.try_into().unwrap() },
      Op::Lt,
      Op::Branch { skip: 2.try_into().unwrap() },
      Op::Pop,
      Op::Jmp { skip: 2.try_into().unwrap() },
      Op::Swap { n: 0.try_into().unwrap() },
      Op::Pop,
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(2 as u8).into())].try_into().unwrap() },
      Op::Dup { n: 2.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(0 as u8).into())].try_into().unwrap() },
      Op::Root,
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Null.try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: (($f.clone().len() as u8) + 1).try_into().unwrap() },
    ]
  };
}
pub use HistoricMerkleTree_insert_hash_index;
#[macro_export]
macro_rules! HistoricMerkleTree_insert_index_default {
  ($f:expr, $fcached:expr, $nat:literal, $value_type:ty, $index:expr) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(0 as u8).into())].try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($index.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Cell(Arc::new(leaf_hash(&ValueReprAlignedValue(AlignedValue::from(AlignedValue::from(<$value_type>::default())))).try_into().unwrap())).try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 2.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(1 as u8).into())].try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new($index.clone().try_into().unwrap())).try_into().unwrap() },
      Op::Addi { immediate: 1.try_into().unwrap() },
      Op::Dup { n: 1.try_into().unwrap() },
      Op::Dup { n: 1.try_into().unwrap() },
      Op::Lt,
      Op::Branch { skip: 2.try_into().unwrap() },
      Op::Pop,
      Op::Jmp { skip: 2.try_into().unwrap() },
      Op::Swap { n: 0.try_into().unwrap() },
      Op::Pop,
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: true.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(2 as u8).into())].try_into().unwrap() },
      Op::Dup { n: 2.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(0 as u8).into())].try_into().unwrap() },
      Op::Root,
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Null.try_into().unwrap() },
      Op::Ins { cached: false.try_into().unwrap(), n: 1.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: (($f.clone().len() as u8) + 1).try_into().unwrap() },
    ]
  };
}
pub use HistoricMerkleTree_insert_index_default;
#[macro_export]
macro_rules! HistoricMerkleTree_reset_history {
  ($f:expr, $fcached:expr, $nat:literal, $value_type:ty) => {
    [
      Op::Idx { cached: $fcached.try_into().unwrap(), push_path: true.try_into().unwrap(), path: $f.clone().try_into().unwrap() },
      Op::Push { storage: false.try_into().unwrap(), value: StateValue::Cell(Arc::new(AlignedValue::from(2 as u8).try_into().unwrap())).try_into().unwrap() },
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Map([].iter().cloned().collect()).try_into().unwrap() },
      Op::Dup { n: 2.try_into().unwrap() },
      Op::Idx { cached: false.try_into().unwrap(), push_path: false.try_into().unwrap(), path: vec![Key::Value(AlignedValue::from(0 as u8).into())].try_into().unwrap() },
      Op::Root,
      Op::Push { storage: true.try_into().unwrap(), value: StateValue::Null.try_into().unwrap() },
      Op::Ins { cached: true.try_into().unwrap(), n: (($f.clone().len() as u8) + 2).try_into().unwrap() },
    ]
  };
}
pub use HistoricMerkleTree_reset_history;
