use crate::base_crypto::fab::{InvalidBuiltinDecode, Value, ValueSlice};
use crate::base_crypto::hash::HashOutput;
use crate::cost_model::CostModel;
use crate::error::TranscriptRejected;
use crate::ops::Op;
use crate::result_mode::ResultMode;
use crate::serialize::{self, Deserializable, Serializable, Versioned};
#[cfg(all(feature = "proptest", test))]
use crate::serialize::{NetworkId, deserialize, serialize, serialized_size};
use crate::state::{ContractState, StateValue};
#[cfg(feature = "proptest")]
use crate::storage::serialize::randomised_serialization_test;
use crate::storage::storage::Map;
use crate::transcript::Transcript;
use crate::transient_crypto::curve::Fr;
use crate::vm::run_program;
use crate::vm_value::{ValueStrength, VmValue};
use coin_structure::coin::{
    Commitment as CoinCommitment, Info as CoinInfo, Nullifier, QualifiedInfo as QualifiedCoinInfo,
};
use coin_structure::contract::Address as ContractAddress;
use coin_structure::transfer::Recipient;
use derive_where::derive_where;
#[cfg(feature = "serde")]
use hex::FromHexError;
#[cfg(feature = "serde")]
use hex::{FromHex, ToHex};
use onchain_vm::storage::db::DB;
#[cfg(feature = "proptest")]
use proptest_derive::Arbitrary;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::Arc;

// Need to: Convert to SerdeBlockContext / SerdeEffects

#[cfg(feature = "serde")]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
struct SerdeBlockContext {
    seconds_since_epoch: u64,
    seconds_since_epoch_err: u32,
    block_hash: String,
}

#[cfg(feature = "serde")]
impl From<BlockContext> for SerdeBlockContext {
    fn from(ctxt: BlockContext) -> SerdeBlockContext {
        SerdeBlockContext {
            seconds_since_epoch: ctxt.seconds_since_epoch,
            seconds_since_epoch_err: ctxt.seconds_since_epoch_err,
            block_hash: ctxt.block_hash.0.encode_hex(),
        }
    }
}

#[cfg(feature = "serde")]
impl TryFrom<SerdeBlockContext> for BlockContext {
    type Error = FromHexError;

    fn try_from(ctxt: SerdeBlockContext) -> Result<BlockContext, FromHexError> {
        let hash =
            <[u8; crate::base_crypto::hash::PERSISTENT_HASH_BYTES]>::from_hex(ctxt.block_hash)?;
        Ok(BlockContext {
            seconds_since_epoch: ctxt.seconds_since_epoch,
            seconds_since_epoch_err: ctxt.seconds_since_epoch_err,
            block_hash: HashOutput(hash),
        })
    }
}

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(try_from = "SerdeBlockContext", into = "SerdeBlockContext")
)]
pub struct BlockContext {
    pub seconds_since_epoch: u64,
    pub seconds_since_epoch_err: u32,
    pub block_hash: HashOutput,
}

#[cfg(feature = "serde")]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
struct SerdeEffects {
    claimed_nullifiers: HashSet<String>,
    claimed_receives: HashSet<String>,
    claimed_spends: HashSet<String>,
    claimed_contract_calls: HashSet<(u64, String, String, Fr)>,
    mints: HashMap<String, u64>,
}

#[cfg(feature = "serde")]
impl From<Effects> for SerdeEffects {
    fn from(eff: Effects) -> SerdeEffects {
        SerdeEffects {
            claimed_nullifiers: eff
                .claimed_nullifiers
                .into_iter()
                .map(|n| n.0.0.encode_hex())
                .collect(),
            claimed_receives: eff
                .claimed_receives
                .into_iter()
                .map(|cm| cm.0.0.encode_hex())
                .collect(),
            claimed_spends: eff
                .claimed_spends
                .into_iter()
                .map(|cm| cm.0.0.encode_hex())
                .collect(),
            claimed_contract_calls: eff
                .claimed_contract_calls
                .into_iter()
                .map(|(seq, addr, ep_hash, comm_hash)| {
                    let mut addr_bytes = Vec::new();
                    Serializable::serialize(&addr, &mut addr_bytes)
                        .expect("In-memory serialization must succeed");
                    (
                        seq,
                        addr_bytes.encode_hex(),
                        ep_hash.0.encode_hex(),
                        comm_hash,
                    )
                })
                .collect(),
            mints: eff
                .mints
                .into_iter()
                .map(|(tt, val)| (tt.0.encode_hex(), val))
                .collect(),
        }
    }
}

#[cfg(feature = "serde")]
impl TryFrom<SerdeEffects> for Effects {
    type Error = std::io::Error;

    fn try_from(eff: SerdeEffects) -> Result<Effects, std::io::Error> {
        let err_conv = |err: FromHexError| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string())
        };
        Ok(Effects {
            claimed_nullifiers: eff
                .claimed_nullifiers
                .into_iter()
                .map(|n| Ok::<_, FromHexError>(Nullifier(HashOutput(FromHex::from_hex(n)?))))
                .collect::<Result<_, _>>()
                .map_err(err_conv)?,
            claimed_receives: eff
                .claimed_receives
                .into_iter()
                .map(|cm| Ok::<_, FromHexError>(CoinCommitment(HashOutput(FromHex::from_hex(cm)?))))
                .collect::<Result<_, _>>()
                .map_err(err_conv)?,
            claimed_spends: eff
                .claimed_spends
                .into_iter()
                .map(|cm| Ok::<_, FromHexError>(CoinCommitment(HashOutput(FromHex::from_hex(cm)?))))
                .collect::<Result<_, _>>()
                .map_err(err_conv)?,
            claimed_contract_calls: eff
                .claimed_contract_calls
                .into_iter()
                .map(|(seq, addr, ep_hash, comm_hash)| {
                    let addr_bytes: Vec<u8> = FromHex::from_hex(addr).map_err(err_conv)?;
                    Ok::<_, std::io::Error>((
                        seq,
                        Deserializable::deserialize(&mut &addr_bytes[..], 0)?,
                        HashOutput(FromHex::from_hex(ep_hash).map_err(err_conv)?),
                        comm_hash,
                    ))
                })
                .collect::<Result<_, _>>()?,
            mints: eff
                .mints
                .into_iter()
                .map(|(tt, val)| Ok::<_, FromHexError>((HashOutput(FromHex::from_hex(tt)?), val)))
                .collect::<Result<_, _>>()
                .map_err(err_conv)?,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Versioned, Serializable, Deserializable)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "proptest", derive(Arbitrary))]
#[cfg_attr(
    feature = "serde",
    serde(try_from = "SerdeEffects", into = "SerdeEffects")
)]
pub struct Effects {
    pub claimed_nullifiers: HashSet<Nullifier>,
    pub claimed_receives: HashSet<CoinCommitment>,
    pub claimed_spends: HashSet<CoinCommitment>,
    pub claimed_contract_calls: HashSet<(u64, ContractAddress, HashOutput, Fr)>,
    pub mints: HashMap<HashOutput, u64>,
}

#[cfg(feature = "proptest")]
randomised_serialization_test!(Effects);

impl<'a, D: DB> From<&'a Effects> for VmValue<D> {
    fn from(eff: &'a Effects) -> VmValue<D> {
        VmValue::new(
            ValueStrength::Weak,
            StateValue::Array(
                vec![
                    StateValue::Map(
                        eff.claimed_nullifiers
                            .iter()
                            .map(|k| ((*k).into(), StateValue::Null))
                            .collect(),
                    ),
                    StateValue::Map(
                        eff.claimed_receives
                            .iter()
                            .map(|k| ((*k).into(), StateValue::Null))
                            .collect(),
                    ),
                    StateValue::Map(
                        eff.claimed_spends
                            .iter()
                            .map(|k| ((*k).into(), StateValue::Null))
                            .collect(),
                    ),
                    StateValue::Map(
                        eff.claimed_contract_calls
                            .iter()
                            .map(|k| ((*k).into(), StateValue::Null))
                            .collect(),
                    ),
                    StateValue::Map(
                        eff.mints
                            .iter()
                            .map(|(k, v)| ((*k).into(), StateValue::Cell(Arc::new((*v).into()))))
                            .collect(),
                    ),
                ]
                .into(),
            ),
        )
    }
}

impl<D: DB> TryFrom<VmValue<D>> for Effects {
    type Error = TranscriptRejected<D>;

    fn try_from(val: VmValue<D>) -> Result<Effects, TranscriptRejected<D>> {
        fn map_from<
            K: Eq + Hash + for<'a> TryFrom<&'a ValueSlice, Error = InvalidBuiltinDecode>,
            V: Default + for<'a> TryFrom<&'a ValueSlice, Error = InvalidBuiltinDecode>,
            D: DB,
        >(
            st: &StateValue<D>,
        ) -> Result<HashMap<K, V>, TranscriptRejected<D>> {
            if let StateValue::Map(m) = st {
                Ok(m.iter()
                    .map(|kv| {
                        let v = match *kv.1 {
                            StateValue::Cell(ref v) => {
                                (&**AsRef::<Value>::as_ref(&v)).try_into()?
                            }
                            StateValue::Null => V::default(),
                            _ => return Err(TranscriptRejected::EffectDecodeError),
                        };
                        Ok::<_, TranscriptRejected<D>>((
                            (&**AsRef::<Value>::as_ref(&(*kv.0))).try_into()?,
                            v,
                        ))
                    })
                    .collect::<Result<_, _>>()?)
            } else {
                Err(TranscriptRejected::EffectDecodeError)
            }
        }
        if let StateValue::Array(arr) = &val.value {
            if arr.len() == 5 {
                return Ok(Effects {
                    claimed_nullifiers: map_from(&arr[0])?.into_iter().map(|(k, ())| k).collect(),
                    claimed_receives: map_from(&arr[1])?.into_iter().map(|(k, ())| k).collect(),
                    claimed_spends: map_from(&arr[2])?.into_iter().map(|(k, ())| k).collect(),
                    claimed_contract_calls: map_from(&arr[3])?
                        .into_iter()
                        .map(|(k, ())| k)
                        .collect(),
                    mints: map_from(&arr[4])?,
                });
            }
        }
        Err(TranscriptRejected::EffectDecodeError)
    }
}

#[derive_where(Clone, Debug; Map<CoinCommitment, u64>, ContractState<D>)]
pub struct QueryContext<D: DB> {
    pub state: StateValue<D>,
    pub effects: Effects,
    pub address: ContractAddress,
    pub com_indicies: Map<CoinCommitment, u64>,
    pub block: BlockContext,
}

impl<D: DB> From<&QueryContext<D>> for VmValue<D> {
    fn from(context: &QueryContext<D>) -> VmValue<D> {
        VmValue::new(
            ValueStrength::Weak,
            StateValue::Array(
                vec![
                    StateValue::Cell(Arc::new(context.address.into())),
                    StateValue::Map(
                        context
                            .com_indicies
                            .iter()
                            .map(|(k, v)| {
                                (k.into(), StateValue::Cell(Arc::new((*v.clone()).into())))
                            })
                            .collect(),
                    ),
                    StateValue::Cell(Arc::new(context.block.seconds_since_epoch.into())),
                    StateValue::Cell(Arc::new(context.block.seconds_since_epoch_err.into())),
                    StateValue::Cell(Arc::new(context.block.block_hash.into())),
                ]
                .into(),
            ),
        )
    }
}

#[derive(Debug)]
pub struct QueryResults<M: ResultMode<D>, D: DB> {
    pub context: QueryContext<D>,
    pub events: Vec<M::Event>,
    pub gas_cost: u64,
}

impl<D: DB> QueryContext<D> {
    pub fn new(state: StateValue<D>, address: ContractAddress) -> Self {
        QueryContext {
            state,
            address,
            effects: Effects::default(),
            com_indicies: Map::new(),
            block: BlockContext::default(),
        }
    }

    pub fn qualify(&self, coin: &CoinInfo) -> Option<QualifiedCoinInfo> {
        self.com_indicies
            .get(&coin.commitment(&Recipient::Contract(self.address)))
            .map(|idx| coin.qualify(*idx))
    }

    #[instrument(skip(self, cost_model))]
    pub fn query<M: ResultMode<D>>(
        &self,
        query: &[Op<M, D>],
        gas_limit: Option<u64>,
        cost_model: &CostModel,
    ) -> Result<QueryResults<M, D>, TranscriptRejected<D>> {
        let mut state: Self = self.clone();
        let stack = &[
            self.into(),
            (&self.effects).into(),
            VmValue::new(ValueStrength::Strong, state.state),
        ];
        let mut res = run_program(&stack[..], query, gas_limit, cost_model)?;
        if res.stack.len() != 3 {
            return Err(TranscriptRejected::FinalStackWrongLength);
        }
        state.state = match res.stack.pop().unwrap() {
            VmValue {
                strength: ValueStrength::Strong,
                value,
            } => value,
            VmValue {
                strength: ValueStrength::Weak,
                ..
            } => return Err(TranscriptRejected::WeakStateReturned),
        };
        state.effects = res.stack.pop().unwrap().try_into()?;
        trace!("transcript application successful");
        Ok(QueryResults {
            context: state,
            events: res.events,
            gas_cost: res.gas_cost,
        })
    }

    #[instrument(skip(self, cost_model))]
    pub fn run_transcript(
        &self,
        transcript: &Transcript<D>,
        cost_model: &CostModel,
    ) -> Result<Self, TranscriptRejected<D>> {
        Ok(self
            .query(&transcript.program, Some(transcript.gas), cost_model)?
            .context)
    }
}
