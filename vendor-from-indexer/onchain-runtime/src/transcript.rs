use crate::context::Effects;
use crate::context::QueryContext;
use crate::cost_model::CostModel;
use crate::error::TranscriptRejected;
use crate::ops::Op;
use crate::result_mode::ResultModeVerify;
use crate::serialize::{Deserializable, Serializable, Version, Versioned, check_injected_version};
use derive_where::derive_where;
use onchain_vm::storage::db::DB;
use onchain_vm::storage::db::InMemoryDB;
#[cfg(feature = "proptest")]
use proptest_derive::Arbitrary;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::iter::once;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(bound(serialize = "", deserialize = "")))]
#[cfg_attr(feature = "proptest", derive(Arbitrary))]
#[derive_where(Clone, PartialEq, Eq, Debug)]
pub struct Transcript<D: DB> {
    pub gas: u64,
    pub effects: Effects,
    pub program: Vec<Op<ResultModeVerify, D>>,
    pub version: Option<Version>,
}

impl<D: DB> Versioned for Transcript<D> {
    const VERSION: Option<Version> = Some(Version { major: 2, minor: 2 });
    const NETWORK_SPECIFIC: bool = true;
}

impl<D: DB> Serializable for Transcript<D> {
    fn unversioned_serialize<W: std::io::Write>(
        value: &Self,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        Serializable::unversioned_serialize(&value.gas, writer)?;
        Serializable::unversioned_serialize(&value.effects, writer)?;
        Serializable::unversioned_serialize(&(value.program.len() as u32), writer)?;
        for op in value.program.iter() {
            Serializable::unversioned_serialize(op, writer)?;
        }
        Ok(())
    }

    fn unversioned_serialized_size(value: &Self) -> usize {
        Serializable::unversioned_serialized_size(&value.gas)
            + Serializable::unversioned_serialized_size(&value.effects)
            + Serializable::unversioned_serialized_size(&(value.program.len() as u32))
            + value
                .program
                .iter()
                .map(Serializable::unversioned_serialized_size)
                .sum::<usize>()
    }
}

impl<D: DB> Deserializable for Transcript<D> {
    fn versioned_deserialize<R: std::io::Read>(
        reader: &mut R,
        version: Option<&Version>,
        recursion_depth: u32,
    ) -> Result<Self, std::io::Error> {
        /// Compile-time check that we're remembering to inject version information into
        /// `Op` correctly
        const _: () = check_injected_version(
            Some(Version { major: 2, minor: 2 }),
            Op::<ResultModeVerify, InMemoryDB>::VERSION,
        );
        match version {
            Some(Version { major: 2, minor: 0 })
            | Some(Version { major: 2, minor: 1 })
            | Some(Version { major: 2, minor: 2 }) => {
                let gas = <u64 as Deserializable>::deserialize(reader, recursion_depth)?;
                let effects = <Effects as Deserializable>::deserialize(reader, recursion_depth)?;
                let program_len = <u32 as Deserializable>::deserialize(reader, recursion_depth)?;
                let program = (0..program_len)
                    .map(|_| {
                        <Op<ResultModeVerify, D>>::versioned_deserialize(
                            reader,
                            version,
                            recursion_depth,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Transcript {
                    gas,
                    effects,
                    program,
                    version: version.cloned(),
                })
            }
            _ => Err(Self::deserialization_error(
                version,
                "Unsupported version.".to_string(),
            )),
        }
    }
}

impl<D: DB> Transcript<D> {
    #[deprecated(
        since = "0.2.0",
        note = "please use ledger's `partition` instead. This method will be removed with the next major version."
    )]
    #[allow(clippy::type_complexity)]
    pub fn new(
        context: &QueryContext<D>,
        program: &[Op<ResultModeVerify, D>],
        cost_model: &CostModel,
    ) -> Result<(Option<Transcript<D>>, Option<Transcript<D>>), TranscriptRejected<D>> {
        let ckpts = program
            .iter()
            .enumerate()
            .filter_map(|(i, op)| match op {
                Op::Ckpt => Some(i),
                _ => None,
            })
            .collect::<Vec<_>>();
        // This is currently a hack! We just add an error margin to the gas cost, eventually we
        // should to symbolic execution, and get something much more realistic.
        //
        // We grab the longest program up to a checkpoint where this error margin falls into our
        // conservative bound on guaranteed budget, and use that as the guaranteed transcript, with
        // the rest being the fallible one.
        let candidates = once((Some(program), None))
            .chain(
                ckpts
                    .into_iter()
                    .map(|ckpt| (Some(&program[..ckpt]), Some(&program[ckpt..]))),
            )
            .chain(once((None, Some(program))));
        fn transcript_and_state<D: DB>(
            program: &[Op<ResultModeVerify, D>],
            context: &QueryContext<D>,
            cost_model: &CostModel,
        ) -> Result<(Transcript<D>, QueryContext<D>), TranscriptRejected<D>> {
            let results = context.query(program, None, cost_model)?;
            Ok((
                Transcript {
                    gas: results.gas_cost + results.gas_cost / 5,
                    effects: results.context.effects.clone(),
                    program: program.to_vec(),
                    version: Transcript::<D>::VERSION,
                },
                results.context,
            ))
        }
        // Moved here during deprecation from the cost model.
        const GUARANTEED_COST_LIMIT_PER_BYTE: u64 = 25;
        fn guaranteed_cost_heuristic(transcript_size: usize) -> u64 {
            // Adding 1kb as a conservative estimate for the rest of the transaction.
            (transcript_size as u64 + 1000) * GUARANTEED_COST_LIMIT_PER_BYTE
        }
        for (guaranteed, fallible) in candidates {
            match (guaranteed, fallible) {
                (Some(guaranteed), Some(fallible)) => {
                    let (gtranscript, ctxt) =
                        transcript_and_state(guaranteed, context, cost_model)?;
                    let (ftranscript, _) = transcript_and_state(fallible, &ctxt, cost_model)?;
                    let transcript_size = Serializable::serialized_size(&gtranscript)
                        + Serializable::serialized_size(&ftranscript);
                    if gtranscript.gas > GUARANTEED_COST_LIMIT_PER_BYTE * transcript_size as u64 {
                        continue;
                    }
                    return Ok((Some(gtranscript), Some(ftranscript)));
                }
                (Some(guaranteed), None) => {
                    let (transcript, _) = transcript_and_state(guaranteed, context, cost_model)?;
                    let transcript_size = Serializable::serialized_size(&transcript);
                    if transcript.gas > guaranteed_cost_heuristic(transcript_size) {
                        continue;
                    }
                    return Ok((Some(transcript), None));
                }
                (None, Some(fallible)) => {
                    let (transcript, _) = transcript_and_state(fallible, context, cost_model)?;
                    return Ok((None, Some(transcript)));
                }
                (None, None) => unreachable!(),
            }
        }
        unreachable!()
    }
}
