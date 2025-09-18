//! This module provides zero-knowledge IR used by Compact.

#[cfg(any(feature = "keygen-verify", feature = "proving"))]
use super::ParamsProverProvider;
#[cfg(feature = "proving")]
use super::Proof;
use super::ProofPreimage;
#[cfg(any(feature = "proving", feature = "keygen-prove"))]
use super::ProverKey;
#[cfg(feature = "keygen-verify")]
use super::VerifierKey;
use crate::curve::Fr;
#[cfg(any(feature = "keygen-verify", feature = "proving"))]
use crate::curve::outer;
use anyhow::Result;
use base_crypto::fab::Alignment;
use halo2_proofs::dev::cost_model::CostOptions;
use midnight_circuits::compact_std_lib::MidnightCircuit;
#[cfg(feature = "proptest")]
use proptest_derive::Arbitrary;
#[cfg(feature = "proving")]
use rand::{CryptoRng, Rng};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "proptest")]
use serialize::randomised_serialization_test;
use serialize::{Deserializable, Serializable, Version, Versioned};
#[cfg(all(feature = "proptest", test))]
use serialize::{NetworkId, deserialize, serialize, serialized_size};
use std::io::{self, Read, Write};
use std::sync::Arc;

/// A low-level IR allowing the prover to populate circuit witnesses.
#[cfg_attr(feature = "proptest", derive(Arbitrary))]
#[derive(Default, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IrSource {
    /// The number of inputs, the initial elements in the memory
    pub num_inputs: u32,
    /// Whether or not this IR should compile a communications commitment
    pub do_communications_commitment: bool,
    /// The sequence of instructions to run in-circuit
    pub instructions: Arc<Vec<Instruction>>,
}

/// An index referring to the circuit memory of the IR machine
pub type Index = u32;

#[cfg(feature = "serde")]
fn field_ser<S: serde::Serializer>(field: &Fr, serializer: S) -> Result<S::Ok, S::Error> {
    let mut repr = field.as_le_bytes();
    while repr.last() == Some(&0) && repr.len() > 1 {
        repr.pop();
    }
    serde::Serializer::serialize_str(serializer, &const_hex::encode(&repr))
}

#[cfg(feature = "serde")]
fn field_deser<'a, D: serde::Deserializer<'a>>(deserializer: D) -> Result<Fr, D::Error> {
    let repr_str: String = serde::Deserialize::deserialize(deserializer)?;
    let mut repr = repr_str.as_bytes();
    let negate = if !repr.is_empty() && repr[0] == b'-' {
        repr = &repr[1..];
        true
    } else {
        false
    };
    let bytes = const_hex::decode(repr)
        .map_err(<D::Error as serde::de::Error>::custom)?
        .into_iter()
        .collect::<Vec<_>>();
    let field = Fr::from_le_bytes(&bytes)
        .ok_or_else(|| <D::Error as serde::de::Error>::custom("Out of range for field element"))?;
    Ok(if negate { -field } else { field })
}

/// An individual ZK IR instruction
#[cfg_attr(feature = "proptest", derive(Arbitrary))]
#[derive(Clone, Debug, Versioned, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case", tag = "op"))]
pub enum Instruction {
    /// Assert that `index` has value `1`. UB if `index` is not `0` or `1`.
    ///
    /// No outputs
    Assert {
        /// The index of the boolean condition being asserted
        cond: Index,
    },
    /// Conditionally select a value. UB is `bit` is not `0` or `1`.
    ///
    /// Outputs one element, `a` or `b`
    CondSelect {
        /// A boolean selector, if `1`, select `a`, else `b`
        bit: Index,
        /// The value to select for `1`
        a: Index,
        /// The value to select for `0`
        b: Index,
    },
    /// Constrains a value to a set number of bits.
    ///
    /// No outputs
    ConstrainBits {
        /// The value to constrain
        var: Index,
        /// The number of bits to constrain it to
        bits: u32,
    },
    /// Constrains two value `a` and `b` to be equal
    ///
    /// No outputs
    ConstrainEq {
        /// The first value to constrain
        a: Index,
        /// The second value to constrain
        b: Index,
    },
    /// Constrains a value `var` to be boolean (`0` or `1`)
    ///
    /// No outputs
    ConstrainToBoolean {
        /// The value to constrain
        var: Index,
    },
    /// Creates a copy of a value `var`. Superfluous, but potentially useful
    /// in some settings, and does not extend the actual circuit
    ///
    /// Outputs one element, `var`
    Copy {
        /// The variable to copy
        var: Index,
    },
    /// Declares a variable as the next public input
    ///
    /// No outputs
    DeclarePubInput {
        /// The variable to use for the public input
        var: Index,
    },
    /// A marker informing the proof assembler that a set of public inputs
    /// belong together (typically as an instruction), and whether they are
    /// active or not.
    ///
    /// Every `DeclarePubInput` should be *followed* by a `PiSkip` covering it.
    ///
    /// No outputs, but adds activity information to [`IrSource::prove`] and
    /// [`IrSource::check`].
    PiSkip {
        /// The boolean condition under which the public input is *not* skipped
        ///
        /// This is only used to inform transcript processing, serving as a marker
        /// for which public inputs comprise an instruction.
        guard: Option<Index>,
        /// The number of public inputs to skip in this group
        count: u32,
    },
    /// Adds two elliptic curve points. UB if either is not a valid curve point.
    ///
    /// Outputs 2 elements, `c_x`, `c_y`
    EcAdd {
        /// The affine x coordiate of `a`
        a_x: Index,
        /// The affine y coordiate of `a`
        a_y: Index,
        /// The affine x coordiate of `b`
        b_x: Index,
        /// The affine y coordiate of `b`
        b_y: Index,
    },
    /// Multiplies an elliptic curve point by a scalar. UB if it is not a valid
    /// curve point.
    ///
    /// Outputs 2 elements, `c_x`, `c_y`
    EcMul {
        /// The affine x coordiate of `a`
        a_x: Index,
        /// The affine y coordiate of `a`
        a_y: Index,
        /// The scalar to multiply by
        scalar: Index,
    },
    /// Multiplies the group generator by a scalar
    ///
    /// Outputs 2 elements, `c_x`, `c_y`
    EcMulGenerator {
        /// The scalar to multiply by
        scalar: Index,
    },
    /// Hashes a sequence of field elements to an embedded curve point
    ///
    /// Outputs 2 elements, `c_x`, `c_y`
    HashToCurve {
        /// The values to hash to a curve point
        inputs: Vec<Index>,
    },
    /// Loads a constant into the circuit
    ///
    /// One output, `imm`
    LoadImm {
        /// The constant to include
        #[cfg_attr(
            feature = "serde",
            serde(serialize_with = "field_ser", deserialize_with = "field_deser")
        )]
        imm: Fr,
    },
    /// Divides with remainder by a power of two (number of bits)
    ///
    /// Two outputs, `var >> bits`, and `var & ((1 << bits) - 1)`
    DivModPowerOfTwo {
        /// The variable to divide
        var: Index,
        /// The number of bits to divide by
        bits: u32,
    },
    /// Takes two inputs, `divisor` and `modulus`, and outputs `divisor << bits | modulus`,
    /// guaranteeing that the result does not overflow the field size, and that `modulus < (1 <<
    /// bits)`. Inverse of `DivModPowerOfTwo`.
    ReconstituteField {
        /// The divisor of the reconstituted field element
        divisor: Index,
        /// The modulus of the reconstituted field element
        modulus: Index,
        /// The number of bits for `modulus`
        bits: u32,
    },
    /// Outputs a `var` from the circuit, including it in the communications
    /// commitment
    ///
    /// No outputs (at the level of the IR VM), despite the name
    Output {
        /// The variable to output
        var: Index,
    },
    /// Calls a circuit-friendly hash function on a sequence of items
    ///
    /// One output, `H(inputs)`
    TransientHash {
        /// The values to hash
        inputs: Vec<Index>,
    },
    /// Calls a long-term hash function on a sequence of items with a given
    /// alignment
    ///
    /// One output, `H(inputs)`, in the binary format.
    PersistentHash {
        /// The alignment of the inputs being passed.
        alignment: Alignment,
        /// The inputs to hash.
        inputs: Vec<Index>,
    },
    /// Tests if `a` and `b` are equal
    ///
    /// One boolean output, `a == b`
    TestEq {
        /// The first value to check for equality
        a: Index,
        /// The second value to check for equality
        b: Index,
    },
    /// Adds `a` and `b` in the prime field
    ///
    /// One output `a + b`
    Add {
        /// The first value to add
        a: Index,
        /// The second value to add
        b: Index,
    },
    /// Multiplies `a` and `b` in the prime field
    ///
    /// One output `a * b`
    Mul {
        /// The first value to multiply
        a: Index,
        /// The second value to multiply
        b: Index,
    },
    /// Negates `a` in the prime field
    ///
    /// One output `-a`
    Neg {
        /// The value to negate
        a: Index,
    },
    /// Boolean not gate
    ///
    /// One output `!a`
    Not {
        /// The value to negate
        a: Index,
    },
    /// Checks if `a` < `b`, intepreting both as `bits`-bit unsigned
    /// integers.
    /// UB if `a` or `b` exceed `bits`.
    ///
    /// One boolean output `a < b`
    LessThan {
        /// The first value to compare
        a: Index,
        /// The second value to compare
        b: Index,
        /// The number of bits to compare
        bits: u32,
    },
    /// Retrieves a public input from the public transcript outputs
    ///
    /// Outputs one element, the next public transcript output, or `0` if the
    /// guard fails
    PublicInput {
        /// An optional condition for retrieving the next public transcript
        /// output
        guard: Option<Index>,
    },
    /// Retrieves an public input from the public transcript outputs
    ///
    /// Outputs one element, the next private transcript output, or `0` if the
    /// guard fails
    PrivateInput {
        /// An optional condition for retrieving the next private transcript
        /// output
        guard: Option<Index>,
    },
}

#[cfg(feature = "serde")]
#[cfg_attr(feature = "serde", derive(Deserialize))]
struct SerdeVersion {
    major: u8,
    minor: u8,
}

#[derive(Debug)]
/// A model containing data about a specific constructed circuit
pub struct Model {
    model: CostOptions,
}

impl Model {
    /// The minimum value of `k` needed for this circuit
    pub fn k(&self) -> u8 {
        // absolute lower bound of k, regardless of circuit
        const MIN_K: u8 = 4;
        // ceil(log_2(rows + 6)); k calculated based on rows and 6 assumed blinding factors
        // this is to account for the model's k calculation currently not taking blinding factors
        // into account.
        let blinding_factor_adjusted = (self.model.rows_count + 6).ilog2() + 1
            - (self.model.rows_count + 6).is_power_of_two() as u32;
        u8::max(
            blinding_factor_adjusted as u8,
            u8::max(self.model.min_k as u8, MIN_K),
        )
    }

    /// The number of rows needed by this circuit, not counting custom gates and lookups
    pub fn rows(&self) -> usize {
        self.model.rows_count
    }
}

impl IrSource {
    /// Attempts to parse an arbitrary input as IR.
    #[cfg(feature = "serde")]
    pub fn load<R: Read>(reader: R) -> io::Result<Self> {
        let value: serde_json::Value = serde_json::from_reader(reader)?;
        match &value {
            serde_json::Value::Object(obj) => {
                let ver = serde_json::from_value(
                    obj.get("version")
                        .ok_or(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Expected a version entry",
                        ))?
                        .clone(),
                )?;
                match ver {
                    SerdeVersion { major: 2, minor: 0 } => Ok(serde_json::from_value(value)?),
                    SerdeVersion { major, minor } => Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Unhandled version: {major}.{minor}"),
                    )),
                }
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Expected a JSON object",
            )),
        }
    }

    /// Returns the k value for this circuit
    pub fn k(&self) -> u8 {
        MidnightCircuit::from_relation(self).min_k() as u8
    }

    /// Retrieves a model representation of this circuit.
    #[cfg(any(feature = "keygen-verify", feature = "proving"))]
    pub fn model(&self, k: Option<u8>) -> Model {
        use halo2_proofs::dev::cost_model::from_circuit_to_cost_model_options;
        use midnight_circuits::compact_std_lib::MidnightCircuit;

        let model = from_circuit_to_cost_model_options::<outer::Scalar, MidnightCircuit<IrSource>>(
            k.map(|k| k as u32),
            &MidnightCircuit::from_relation(self),
            self.instructions
                .iter()
                .filter(|op| matches!(op, Instruction::DeclarePubInput { .. }))
                .count(),
        );

        Model { model }
    }

    /// Performs key generation on this circuit, outputting the prover key
    #[cfg(feature = "keygen-verify")]
    pub async fn keygen_vk(&self, params: &impl ParamsProverProvider) -> Result<VerifierKey> {
        use midnight_circuits::compact_std_lib::setup_vk;

        use super::InnerVerifierKey;
        use std::sync::Mutex;

        let vk = VerifierKey(Arc::new(Mutex::new(InnerVerifierKey::Initialized(
            setup_vk(&params.get_params(self.k()).await?.0, self),
        ))));

        Ok(vk)
    }

    /// Performs key generation on this circuit, outputting the prover/verifier
    /// key pair
    #[cfg(feature = "keygen-prove")]
    pub async fn keygen(
        &self,
        params: &impl ParamsProverProvider,
    ) -> Result<(ProverKey, VerifierKey)> {
        use midnight_circuits::compact_std_lib::{setup_pk, setup_vk};

        use crate::proofs::{InnerProverKey, InnerVerifierKey};
        use std::sync::Mutex;

        let vk = setup_vk(&params.get_params(self.k()).await?.0, self);
        let pk = setup_pk(self, &vk);

        Ok((
            ProverKey(Arc::new(Mutex::new(InnerProverKey::Initialized(Arc::new(
                pk,
            ))))),
            VerifierKey(Arc::new(Mutex::new(InnerVerifierKey::Initialized(vk)))),
        ))
    }

    /// Runs witness generation and checks for correctness without generating a
    /// proof
    pub fn check(&self, proof_preimage: &ProofPreimage) -> Result<Vec<Option<usize>>> {
        Ok(self.preprocess(proof_preimage)?.pi_skips)
    }

    /// Intended for testing only. This method enables fully controlling the inputs passed to
    /// proving, to test malicious prover behaviour.
    #[cfg(feature = "proving")]
    pub async fn prove_unchecked<R: Rng + CryptoRng>(
        &self,
        rng: R,
        params: &impl ParamsProverProvider,
        pk: ProverKey,
        preproc: super::ir_vm::Preprocessed,
    ) -> Result<Proof> {
        use midnight_circuits::compact_std_lib::prove;

        let params_k = params.get_params(pk.force_init()?.k()).await?;
        let pis = preproc.pis.clone();

        let pk = pk
            .force_init()
            .map_err(|_| anyhow::anyhow!("Could not init pk"))?;

        let proof = prove(&params_k.0, &pk, self, &pis, preproc, rng)?;

        Ok(Proof(proof))
    }

    /// Runs the prover against a proof preimage
    #[cfg(feature = "proving")]
    pub async fn prove<R: Rng + CryptoRng>(
        &self,
        rng: R,
        params: &impl ParamsProverProvider,
        pk: ProverKey,
        proof_preimage: &ProofPreimage,
    ) -> Result<(Proof, Vec<Fr>, Vec<Option<usize>>)> {
        use midnight_circuits::compact_std_lib::prove;

        let params_k = params.get_params(pk.force_init()?.k()).await?;
        let preproc = self.preprocess(proof_preimage)?;
        let pis = preproc.pis.clone();
        let pi_skips = preproc.pi_skips.clone();

        let pk = pk
            .force_init()
            .map_err(|_| anyhow::anyhow!("Could not init pk"))?;

        let proof = prove(&params_k.0, &pk, self, &pis, preproc, rng)?;

        Ok((Proof(proof), pis.into_iter().map(Fr).collect(), pi_skips))
    }
}

impl Versioned for IrSource {
    const VERSION: Option<Version> = Some(Version { major: 2, minor: 0 });
}

impl Deserializable for IrSource {
    fn versioned_deserialize<R: Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> io::Result<Self> {
        match version {
            Some(Version { major: 2, minor: 0 }) => Ok(IrSource {
                num_inputs: Deserializable::deserialize(reader, recursion_depth)?,
                do_communications_commitment: Deserializable::deserialize(reader, recursion_depth)?,
                instructions: Arc::new(Deserializable::deserialize(reader, recursion_depth)?),
            }),
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version.".into(),
            )),
        }
    }
}

#[cfg(feature = "proptest")]
randomised_serialization_test!(IrSource);

impl Serializable for IrSource {
    fn unversioned_serialize<W: Write>(value: &Self, writer: &mut W) -> io::Result<()> {
        Serializable::unversioned_serialize(&value.num_inputs, writer)?;
        Serializable::unversioned_serialize(&value.do_communications_commitment, writer)?;
        Serializable::unversioned_serialize(&*value.instructions, writer)?;
        Ok(())
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        Serializable::unversioned_serialized_size(&value.num_inputs)
            + Serializable::unversioned_serialized_size(&value.do_communications_commitment)
            + Serializable::unversioned_serialized_size(&*value.instructions)
    }
}

impl Deserializable for Instruction {
    fn versioned_deserialize<R: Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> io::Result<Self> {
        let mut discriminant = [0u8];
        reader.read_exact(&mut discriminant[..])?;
        use Instruction as I;
        Ok(match discriminant[0] {
            0u8 => I::Assert {
                cond: Deserializable::deserialize(reader, recursion_depth)?,
            },
            1u8 => I::CondSelect {
                bit: Deserializable::deserialize(reader, recursion_depth)?,
                a: Deserializable::deserialize(reader, recursion_depth)?,
                b: Deserializable::deserialize(reader, recursion_depth)?,
            },
            2u8 => I::ConstrainBits {
                var: Deserializable::deserialize(reader, recursion_depth)?,
                bits: Deserializable::deserialize(reader, recursion_depth)?,
            },
            3u8 => I::ConstrainEq {
                a: Deserializable::deserialize(reader, recursion_depth)?,
                b: Deserializable::deserialize(reader, recursion_depth)?,
            },
            4u8 => I::ConstrainToBoolean {
                var: Deserializable::deserialize(reader, recursion_depth)?,
            },
            5u8 => I::Copy {
                var: Deserializable::deserialize(reader, recursion_depth)?,
            },
            6u8 => I::DeclarePubInput {
                var: Deserializable::deserialize(reader, recursion_depth)?,
            },
            7u8 => I::PiSkip {
                guard: Deserializable::deserialize(reader, recursion_depth)?,
                count: Deserializable::deserialize(reader, recursion_depth)?,
            },
            8u8 => I::EcAdd {
                a_x: Deserializable::deserialize(reader, recursion_depth)?,
                a_y: Deserializable::deserialize(reader, recursion_depth)?,
                b_x: Deserializable::deserialize(reader, recursion_depth)?,
                b_y: Deserializable::deserialize(reader, recursion_depth)?,
            },
            9u8 => I::EcMul {
                a_x: Deserializable::deserialize(reader, recursion_depth)?,
                a_y: Deserializable::deserialize(reader, recursion_depth)?,
                scalar: Deserializable::deserialize(reader, recursion_depth)?,
            },
            10u8 => I::EcMulGenerator {
                scalar: Deserializable::deserialize(reader, recursion_depth)?,
            },
            11u8 => I::HashToCurve {
                inputs: Deserializable::deserialize(reader, recursion_depth)?,
            },
            12u8 => I::LoadImm {
                imm: Deserializable::deserialize(reader, recursion_depth)?,
            },
            13u8 => I::DivModPowerOfTwo {
                var: Deserializable::deserialize(reader, recursion_depth)?,
                bits: Deserializable::deserialize(reader, recursion_depth)?,
            },
            14u8 => I::Output {
                var: Deserializable::deserialize(reader, recursion_depth)?,
            },
            15u8 => I::TransientHash {
                inputs: Deserializable::deserialize(reader, recursion_depth)?,
            },
            16u8 => I::TestEq {
                a: Deserializable::deserialize(reader, recursion_depth)?,
                b: Deserializable::deserialize(reader, recursion_depth)?,
            },
            17u8 => I::Add {
                a: Deserializable::deserialize(reader, recursion_depth)?,
                b: Deserializable::deserialize(reader, recursion_depth)?,
            },
            18u8 => I::Mul {
                a: Deserializable::deserialize(reader, recursion_depth)?,
                b: Deserializable::deserialize(reader, recursion_depth)?,
            },
            19u8 => I::Neg {
                a: Deserializable::deserialize(reader, recursion_depth)?,
            },
            23u8 => I::Not {
                a: Deserializable::deserialize(reader, recursion_depth)?,
            },
            25u8 => I::LessThan {
                a: Deserializable::deserialize(reader, recursion_depth)?,
                b: Deserializable::deserialize(reader, recursion_depth)?,
                bits: Deserializable::deserialize(reader, recursion_depth)?,
            },
            26u8 => I::PublicInput {
                guard: Deserializable::deserialize(reader, recursion_depth)?,
            },
            27u8 => I::PrivateInput {
                guard: Deserializable::deserialize(reader, recursion_depth)?,
            },
            28u8 => I::ReconstituteField {
                divisor: Deserializable::deserialize(reader, recursion_depth)?,
                modulus: Deserializable::deserialize(reader, recursion_depth)?,
                bits: Deserializable::deserialize(reader, recursion_depth)?,
            },
            29u8 => I::PersistentHash {
                alignment: Deserializable::deserialize(reader, recursion_depth)?,
                inputs: Deserializable::deserialize(reader, recursion_depth)?,
            },
            _ => {
                return Err(Self::deserialization_error(
                    version,
                    "Invalid discriminant.".into(),
                ));
            }
        })
    }
}

#[cfg(feature = "proptest")]
randomised_serialization_test!(Instruction);

impl Serializable for Instruction {
    fn unversioned_serialize<W: Write>(value: &Self, writer: &mut W) -> io::Result<()> {
        use Instruction as I;
        match value {
            I::Assert { cond } => Serializable::unversioned_serialize(&(0u8, *cond), writer),
            I::CondSelect { bit, a, b } => {
                Serializable::unversioned_serialize(&(1u8, bit, a, b), writer)
            }
            I::ConstrainBits { var, bits } => {
                Serializable::unversioned_serialize(&(2u8, var, bits), writer)
            }
            I::ConstrainEq { a, b } => Serializable::unversioned_serialize(&(3u8, a, b), writer),
            I::ConstrainToBoolean { var } => {
                Serializable::unversioned_serialize(&(4u8, var), writer)
            }
            I::Copy { var } => Serializable::unversioned_serialize(&(5u8, var), writer),
            I::DeclarePubInput { var } => Serializable::unversioned_serialize(&(6u8, var), writer),
            I::PiSkip { guard, count } => {
                Serializable::unversioned_serialize(&(7u8, guard, count), writer)
            }
            I::EcAdd { a_x, a_y, b_x, b_y } => {
                Serializable::unversioned_serialize(&(8u8, a_x, a_y, b_x, b_y), writer)
            }
            I::EcMul { a_x, a_y, scalar } => {
                Serializable::unversioned_serialize(&(9u8, a_x, a_y, scalar), writer)
            }
            I::EcMulGenerator { scalar } => {
                Serializable::unversioned_serialize(&(10u8, scalar), writer)
            }
            I::HashToCurve { inputs } => {
                Serializable::unversioned_serialize(&(11u8, inputs), writer)
            }
            I::LoadImm { imm } => Serializable::unversioned_serialize(&(12u8, imm), writer),
            I::DivModPowerOfTwo { var, bits } => {
                Serializable::unversioned_serialize(&(13u8, var, bits), writer)
            }
            I::ReconstituteField {
                divisor,
                modulus,
                bits,
            } => Serializable::unversioned_serialize(&(28u8, divisor, modulus, bits), writer),
            I::Output { var } => Serializable::unversioned_serialize(&(14u8, var), writer),
            I::TransientHash { inputs } => {
                Serializable::unversioned_serialize(&(15u8, inputs), writer)
            }
            I::PersistentHash { alignment, inputs } => {
                Serializable::unversioned_serialize(&(29u8, alignment, inputs), writer)
            }
            I::TestEq { a, b } => Serializable::unversioned_serialize(&(16u8, a, b), writer),
            I::Add { a, b } => Serializable::unversioned_serialize(&(17u8, a, b), writer),
            I::Mul { a, b } => Serializable::unversioned_serialize(&(18u8, a, b), writer),
            I::Neg { a } => Serializable::unversioned_serialize(&(19u8, a), writer),
            I::Not { a } => Serializable::unversioned_serialize(&(23u8, a), writer),
            I::LessThan { a, b, bits } => {
                Serializable::unversioned_serialize(&(25u8, a, b, bits), writer)
            }
            I::PublicInput { guard } => Serializable::unversioned_serialize(&(26u8, guard), writer),
            I::PrivateInput { guard } => {
                Serializable::unversioned_serialize(&(27u8, guard), writer)
            }
        }
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        use Instruction as I;
        match value {
            I::Assert { cond } => Serializable::unversioned_serialized_size(&(0u8, *cond)),
            I::CondSelect { bit, a, b } => {
                Serializable::unversioned_serialized_size(&(1u8, bit, a, b))
            }
            I::ConstrainBits { var, bits } => {
                Serializable::unversioned_serialized_size(&(2u8, var, bits))
            }
            I::ConstrainEq { a, b } => Serializable::unversioned_serialized_size(&(3u8, a, b)),
            I::ConstrainToBoolean { var } => Serializable::unversioned_serialized_size(&(4u8, var)),
            I::Copy { var } => Serializable::unversioned_serialized_size(&(5u8, var)),
            I::DeclarePubInput { var } => Serializable::unversioned_serialized_size(&(6u8, var)),
            I::PiSkip { guard, count } => {
                Serializable::unversioned_serialized_size(&(7u8, guard, count))
            }
            I::EcAdd { a_x, a_y, b_x, b_y } => {
                Serializable::unversioned_serialized_size(&(8u8, a_x, a_y, b_x, b_y))
            }
            I::EcMul { a_x, a_y, scalar } => {
                Serializable::unversioned_serialized_size(&(9u8, a_x, a_y, scalar))
            }
            I::EcMulGenerator { scalar } => {
                Serializable::unversioned_serialized_size(&(10u8, scalar))
            }
            I::HashToCurve { inputs } => Serializable::unversioned_serialized_size(&(11u8, inputs)),
            I::LoadImm { imm } => Serializable::unversioned_serialized_size(&(12u8, imm)),
            I::DivModPowerOfTwo { var, bits } => {
                Serializable::unversioned_serialized_size(&(13u8, var, bits))
            }
            I::ReconstituteField {
                divisor,
                modulus,
                bits,
            } => Serializable::unversioned_serialized_size(&(28u8, divisor, modulus, bits)),
            I::Output { var } => Serializable::unversioned_serialized_size(&(14u8, var)),
            I::TransientHash { inputs } => {
                Serializable::unversioned_serialized_size(&(15u8, inputs))
            }
            I::PersistentHash { alignment, inputs } => {
                Serializable::unversioned_serialized_size(&(15u8, alignment, inputs))
            }
            I::TestEq { a, b } => Serializable::unversioned_serialized_size(&(16u8, a, b)),
            I::Add { a, b } => Serializable::unversioned_serialized_size(&(17u8, a, b)),
            I::Mul { a, b } => Serializable::unversioned_serialized_size(&(18u8, a, b)),
            I::Neg { a } => Serializable::unversioned_serialized_size(&(19u8, a)),
            I::Not { a } => Serializable::unversioned_serialized_size(&(23u8, a)),
            I::LessThan { a, b, bits } => {
                Serializable::unversioned_serialized_size(&(25u8, a, b, bits))
            }
            I::PublicInput { guard } => Serializable::unversioned_serialized_size(&(26u8, guard)),
            I::PrivateInput { guard } => Serializable::unversioned_serialized_size(&(27u8, guard)),
        }
    }
}
