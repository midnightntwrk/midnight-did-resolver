//! This module provides access to creating, and verifying zero-knowledge
//! proofs. It assumes that keys and IR are generated externally, which is the
//! focus of [Compact](https://github.com/input-output-hk/compactc).

pub mod ir;
mod ir_vm;
use base_crypto::hash::{HashOutput, persistent_hash};
pub use ir_vm::Preprocessed;
use lazy_static::lazy_static;
use lru::LruCache;

use crate::curve::Fr;
use blstrs::Bls12;
use borsh::{BorshDeserialize, BorshSerialize};
use halo2_proofs::{
    poly::kzg::params::{ParamsKZG, ParamsVerifierKZG},
    utils::SerdeFormat,
};
pub use ir::IrSource;
use midnight_circuits::compact_std_lib::{MidnightPK, MidnightVK};
#[cfg(feature = "proptest")]
use proptest::arbitrary::Arbitrary;
#[cfg(feature = "proptest")]
use proptest_derive::Arbitrary;
use rand::distributions::{Distribution, Standard};
use rand::{CryptoRng, Rng};
#[cfg(feature = "serde")]
use serde::{Serialize, Serializer, ser::Error as SerError};
use serialize::{Deserializable, ReadExt, Serializable, VecExt, Version, Versioned};
#[cfg(feature = "proptest")]
use serialize::{NoStrategy, simple_arbitrary};
use std::cmp::Ordering;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::io::{self, Read};
#[cfg(feature = "proptest")]
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::{borrow::Cow, num::NonZeroUsize};

/// A provider of prover parameters.
pub trait ParamsProverProvider {
    // Allowed because we don't care about auto traits here.
    #[allow(async_fn_in_trait)]
    /// Retrieve the parameters for a given `k` value
    async fn get_params(&self, k: u8) -> io::Result<ParamsProver>;
}

#[cfg(feature = "data-provider")]
impl ParamsProverProvider for base_crypto::data_provider::MidnightDataProvider {
    async fn get_params(&self, k: u8) -> io::Result<ParamsProver> {
        let name = Self::name_k(k);
        let reader = self
            .get_file(
                &name,
                &format!("public parameters for k={k} not found in cache"),
            )
            .await?;
        ParamsProver::read(reader)
    }
}

/// A specific instance of the prover parameters.
#[derive(Clone)]
pub struct ParamsProver(Arc<ParamsKZG<Bls12>>);

impl ParamsProver {
    /// Reads the prover parameters from a data stream
    pub fn read<R: Read>(mut reader: R) -> io::Result<Self> {
        Ok(ParamsProver(Arc::new(ParamsKZG::read_custom(
            &mut reader,
            SerdeFormat::RawBytesUnchecked,
        )?)))
    }

    pub(crate) fn as_verifier(&self) -> ParamsVerifier {
        ParamsVerifier(Arc::new(self.0.verifier_params()))
    }
}

/**
 * The maximum degree supported by the standard verifier key.
 * This limits the number of public inputs usable.
 */
pub const VERIFIER_MAX_DEGREE: u8 = 14;

/// Parameters used for verifying with the `KZG` commitment scheme
#[derive(Clone)]
pub struct ParamsVerifier(Arc<ParamsVerifierKZG<Bls12>>);

impl ParamsVerifier {
    /// Reads in verifier parameters
    pub fn read<R: Read>(reader: R) -> io::Result<Self> {
        Ok(ParamsProver::read(reader)?.as_verifier())
    }
}

const PARAMS_VERIFIER_RAW: &[u8] = include_bytes!("../../../static/bls_filecoin_2p14");

lazy_static! {
    /// The filecoin verifier parameters, up to [`VERIFIER_MAX_DEGREE`].
    ///
    /// Note that using this *will* embed these into the binary at compile time, if that's not what
    /// you want, please use `ParamsVerifier::read` instead.
    pub static ref PARAMS_VERIFIER: ParamsVerifier = ParamsVerifier::read(PARAMS_VERIFIER_RAW).expect("Static verifier parameters should be valid.");
}

/// A zero-knowledge proof.
#[cfg_attr(feature = "proptest", derive(Arbitrary))]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Proof(Vec<u8>);

impl Versioned for Proof {
    const VERSION: Option<Version> = Some(Version { major: 4, minor: 0 });
    const NETWORK_SPECIFIC: bool = false;
}

impl Serializable for Proof {
    fn unversioned_serialize<W: Write>(value: &Self, writer: &mut W) -> Result<(), std::io::Error> {
        Serializable::unversioned_serialize(&value.0, writer)
    }
    fn unversioned_serialized_size(value: &Self) -> usize {
        Serializable::unversioned_serialized_size(&value.0)
    }
}

impl Deserializable for Proof {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        match version {
            Some(Version { major: 4, minor: 0 }) => {
                Ok(Proof(Deserializable::deserialize(reader, recursion_depth)?))
            }
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version.".into(),
            )),
        }
    }
}

/// A prover key, used for creating proofs.
#[derive(Debug, Clone)]
pub struct ProverKey(Arc<Mutex<InnerProverKey>>);

impl PartialEq for ProverKey {
    fn eq(&self, other: &Self) -> bool {
        let mut self_ser = Vec::new();
        let mut other_ser = Vec::new();
        Serializable::serialize(self, &mut self_ser).expect("In-memory serialization must succeed");
        Serializable::serialize(other, &mut other_ser)
            .expect("In-memory serialization must succeed");
        self_ser == other_ser
    }
}

impl Eq for ProverKey {}

#[cfg(feature = "proptest")]
simple_arbitrary!(ProverKey);

impl Distribution<ProverKey> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> ProverKey {
        let size: u8 = rng.gen_range(0..32);
        let mut bytes = Vec::with_bounded_capacity(size as usize);
        rng.fill_bytes(&mut bytes);
        ProverKey(Arc::new(Mutex::new(InnerProverKey::Uninitialized(bytes))))
    }
}

#[derive(Debug, Clone)]
pub(crate) enum InnerProverKey {
    Uninitialized(Vec<u8>),
    Invalid(Vec<u8>),
    Initialized(Arc<MidnightPK<IrSource>>),
}

impl Versioned for ProverKey {
    const VERSION: Option<Version> = Some(Version { major: 4, minor: 0 });
    const NETWORK_SPECIFIC: bool = false;
}

const PK_COMPRESSION_LEVEL: u32 = 6;
const PK_CACHE_SIZE: usize = 5;

lazy_static! {
    static ref PK_CACHE: Mutex<LruCache<HashOutput, Arc<MidnightPK<IrSource>>>> =
        Mutex::new(LruCache::new(NonZeroUsize::new(PK_CACHE_SIZE).unwrap()));
}

impl InnerProverKey {
    fn try_cache(&mut self) {
        let hash = match self {
            InnerProverKey::Uninitialized(data) => persistent_hash(&data[..]),
            _ => return,
        };
        if let Some(pk) = PK_CACHE.lock().ok().and_then(|mut c| c.get(&hash).cloned()) {
            *self = InnerProverKey::Initialized(pk);
        }
    }
}

impl ProverKey {
    /// Initializes the lazy prover key
    pub fn init(&self) -> Result<(), ProvingError> {
        self.force_init()?;
        Ok(())
    }

    pub(crate) fn force_init(&self) -> Result<Arc<MidnightPK<IrSource>>, ProvingError> {
        let mut mutex = self.0.lock().expect("mutex is not poisoned");
        mutex.try_cache();
        let data = match &*mutex {
            InnerProverKey::Initialized(key) => {
                return Ok(key.clone());
            }
            InnerProverKey::Invalid(_) => {
                return Err(anyhow::anyhow!("known invalid verifier key"));
            }
            InnerProverKey::Uninitialized(data) => data.clone(),
        };
        let inner_reader = &mut &data[..];
        let mut reader = flate2::read::GzDecoder::new(inner_reader);
        let read_inner = |reader| {
            let pk = MidnightPK::<IrSource>::read(reader, SerdeFormat::RawBytesUnchecked)?;
            Ok(pk)
        };
        let res: Result<_, ProvingError> = read_inner(&mut reader);
        match res {
            Ok(pk) => {
                let key = Arc::new(pk);
                PK_CACHE
                    .lock()
                    .ok()
                    .and_then(|mut c| c.put(persistent_hash(&data), key.clone()));
                *mutex = InnerProverKey::Initialized(key.clone());
                Ok(key)
            }
            Err(e) => {
                *mutex = InnerProverKey::Invalid(data);
                Err(e)
            }
        }
    }

    fn inner_serialize<W: std::io::Write>(&self, mut writer: W) -> std::io::Result<()> {
        match &*self.0.lock().expect("mutex is not poisoned") {
            InnerProverKey::Uninitialized(data) | InnerProverKey::Invalid(data) => {
                writer.write_all(data)?;
                Ok(())
            }
            InnerProverKey::Initialized(key) => {
                let mut writer = flate2::write::GzEncoder::new(
                    writer,
                    flate2::Compression::new(PK_COMPRESSION_LEVEL),
                );
                key.write(&mut writer, SerdeFormat::RawBytesUnchecked)
            }
        }
    }
}

struct Count(usize);

impl std::io::Write for Count {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0 += buf.len();
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Serializable for ProverKey {
    fn unversioned_serialize<W: std::io::Write>(
        value: &Self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let size = Serializable::unversioned_serialized_size(value) - 4;
        Serializable::serialize(&(size as u32), writer)?;
        value.inner_serialize(writer)
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        let mut writer = Count(0);
        value.inner_serialize(&mut writer).ok();
        4 + writer.0
    }
}

impl Deserializable for ProverKey {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        match version {
            Some(Version { major: 4, minor: 0 }) => {
                let size = <u32 as Deserializable>::deserialize(reader, recursion_depth)?;
                let buf = reader.read_exact_to_vec(size as usize)?;
                let mut pk = InnerProverKey::Uninitialized(buf);
                pk.try_cache();
                Ok(Self(Arc::new(Mutex::new(pk))))
            }
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version.".to_string(),
            )),
        }
    }
}

/// A verifier key, used for checking proofs.
#[derive(Debug)]
pub struct VerifierKey(Arc<Mutex<InnerVerifierKey>>);

#[cfg(feature = "proptest")]
simple_arbitrary!(VerifierKey);

impl Distribution<VerifierKey> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> VerifierKey {
        let size: u8 = rng.gen();
        let mut bytes = Vec::with_bounded_capacity(size as usize);
        rng.fill_bytes(&mut bytes);
        VerifierKey(Arc::new(Mutex::new(InnerVerifierKey::Uninitialized(bytes))))
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Some features don't try to initialize
#[allow(clippy::large_enum_variant)]
pub(crate) enum InnerVerifierKey {
    Uninitialized(Vec<u8>),
    Invalid(Vec<u8>),
    Initialized(MidnightVK),
}

impl Clone for VerifierKey {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl Versioned for VerifierKey {
    const VERSION: Option<Version> = Some(Version { major: 4, minor: 0 });
    const NETWORK_SPECIFIC: bool = false;
}

impl Deserializable for VerifierKey {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        match version {
            Some(Version { major: 4, minor: 0 }) => {
                const MAX_EXPECTED_SIZE: u32 = 50_000;
                let size: u32 = Deserializable::deserialize(reader, recursion_depth)?;
                if size > MAX_EXPECTED_SIZE {
                    return Err(Self::deserialization_error(
                        version,
                        format!(
                            "Declared vk size {size} exceeded permitted limit of {MAX_EXPECTED_SIZE}"
                        ),
                    ));
                }
                let mut data = vec![0u8; size as usize];
                reader.read_exact(&mut data)?;
                Ok(Self(Arc::new(Mutex::new(InnerVerifierKey::Uninitialized(
                    data,
                )))))
            }
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version.".to_string(),
            )),
        }
    }
}

#[cfg(feature = "serde")]
impl Serialize for VerifierKey {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let mut vec = Vec::new();
        <VerifierKey as Serializable>::serialize(self, &mut vec).map_err(S::Error::custom)?;
        ser.serialize_bytes(&vec)
    }
}

#[allow(clippy::derived_hash_with_manual_eq)]
impl Hash for VerifierKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut data = Vec::new();
        Serializable::serialize(&self, &mut data).ok();
        state.write(&data);
    }
}

impl Serializable for VerifierKey {
    fn unversioned_serialize<W: Write>(value: &Self, writer: &mut W) -> Result<(), std::io::Error> {
        let size = Serializable::unversioned_serialized_size(value) - 4;
        Serializable::serialize(&(size as u32), writer)?;
        value.inner_serialize(writer)
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        let mut writer = Count(0);
        value.inner_serialize(&mut writer).ok();
        4 + writer.0
    }
}

impl VerifierKey {
    /// Initializes the lazy verifier key
    pub fn init(&self) -> Result<(), VerifyingError> {
        self.force_init()?;
        Ok(())
    }

    // warning! This grabs the lock! Make sure to drop the result before re-running!
    #[allow(dead_code)] // Some features don't try to initialize
    pub(crate) fn force_init(&self) -> Result<MidnightVK, VerifyingError> {
        let mut mutex = self.0.lock().expect("mutex is not poisoned");
        let data = match &*mutex {
            InnerVerifierKey::Initialized(key) => {
                return Ok(key.clone());
            }
            InnerVerifierKey::Invalid(_) => {
                return Err(anyhow::anyhow!("known invalid verifier key"));
            }
            InnerVerifierKey::Uninitialized(data) => data.clone(),
        };
        let reader = &mut &data[..];
        let vk = MidnightVK::read::<&[u8], IrSource>(reader, SerdeFormat::Processed)
            .map_err(|_| anyhow::anyhow!("problem reading the verifier key"))?;
        *mutex = InnerVerifierKey::Initialized(vk.clone());
        Ok(vk)
    }

    fn inner_serialize<W: std::io::Write>(&self, mut writer: W) -> std::io::Result<()> {
        match &*self.0.lock().expect("mutex is not poisoned") {
            InnerVerifierKey::Uninitialized(data) | InnerVerifierKey::Invalid(data) => {
                writer.write_all(data)
            }
            InnerVerifierKey::Initialized(key) => key.write(&mut writer, SerdeFormat::Processed),
        }
    }

    /// Checks a proof against a statement.
    #[cfg(feature = "verifying")]
    pub fn verify<F: Iterator<Item = Fr>>(
        &self,
        params: &ParamsVerifier,
        proof: &Proof,
        statement: F,
    ) -> Result<(), VerifyingError> {
        use midnight_circuits::compact_std_lib;

        let vk = self.force_init()?;
        let pi = statement.map(|f| f.0).collect::<Vec<_>>();
        trace!(statement = ?pi, "verifying proof against statement");
        compact_std_lib::verify::<IrSource>(&params.0, &vk, &pi, &proof.0)
            .map_err(|_| anyhow::anyhow!("Invalid proof"))
    }

    /// Checks a sequence of proofs against their corresponding statements and verifier keys
    #[cfg(feature = "verifying")]
    pub fn batch_verify<
        'a,
        F: Iterator<Item = Fr>,
        V: Iterator<Item = (&'a VerifierKey, &'a Proof, F)>,
    >(
        params: &ParamsVerifier,
        parts: V,
    ) -> Result<(), VerifyingError> {
        use midnight_circuits::compact_std_lib::batch_verify;

        let mut params_verifier = vec![];
        let mut vks = vec![];
        let mut pis = vec![];
        let mut proofs = vec![];

        for (vk, proof, stmt) in parts.into_iter() {
            let pi = stmt.map(|f| f.0).collect::<Vec<_>>();
            let vk = vk.force_init()?;
            params_verifier.push((*params.0).clone());
            vks.push(vk);
            pis.push(pi);
            proofs.push(proof.0.clone());
        }

        batch_verify(&params_verifier, &vks, &pis, &proofs)
            .map_err(|_| anyhow::anyhow!("Invalid proof"))
    }
}

/// A hint on where keys for a circuit can be found.
///
/// Circuit keys are associated with a string name, and are resolved at proving
/// time against a hashtable of provided keys.
#[derive(
    Clone, Debug, BorshSerialize, BorshDeserialize, PartialEq, Eq, PartialOrd, Ord, Versioned, Hash,
)]
#[cfg_attr(feature = "proptest", derive(Arbitrary))]
pub struct KeyLocation(pub Cow<'static, str>);

impl Serializable for KeyLocation {
    fn unversioned_serialize<W: Write>(value: &Self, writer: &mut W) -> Result<(), std::io::Error> {
        value.serialize(writer)
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        let mut bytes = Vec::new();
        value.serialize(&mut bytes).ok();
        bytes.len()
    }
}

impl Deserializable for KeyLocation {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        _version: Option<&Version>,
        _recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        KeyLocation::deserialize_reader(reader)
    }
}

/// A mechanism to retrieve / resolve zero-knowledge key material from a short location string.
pub trait Resolver {
    /// Resolves the given key to the key material it represents, if available.
    // Allowed as we do not need auto traits here
    #[allow(async_fn_in_trait)]
    async fn resolve_key(
        &self,
        key: KeyLocation,
    ) -> io::Result<Option<(ProverKey, VerifierKey, IrSource)>>;
}

/// Everything necessary to produce a proof.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Versioned, Serializable, Deserializable)]
#[cfg_attr(feature = "proptest", derive(Arbitrary))]
pub struct ProofPreimage {
    /// The inputs to be directly handed to the IR.
    pub inputs: Vec<Fr>,
    /// A private witness vector consumed by active witness calls in the IR.
    pub private_transcript: Vec<Fr>,
    /// A public statement vector encoding statement call information in the IR.
    pub public_transcript_inputs: Vec<Fr>,
    /// A public statement vector encoding statement call results in the IR.
    pub public_transcript_outputs: Vec<Fr>,
    /// An arbitrary input to be bound to in the proof.
    pub binding_input: Fr,
    /// The communications commitment that will be checked, and its randomness.
    /// May be [None], in which case inputs and outputs are not committed to.
    pub communications_commitment: Option<(Fr, Fr)>,
    /// Where the keys for carrying out the proving can be found.
    pub key_location: KeyLocation,
}

impl ProofPreimage {
    /// Runs witness generation and checks for correctness without generating a
    /// proof
    #[allow(unused_variables)]
    pub fn check(&self, ir: &IrSource) -> Result<Vec<Option<usize>>, ProvingError> {
        ir.check(self)
    }

    /// Carries out the actual proving of the proof preimage.
    #[cfg(feature = "proving")]
    #[allow(unreachable_code, unused_variables)]
    pub async fn prove(
        &self,
        rng: impl Rng + CryptoRng,
        params: &impl ParamsProverProvider,
        resolver: &impl Resolver,
    ) -> Result<(Proof, Vec<Option<usize>>), ProvingError> {
        let (prover_key, verifier_key, ir) = resolver
            .resolve_key(self.key_location.clone())
            .await?
            .ok_or(anyhow::Error::msg(format!(
                "failed to find proving key for '{}'",
                &self.key_location.0
            )))?;
        let (proof, pis, pi_skips) = ir.prove(rng, params, prover_key, self).await?;
        debug!("proof created; verifying to make sure");
        let k = verifier_key.force_init()?.k();
        if let Err(e) = verifier_key.verify(
            &params.get_params(k).await?.as_verifier(),
            &proof,
            pis.iter().copied(),
        ) {
            error!(error = ?e, ?pis, ?ir, "self-verification failed! This may be a bug, check that your keys match!");
            return Err(e);
        }
        debug!("proof ok");
        Ok((proof, pi_skips))
    }
}

impl PartialEq for VerifierKey {
    fn eq(&self, other: &Self) -> bool {
        let mut self_ser = Vec::new();
        let mut other_ser = Vec::new();
        Serializable::serialize(self, &mut self_ser).expect("In-memory serialization must succeed");
        Serializable::serialize(other, &mut other_ser)
            .expect("In-memory serialization must succeed");
        self_ser == other_ser
    }
}

impl Eq for VerifierKey {}

impl PartialOrd for VerifierKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VerifierKey {
    fn cmp(&self, other: &Self) -> Ordering {
        let mut self_ser = Vec::new();
        let mut other_ser = Vec::new();
        Serializable::serialize(self, &mut self_ser).expect("In-memory serialization must succeed");
        Serializable::serialize(other, &mut other_ser)
            .expect("In-memory serialization must succeed");
        self_ser.cmp(&other_ser)
    }
}

/// An error during proving. The type of this should not be considered part of
/// the public API, although it may be assumed to be [`Debug`]` +
/// `[`Display`](std::fmt::Display).
pub type ProvingError = anyhow::Error;
/// An error during verifying. The type of this should not be considered part of
/// the public API, although it may be assumed to be [`Debug`]` +
/// `[`Display`](std::fmt::Display).
pub type VerifyingError = anyhow::Error;
