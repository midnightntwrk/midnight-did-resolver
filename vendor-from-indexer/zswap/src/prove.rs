use crate::serialize::Deserializable;
use crate::structure::*;
use crate::transient_crypto::proofs::ParamsProver;
use base_crypto::data_provider::MidnightDataProvider;
use base_crypto::rng::SplittableRng;
use futures::future::join_all;
use rand::{CryptoRng, Rng};
use std::future::Future;
use transient_crypto::proofs::{
    IrSource, KeyLocation, ParamsProverProvider, Proof, ProofPreimage, ProverKey, ProvingError,
    Resolver, VerifierKey,
};

#[derive(Clone)]
pub struct ZswapResolver(pub MidnightDataProvider);

impl Resolver for ZswapResolver {
    async fn resolve_key(
        &self,
        key: KeyLocation,
    ) -> std::io::Result<Option<(ProverKey, VerifierKey, IrSource)>> {
        let file_root = match &*key.0 {
            "midnight/zswap/spend" => {
                concat!("zswap/", include_str!("../../static/version"), "/spend")
            }
            "midnight/zswap/output" => {
                concat!("zswap/", include_str!("../../static/version"), "/output")
            }
            "midnight/zswap/sign" => {
                concat!("zswap/", include_str!("../../static/version"), "/sign")
            }
            _ => return Ok(None),
        };
        let prover = ProverKey::deserialize(
            &mut self
                .0
                .get_file(
                    &format!("{file_root}.prover"),
                    &format!("failed to find built-in zswap prover key {file_root}.prover"),
                )
                .await?,
            0,
        )?;
        let verifier = VerifierKey::deserialize(
            &mut self
                .0
                .get_file(
                    &format!("{file_root}.verifier"),
                    &format!("failed to find built-in zswap verifier key {file_root}.verifier"),
                )
                .await?,
            0,
        )?;
        let ir_source = IrSource::deserialize(
            &mut self
                .0
                .get_file(
                    &format!("{file_root}.bzkir"),
                    &format!("failed to find built-in zswap IR {file_root}.bzkir"),
                )
                .await?,
            0,
        )?;
        Ok(Some((prover, verifier, ir_source)))
    }
}

impl ParamsProverProvider for ZswapResolver {
    async fn get_params(&self, k: u8) -> std::io::Result<ParamsProver> {
        self.0.get_params(k).await
    }
}

impl AuthorizedMint<ProofPreimage> {
    pub async fn prove(
        &self,
        rng: impl Rng + CryptoRng,
        pp: &impl ParamsProverProvider,
        resolver: &impl Resolver,
    ) -> Result<AuthorizedMint<Proof>, ProvingError> {
        Ok(AuthorizedMint {
            coin: self.coin,
            recipient: self.recipient,
            proof: self.proof.prove(rng, pp, resolver).await?.0,
        })
    }
}

impl Offer<ProofPreimage> {
    pub async fn prove(
        &self,
        mut rng: impl CryptoRng + SplittableRng,
        pp: &impl ParamsProverProvider,
        resolver: &impl Resolver,
    ) -> Result<Offer<Proof>, ProvingError> {
        let (inputs, outputs, transient) = futures::join!(
            join_all(
                self.inputs
                    .iter()
                    .map(|i| i.prove(rng.split(), pp, resolver))
            ),
            join_all(
                self.outputs
                    .iter()
                    .map(|o| o.prove(rng.split(), pp, resolver))
            ),
            join_all(
                self.transient
                    .iter()
                    .map(|io| io.prove(rng.split(), pp, resolver))
            )
        );
        let mut offer = Offer {
            inputs: inputs.into_iter().collect::<Result<_, _>>()?,
            outputs: outputs.into_iter().collect::<Result<_, _>>()?,
            transient: transient.into_iter().collect::<Result<_, _>>()?,
            deltas: self.deltas.clone(),
        };
        offer.normalize();
        Ok(offer)
    }
}

impl Input<ProofPreimage> {
    pub async fn prove(
        &self,
        rng: impl CryptoRng + SplittableRng,
        pp: &impl ParamsProverProvider,
        resolver: &impl Resolver,
    ) -> Result<Input<Proof>, ProvingError> {
        Ok(Input {
            nullifier: self.nullifier,
            value_commitment: self.value_commitment,
            contract_address: self.contract_address,
            merkle_tree_root: self.merkle_tree_root,
            proof: self.proof.prove(rng, pp, resolver).await?.0,
        })
    }
}

impl Output<ProofPreimage> {
    pub async fn prove(
        &self,
        rng: impl CryptoRng + SplittableRng,
        pp: &impl ParamsProverProvider,
        resolver: &impl Resolver,
    ) -> Result<Output<Proof>, ProvingError> {
        let proof = self.proof.prove(rng, pp, resolver).await?.0;
        Ok(Output {
            coin_com: self.coin_com,
            value_commitment: self.value_commitment,
            contract_address: self.contract_address,
            ciphertext: self.ciphertext.clone(),
            proof,
        })
    }
}

impl Transient<ProofPreimage> {
    pub async fn prove(
        &self,
        mut rng: impl CryptoRng + SplittableRng,
        pp: &impl ParamsProverProvider,
        resolver: &impl Resolver,
    ) -> Result<Transient<Proof>, ProvingError> {
        let (proof_input, proof_output) = futures::join!(
            self.proof_input.prove(rng.split(), pp, resolver),
            self.proof_output.prove(rng.split(), pp, resolver),
        );
        Ok(Transient {
            nullifier: self.nullifier,
            coin_com: self.coin_com,
            value_commitment_input: self.value_commitment_input,
            value_commitment_output: self.value_commitment_output,
            contract_address: self.contract_address,
            ciphertext: self.ciphertext.clone(),
            proof_input: proof_input?.0,
            proof_output: proof_output?.0,
        })
    }
}

#[cfg(test)]
mod tests {
    use transient_crypto::proofs::ir::Instruction;

    use super::*;

    #[test]
    fn test_pi_lengths() {
        fn count_pis(ir: &str) -> usize {
            use crate::serialize::Deserializable;
            use std::fs::File;
            use std::path::PathBuf;
            let file = PathBuf::from("../static/zswap")
                .join(ir)
                .with_extension("bzkir");
            let ir = IrSource::deserialize(&mut File::open(file).unwrap(), 0).unwrap();
            ir.instructions
                .iter()
                .filter_map(|ins| match ins {
                    Instruction::PiSkip { count, .. } => Some(*count as usize),
                    _ => None,
                })
                .sum::<usize>()
                + 1
        }
        assert_eq!(AUTHORIZED_MINT_PIS, count_pis("sign"));
        assert_eq!(OUTPUT_PIS, count_pis("output"));
        assert_eq!(INPUT_PIS, count_pis("spend"));
    }
}
