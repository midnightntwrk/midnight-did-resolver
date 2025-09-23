use core::fmt::Debug;
use core::fmt::Formatter;

use crate::serialize::{Deserializable, Serializable, Version, Versioned};
use crate::storage::storage::default_storage;
use crate::storage::storage::{HashMap, Map};
use crate::transient_crypto::merkle_tree::{self, MerkleTree, MerkleTreeCollapsedUpdate};
use base_crypto::hash::{PERSISTENT_HASH_BYTES, PersistentHashWriter};
use base_crypto::repr::MemWrite;
use coin_structure::coin::{
    self, Commitment, Info as CoinInfo, Nullifier, QualifiedInfo as QualifiedCoinInfo,
};
use coin_structure::storage::db::DB;
use coin_structure::transfer::{Recipient, SenderEvidence};
use derive_where::derive_where;
use rand::{CryptoRng, Rng};
use transient_crypto::encryption;
#[cfg(feature = "offer-construction")]
use transient_crypto::proofs::ProofPreimage;
use transient_crypto::repr::FieldRepr;

use crate::ZSWAP_TREE_HEIGHT;
#[cfg(feature = "offer-construction")]
use crate::error::OfferCreationFailed;
use crate::keys::{SecretKeys, Seed};
use crate::structure::*;

#[derive(Debug)]
#[derive_where(Clone)]
pub struct State<D: DB> {
    pub coins: Map<Nullifier, QualifiedCoinInfo, D>,
    pub pending_spends: Map<Nullifier, QualifiedCoinInfo, D>,
    pub pending_outputs: Map<Commitment, CoinInfo, D>,
    merkle_tree: MerkleTree<(), D>,
    pub first_free: u64,
}

impl<D: DB> Serializable for State<D> {
    fn unversioned_serialize<W: std::io::Write>(
        value: &Self,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        Serializable::serialize(&value.coins, writer)?;
        Serializable::serialize(&value.pending_spends, writer)?;
        Serializable::serialize(&value.pending_outputs, writer)?;
        Serializable::serialize(&value.merkle_tree, writer)?;
        Serializable::serialize(&value.first_free, writer)?;
        Ok(())
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        Serializable::serialized_size(&value.coins)
            + Serializable::serialized_size(&value.pending_spends)
            + Serializable::serialized_size(&value.pending_outputs)
            + Serializable::serialized_size(&value.merkle_tree)
            + Serializable::serialized_size(&value.first_free)
    }
}

impl<D: DB> Versioned for State<D> {
    const VERSION: Option<Version> = Some(Version { major: 5, minor: 0 });
}

impl<D: DB> Deserializable for State<D> {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursive_depth: u32,
    ) -> Result<Self, std::io::Error> {
        match version {
            Some(Version { major: 5, minor: 0 }) => Ok(Self {
                coins: Map::<Nullifier, QualifiedCoinInfo, D>::deserialize(
                    reader,
                    recursive_depth,
                )?,
                pending_spends: Map::<Nullifier, QualifiedCoinInfo, D>::deserialize(
                    reader,
                    recursive_depth,
                )?,
                pending_outputs: Map::<Commitment, CoinInfo, D>::deserialize(
                    reader,
                    recursive_depth,
                )?,
                merkle_tree: MerkleTree::<(), D>::deserialize(reader, recursive_depth)?,
                first_free: u64::deserialize(reader, recursive_depth)?,
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
        Self::new()
    }
}

impl<D: DB> State<D> {
    pub fn new() -> Self {
        State {
            coins: Map::new(),
            pending_spends: Map::new(),
            pending_outputs: Map::new(),
            merkle_tree: MerkleTree::blank(ZSWAP_TREE_HEIGHT),
            first_free: 0,
        }
    }

    pub fn apply_collapsed_update(
        &self,
        update: &MerkleTreeCollapsedUpdate,
    ) -> Result<Self, merkle_tree::InvalidUpdate> {
        Ok(Self {
            merkle_tree: self.merkle_tree.apply_collapsed_update(update)?,
            first_free: u64::max(self.first_free, update.end + 1),
            ..self.clone()
        })
    }

    pub fn apply_mint<P>(&self, secret_keys: &SecretKeys, tx: &AuthorizedMint<P>) -> Self {
        let mut res = self.clone();
        res.merkle_tree = res.merkle_tree.update_hash(
            res.first_free,
            tx.coin.commitment(&Recipient::User(tx.recipient)).0,
            (),
        );
        if secret_keys.coin_public_key() == tx.recipient {
            res.coins = self.coins.insert(
                tx.coin
                    .nullifier(&SenderEvidence::User(secret_keys.coin_secret_key)),
                tx.coin.qualify(self.first_free),
            );
        } else {
            res.merkle_tree.collapse(res.first_free, res.first_free);
        }
        res.first_free += 1;
        res
    }

    #[instrument(skip(self, tx))]
    pub fn apply_failed<P>(&self, tx: &Offer<P>) -> State<D> {
        let mut res = self.clone();
        for nullifier in tx
            .inputs
            .iter()
            .map(|o| &o.nullifier)
            .chain(tx.transient.iter().map(|io| &io.nullifier))
        {
            res.pending_spends = res.pending_spends.remove(nullifier);
        }
        for coin_com in tx
            .outputs
            .iter()
            .map(|o| &o.coin_com)
            .chain(tx.transient.iter().map(|io| &io.coin_com))
        {
            res.pending_outputs = res.pending_outputs.remove(coin_com);
        }
        res
    }

    #[instrument(skip(self, tx))]
    pub fn apply<P>(&self, secret_keys: &SecretKeys, tx: &Offer<P>) -> State<D> {
        let mut res = self.clone();
        for (coin_com, ciph) in tx
            .outputs
            .iter()
            .map(|o| (&o.coin_com, &o.ciphertext))
            .chain(tx.transient.iter().map(|io| (&io.coin_com, &io.ciphertext)))
        {
            res.merkle_tree = res.merkle_tree.update_hash(res.first_free, coin_com.0, ());
            if let Some(ci) = ciph.as_ref().and_then(|ciph| secret_keys.try_decrypt(ciph)) {
                info!(coin=?ci, "received coin");
                let qci = ci.qualify(res.first_free);
                // Verify that what we got is actually valid.
                if &ci.commitment(&Recipient::User(secret_keys.coin_public_key())) == coin_com {
                    res.coins = res.coins.insert(
                        CoinInfo::nullifier(
                            &(&qci).into(),
                            &SenderEvidence::User(secret_keys.coin_secret_key),
                        ),
                        qci,
                    );
                    res.pending_outputs = res.pending_outputs.remove(coin_com);
                }
            } else if let Some(coin) = res.pending_outputs.get(coin_com) {
                info!(?coin, "received coin");
                let qci = coin.qualify(res.first_free);
                res.coins = res.coins.insert(
                    CoinInfo::nullifier(
                        &(&qci).into(),
                        &SenderEvidence::User(secret_keys.coin_secret_key),
                    ),
                    qci,
                );
                res.pending_outputs = res.pending_outputs.remove(coin_com);
            } else {
                res.merkle_tree = res.merkle_tree.collapse(res.first_free, res.first_free);
            }
            res.first_free += 1;
        }
        for nul in tx
            .inputs
            .iter()
            .map(|i| &i.nullifier)
            .chain(tx.transient.iter().map(|io| &io.nullifier))
        {
            if let Some(coin) = res.coins.get(nul) {
                info!(?coin, "spent coin finalized");
                res.coins = res.coins.remove(nul);
            }
            if let Some(coin) = res.pending_spends.get(nul) {
                info!(?coin, "pending spend removed");
                res.pending_spends = res.pending_spends.remove(nul);
            }
        }
        res
    }

    #[cfg(feature = "offer-construction")]
    #[instrument(skip(self, rng))]
    pub fn spend<R: Rng + CryptoRng + ?Sized>(
        &self,
        rng: &mut R,
        secret_keys: &SecretKeys,
        coin: &QualifiedCoinInfo,
        segment: u16,
    ) -> Result<(State<D>, Input<ProofPreimage>), OfferCreationFailed> {
        self.spend_from_tree(rng, secret_keys, coin, segment, &self.merkle_tree.clone())
    }

    #[cfg(feature = "offer-construction")]
    #[instrument(skip(self, rng, tree))]
    fn spend_from_tree<R: Rng + CryptoRng + ?Sized>(
        &self,
        rng: &mut R,
        secret_keys: &SecretKeys,
        coin: &QualifiedCoinInfo,
        segment: u16,
        tree: &MerkleTree<(), D>,
    ) -> Result<(State<D>, Input<ProofPreimage>), OfferCreationFailed> {
        let inp = Input::new_from_secret_key(
            rng,
            coin,
            segment,
            SenderEvidence::User(secret_keys.coin_secret_key),
            tree,
        )?;
        let res = State {
            pending_spends: self.pending_spends.insert(inp.nullifier, *coin),
            ..self.clone()
        };
        Ok((res, inp))
    }

    #[cfg(feature = "offer-construction")]
    #[instrument(skip(self, rng))]
    pub fn spend_from_output<R: Rng + CryptoRng + ?Sized>(
        &self,
        rng: &mut R,
        secret_keys: &SecretKeys,
        coin: &QualifiedCoinInfo,
        segment: u16,
        output: Output<ProofPreimage>,
    ) -> Result<(State<D>, Transient<ProofPreimage>), OfferCreationFailed> {
        let tree = MerkleTree::blank(ZSWAP_TREE_HEIGHT).update_hash(0, output.coin_com.0, ());
        let (res, input) = self.spend_from_tree(rng, secret_keys, coin, segment, &tree)?;
        let io = Transient {
            nullifier: input.nullifier,
            coin_com: output.coin_com,
            value_commitment_input: input.value_commitment,
            value_commitment_output: output.value_commitment,
            contract_address: output.contract_address,
            ciphertext: output.ciphertext,
            proof_input: input.proof,
            proof_output: output.proof,
        };
        Ok((res, io))
    }

    #[cfg(feature = "offer-construction")]
    #[instrument(skip(rng))]
    pub fn authorize_mint<R: Rng + CryptoRng + ?Sized>(
        &self,
        rng: &mut R,
        secret_keys: &SecretKeys,
        coin: CoinInfo,
    ) -> Result<AuthorizedMint<ProofPreimage>, OfferCreationFailed> {
        AuthorizedMint::new::<R, D>(rng, coin, &secret_keys.coin_secret_key)
    }

    pub fn watch_for(&self, coin_public_key: &coin::PublicKey, coin: &CoinInfo) -> State<D> {
        debug!(?coin, "watching for coin");
        State {
            pending_outputs: self
                .pending_outputs
                .insert(coin.commitment(&Recipient::User(*coin_public_key)), *coin),
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use coin_structure::coin::TokenType;
    use hex::FromHex;
    use rand::rngs::OsRng;
    use serde::{Deserialize, Deserializer};

    use super::*;

    #[test]
    fn coin_encryption_succeeds() {
        // Just tries to encrypt a dummy coin. This mainly tests that
        // COIN_CIPHERTEXT_LEN is accurate.
        let keys: SecretKeys = Seed::random(&mut OsRng).into();
        let coin = CoinInfo {
            nonce: OsRng.gen(),
            type_: TokenType(OsRng.gen()),
            value: OsRng.gen(),
        };
        CoinCiphertext::new(&mut OsRng, &coin, keys.enc_public_key());
    }
}
