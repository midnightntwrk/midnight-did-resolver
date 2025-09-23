use crate::ZSWAP_TREE_HEIGHT;
use crate::error::TransactionInvalid;
use crate::serialize::{Deserializable, Serializable, Version, Versioned};
use crate::storage::storage::default_storage;
use crate::storage::storage::{HashMap, Map};
use crate::structure::*;
use crate::transient_crypto::merkle_tree::{MerkleTree, MerkleTreeDigest};
use coin_structure::coin::{Commitment, Nullifier};
use coin_structure::contract::Address as ContractAddress;
use coin_structure::storage::db::DB;
use coin_structure::storage::{Storable, arena::ArenaKey, storable::Loader};
use derive_where::derive_where;
#[cfg(feature = "serde")]
use serde::Serialize;
use std::fmt::Debug;

#[derive(Debug, Eq, Storable)]
#[derive_where(Clone, PartialEq)]
#[storable(db = D)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct State<D: DB> {
    pub coin_coms: MerkleTree<Option<ContractAddress>, D>,
    pub coin_coms_set: HashMap<Commitment, (), D>,
    pub first_free: u64,
    pub nullifiers: HashMap<Nullifier, (), D>,
    pub past_roots: HashMap<MerkleTreeDigest, (), D>,
}

impl<D: DB> Serializable for State<D> {
    fn unversioned_serialize<W: std::io::Write>(
        value: &Self,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        Serializable::serialize(&value.coin_coms, writer)?;
        Serializable::serialize(&value.coin_coms_set, writer)?;
        Serializable::serialize(&value.first_free, writer)?;
        Serializable::serialize(&value.nullifiers, writer)?;
        Serializable::serialize(&value.past_roots, writer)?;
        Ok(())
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        Serializable::serialized_size(&value.coin_coms)
            + Serializable::serialized_size(&value.coin_coms_set)
            + Serializable::serialized_size(&value.first_free)
            + Serializable::serialized_size(&value.nullifiers)
            + Serializable::serialized_size(&value.past_roots)
    }
}

impl<D: DB> Versioned for State<D> {
    const VERSION: Option<crate::serialize::Version> = Some(Version { major: 4, minor: 0 });
}

impl<D: DB> Deserializable for State<D> {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursive_depth: u32,
    ) -> Result<Self, std::io::Error> {
        match version {
            Some(Version { major: 4, minor: 0 }) => Ok(Self {
                coin_coms: <MerkleTree<Option<ContractAddress>, D> as Deserializable>::deserialize(
                    reader,
                    recursive_depth,
                )?,
                coin_coms_set: HashMap::<Commitment, (), D>::deserialize(reader, recursive_depth)?,
                first_free: u64::deserialize(reader, recursive_depth)?,
                nullifiers: HashMap::<Nullifier, (), D>::deserialize(reader, recursive_depth)?,
                past_roots: HashMap::<MerkleTreeDigest, (), D>::deserialize(
                    reader,
                    recursive_depth,
                )?,
            }),
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version.".to_string(),
            )),
        }
    }
}

impl<D: DB> Default for State<D> {
    fn default() -> Self {
        State {
            coin_coms: MerkleTree::blank(ZSWAP_TREE_HEIGHT),
            coin_coms_set: HashMap::new(),
            first_free: 0,
            nullifiers: HashMap::new(),
            past_roots: HashMap::new(),
        }
    }
}

impl<D: DB> State<D> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn apply_mint<P>(
        &self,
        mint: &AuthorizedMint<P>,
        collapse_result: bool,
    ) -> Result<(Self, Commitment, u64), TransactionInvalid> {
        let mut state = self.clone();
        let com = mint
            .coin
            .commitment(&coin_structure::transfer::Recipient::User(mint.recipient));
        state.coin_coms = state.coin_coms.update_hash(state.first_free, com.0, None);
        if state.coin_coms_set.contains_key(&com) {
            warn!(?com, "attempted faerie gold");
            return Err(TransactionInvalid::CommitmentAlreadyPresent(com));
        }
        state.coin_coms_set = state.coin_coms_set.insert(com, ());
        let idx = state.first_free;
        if collapse_result {
            state.coin_coms = state.coin_coms.collapse(state.first_free, state.first_free);
        }
        state.first_free += 1;
        Ok((state, com, idx))
    }

    #[instrument(skip(self, offer, whitelist))]
    pub fn try_apply<P>(
        &self,
        offer: &Offer<P>,
        whitelist: Option<Map<ContractAddress, ()>>,
    ) -> Result<(Self, Map<Commitment, u64>), TransactionInvalid> {
        fn on_whitelist(
            whitelist: &Option<Map<ContractAddress, ()>>,
            contract: &Option<ContractAddress>,
        ) -> bool {
            match (whitelist, contract) {
                (Some(list), Some(addr)) => list.contains_key(addr),
                // If we have a contract whitelist, the assumption is that we're
                // tracking a contract, *not* a user state!
                (Some(_), None) => false,
                (None, None) | (None, Some(_)) => true,
            }
        }
        let mut coin_coms = self.coin_coms.clone();
        let mut coin_coms_set = self.coin_coms_set.clone();
        let mut indicies = Map::new();
        let mut first_free = self.first_free;
        let mut nullifiers = self.nullifiers.clone();
        for input in offer.inputs.iter() {
            if nullifiers.contains_key(&input.nullifier) {
                warn!(?input.nullifier, "attempted double spend");
                return Err(TransactionInvalid::NullifierAlreadyPresent(input.nullifier));
            }
            if !self.past_roots.contains_key(&input.merkle_tree_root) {
                warn!(
                    ?input.merkle_tree_root,
                    "attempted spend with unknown Merkle tree"
                );
                return Err(TransactionInvalid::UnknownMerkleRoot(
                    input.merkle_tree_root,
                ));
            }
            if on_whitelist(&whitelist, &input.contract_address) {
                nullifiers = nullifiers.insert(input.nullifier, ());
            }
        }
        for output in offer.outputs.iter() {
            coin_coms =
                coin_coms.update_hash(first_free, output.coin_com.0, output.contract_address);
            if coin_coms_set.contains_key(&output.coin_com) {
                warn!(?output.coin_com, "attempted faerie gold");
                return Err(TransactionInvalid::CommitmentAlreadyPresent(
                    output.coin_com,
                ));
            }
            coin_coms_set = coin_coms_set.insert(output.coin_com, ());
            indicies = indicies.insert(output.coin_com, first_free);
            if !on_whitelist(&whitelist, &output.contract_address) {
                coin_coms = coin_coms.collapse(first_free, first_free);
            }
            first_free += 1;
        }
        for io in offer.transient.iter() {
            coin_coms = coin_coms.update_hash(first_free, io.coin_com.0, io.contract_address);
            if coin_coms_set.contains_key(&io.coin_com) {
                warn!(?io.coin_com, "attempted faerie gold");
                return Err(TransactionInvalid::CommitmentAlreadyPresent(io.coin_com));
            }
            coin_coms_set = coin_coms_set.insert(io.coin_com, ());
            indicies = indicies.insert(io.coin_com, first_free);
            if !on_whitelist(&whitelist, &io.contract_address) {
                coin_coms = coin_coms.collapse(first_free, first_free);
            }
            first_free += 1;
            if nullifiers.contains_key(&io.nullifier) {
                return Err(TransactionInvalid::NullifierAlreadyPresent(io.nullifier));
            } else if on_whitelist(&whitelist, &io.contract_address) {
                nullifiers = nullifiers.insert(io.nullifier, ());
            }
        }
        let root = coin_coms.root();
        Ok((
            State {
                coin_coms,
                coin_coms_set,
                first_free,
                nullifiers,
                past_roots: self.past_roots.insert(root, ()),
            },
            indicies,
        ))
    }

    pub fn filter(&self, filter: &[ContractAddress]) -> MerkleTree<Option<ContractAddress>, D> {
        let retained_indices: Vec<u64> = self
            .coin_coms
            .iter_aux()
            .filter(|(_index, (_hash, opt_aux))| match opt_aux {
                Some(aux) => filter.contains(aux),
                None => false,
            })
            .map(|(index, ..)| index)
            .collect();
        let mut tree = self.coin_coms.clone();
        let mut p = 0;
        for i in retained_indices {
            if i > 0 {
                tree = tree.collapse(p, i - 1);
            }
            if i < u64::MAX {
                p = i + 1;
            }
        }
        if self.first_free > 0 {
            tree.collapse(p, self.first_free - 1)
        } else {
            tree
        }
    }
}

#[cfg(test)]
mod tests {
    use super::State;
    use crate::DB;
    use crate::{Input, Offer, Output};
    use coin_structure::coin::{Info as CoinInfo, NATIVE_TOKEN, TokenType};
    use coin_structure::contract::Address as ContractAddress;
    use coin_structure::storage::db::InMemoryDB;
    use coin_structure::transfer::Recipient;
    use rand::rngs::ThreadRng;
    use rand::{CryptoRng, Rng};

    #[test]
    fn test_filtered_spend() {
        fn insert_dummy_outputs<R: Rng + CryptoRng, D: DB>(
            rng: &mut R,
            mut state: State<D>,
            n: usize,
        ) -> State<D> {
            for _ in 0..n {
                let (type_, value) = rng.gen();
                let info = CoinInfo {
                    nonce: rng.gen(),
                    type_: TokenType(type_),
                    value,
                };
                let cpk = coin_structure::coin::PublicKey(rng.gen());
                let output = Output::new(rng, &info, 0, &cpk, None).unwrap();
                state = state
                    .try_apply(
                        &Offer {
                            inputs: vec![],
                            outputs: vec![output],
                            transient: vec![],
                            deltas: vec![(TokenType(type_), value as i128)],
                        },
                        None,
                    )
                    .unwrap()
                    .0;
            }
            state
        }
        let mut state = State::<InMemoryDB>::new();
        let mut rng = rand::thread_rng();
        let coin = CoinInfo {
            nonce: rng.gen(),
            type_: NATIVE_TOKEN,
            value: 500,
        };
        let addr = ContractAddress::default();
        state = insert_dummy_outputs(&mut rng, state, 25);
        let output = Output::new_contract_owned(&mut rng, &coin, 0, addr).unwrap();
        let (new_state, indicies) = state
            .try_apply(
                &Offer {
                    inputs: vec![],
                    outputs: vec![output],
                    transient: vec![],
                    deltas: vec![(NATIVE_TOKEN, 500)],
                },
                None,
            )
            .unwrap();
        state = new_state;
        state = insert_dummy_outputs(&mut rng, state, 25);
        let qcoin = coin.qualify(
            *indicies
                .get(&coin.commitment(&Recipient::Contract(addr)))
                .unwrap(),
        );
        Input::new_contract_owned(&mut rng, &qcoin, 0, addr, &state.filter(&[addr])).unwrap();
    }
}
