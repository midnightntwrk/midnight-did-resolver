#[cfg(feature = "proving")]
use crate::base_crypto::data_provider::{self, MidnightDataProvider};
use crate::base_crypto::rng::SplittableRng;
use crate::error::{MalformedTransaction, TransactionProvingError};
#[cfg(feature = "proving")]
pub use crate::prove::Resolver;
use crate::semantics::{TransactionContext, TransactionResult};
#[cfg(feature = "proving")]
use crate::serialize::Deserializable;
#[cfg(feature = "proving")]
use crate::serialize::deserialize;
#[cfg(feature = "proving")]
use crate::serialize::{NetworkId, serialize};
#[cfg(feature = "proving")]
use crate::structure::Proof;
#[cfg(feature = "proving")]
use crate::structure::ProvingData;
use crate::structure::{
    DUMMY_PARAMETERS, DUMMY_TRANSACTION_COST_MODEL, LedgerState, ProofPreimage, Proofish,
    Transaction,
};
#[cfg(feature = "proving")]
use crate::transient_crypto::proofs::ProverKey;
use coin_structure::coin::{Info as CoinInfo, NATIVE_TOKEN};
#[cfg(feature = "proving")]
use lazy_static::lazy_static;
use rand::{CryptoRng, Rng};
#[cfg(feature = "proving")]
use reqwest::Client;
#[cfg(feature = "proving")]
use std::collections::HashMap;
#[cfg(feature = "proving")]
use std::env;
#[cfg(feature = "proving")]
use std::fs::File;
use std::io;
#[cfg(feature = "proving")]
use std::io::BufReader;
use zswap::keys::SecretKeys;
use zswap::local::State as ZswapLocalState;
#[cfg(feature = "proving")]
use zswap::prove::ZswapResolver;
use zswap::storage::db::DB;
#[cfg(feature = "proving")]
use zswap::transient_crypto::proofs::KeyLocation;
use zswap::transient_crypto::proofs::VerifierKey;
use zswap::{Offer as ZswapOffer, Output as ZswapOutput};

const BALANCING_OVERHEAD: u128 = DUMMY_TRANSACTION_COST_MODEL.input_fee_overhead()
    + DUMMY_TRANSACTION_COST_MODEL.output_fee_overhead()
    // To cover misc size deltas, and as a buffer for discrepancies between proven/unproven
    + 20_000;

#[cfg(feature = "proving")]
pub type Pk = ProverKey;
#[cfg(not(feature = "proving"))]
pub type Pk = ();

#[cfg(feature = "proving")]
pub type Tx<D> = Transaction<Proof, D>;
#[cfg(not(feature = "proving"))]
pub type Tx<D> = Transaction<(), D>;

#[cfg(not(feature = "proving"))]
pub type Resolver = ();

#[cfg(feature = "proving")]
lazy_static! {
    pub static ref PUBLIC_PARAMS: ZswapResolver = ZswapResolver(MidnightDataProvider::new(
        data_provider::FetchMode::OnDemand,
        data_provider::OutputMode::Log,
        zswap::ZSWAP_EXPECTED_FILES.to_owned(),
    ));
}

impl<D: DB> LedgerState<D> {
    pub fn assert_apply<P: Proofish<D>>(&self, tx: &Transaction<P, D>) -> Self {
        let (next, res) = self.apply(tx, &TransactionContext::default());
        dbg!(&res);
        assert!(matches!(res, TransactionResult::Success));
        next
    }
}

#[cfg(not(feature = "proving"))]
pub fn test_resolver(_test_name: &'static str) -> Resolver {
    ()
}

#[cfg(feature = "proving")]
pub async fn verifier_key(resolver: &Resolver, name: &'static str) -> Option<VerifierKey> {
    match resolver
        .resolve(&KeyLocation(std::borrow::Cow::Borrowed(name)))
        .await
        .ok()??
    {
        ProvingData::V4(_, vk, _) => Some(vk),
    }
}

#[cfg(not(feature = "proving"))]
pub async fn verifier_key(_resolver: &Resolver, _name: &'static str) -> Option<VerifierKey> {
    None
}

#[cfg(feature = "proving")]
pub fn test_resolver(test_name: &'static str) -> Resolver {
    let test_dir = env::var("MIDNIGHT_LEDGER_TEST_STATIC_DIR").unwrap();
    Resolver::new(
        PUBLIC_PARAMS.clone(),
        Box::new(move |KeyLocation(loc)| {
            let files = [("keys", "prover"), ("keys", "verifier"), ("zkir", "bzkir")]
                .iter()
                .map(|(dir, ext)| format!("{test_dir}/{test_name}/{dir}/{loc}.{ext}"))
                .map(|path| Ok::<_, io::Error>(BufReader::new(File::open(path)?)))
                .collect::<Result<Vec<_>, _>>();
            let mut files = match files {
                Ok(f) => f,
                Err(e) => {
                    return Box::pin(std::future::ready(if io::ErrorKind::NotFound == e.kind() {
                        Ok(None)
                    } else {
                        Err(e)
                    }));
                }
            };
            let pk = Deserializable::deserialize(&mut files[0], 0).unwrap();
            let vk = Deserializable::deserialize(&mut files[1], 0).unwrap();
            let ir = Deserializable::deserialize(&mut files[2], 0).unwrap();
            Box::pin(std::future::ready(Ok(Some(ProvingData::V4(pk, vk, ir)))))
        }),
    )
}

#[derive(Debug)]
pub enum ClientProvingError<D: DB> {
    Io(io::Error),
    Local(TransactionProvingError<D>),
    Reqwest(reqwest::Error),
}

pub async fn tx_prove<R: Rng + CryptoRng + SplittableRng, D: DB>(
    #[cfg_attr(feature = "proving", allow(unused_mut))] mut rng: R,
    tx: &Transaction<ProofPreimage, D>,
    #[cfg_attr(not(feature = "proving"), allow(unused_variables))] resolver: &Resolver,
) -> Result<Tx<D>, ClientProvingError<D>> {
    #[cfg(feature = "proving")]
    {
        if let Ok(addr) = env::var("MIDNIGHT_PROOF_SERVER") {
            let ser = serialize_request_body(tx, resolver, NetworkId::Undeployed).await?;
            println!("    Proving request: {} bytes", ser.len());
            let resp = Client::new()
                .post(format!("{addr}/prove-tx"))
                .body(ser)
                .send()
                .await
                .map_err(ClientProvingError::Reqwest)?;
            if resp.status().is_success() {
                let bytes = resp.bytes().await.map_err(ClientProvingError::Reqwest)?;
                println!("    Proving response: {} bytes", bytes.len());
                deserialize(&mut bytes.to_vec().as_slice(), NetworkId::Undeployed)
                    .map_err(ClientProvingError::Io)
            } else {
                panic!(
                    "proving server error: {}",
                    resp.text().await.expect("error retrieving error")
                )
            }
        } else {
            tx.prove(rng, &*PUBLIC_PARAMS, resolver)
                .await
                .map_err(ClientProvingError::Local)
        }
    }
    #[cfg(not(feature = "proving"))]
    Ok(tx.erase_proofs(&mut rng))
}

#[cfg(feature = "proving")]
pub async fn serialize_request_body<D: DB>(
    tx: &Transaction<ProofPreimage, D>,
    resolver: &Resolver,
    netid: NetworkId,
) -> Result<Vec<u8>, ClientProvingError<D>> {
    let circuits_used = tx
        .calls()
        .map(|c| String::from_utf8_lossy(&c.entry_point).into_owned())
        .collect::<Vec<_>>();
    let mut keys = HashMap::new();
    for k in circuits_used.into_iter() {
        let k = KeyLocation(std::borrow::Cow::Owned(k));
        let data = resolver.resolve(&k).await.map_err(ClientProvingError::Io)?;
        if let Some(data) = data {
            keys.insert(k, data);
        }
    }
    let mut ser = Vec::new();
    serialize(tx, &mut ser, netid).map_err(ClientProvingError::Io)?;
    serialize(&keys, &mut ser, netid).map_err(ClientProvingError::Io)?;
    Ok(ser)
}

pub trait ProofishExt<D: DB>: Proofish<D> {
    // Allowed as this is testing-only API
    #[allow(async_fn_in_trait)]
    async fn from_unproven(
        rng: impl CryptoRng + SplittableRng,
        resolver: &Resolver,
        tx: Transaction<ProofPreimage, D>,
    ) -> Transaction<Self, D>;
}

impl<D: DB> ProofishExt<D> for ProofPreimage {
    async fn from_unproven(
        _rng: impl CryptoRng + SplittableRng,
        _resolver: &Resolver,
        tx: Transaction<ProofPreimage, D>,
    ) -> Transaction<Self, D> {
        tx
    }
}

impl<D: DB> ProofishExt<D> for () {
    async fn from_unproven(
        mut rng: impl CryptoRng + SplittableRng,
        _resolver: &Resolver,
        tx: Transaction<ProofPreimage, D>,
    ) -> Transaction<Self, D> {
        tx.erase_proofs(&mut rng)
    }
}

#[cfg(feature = "proving")]
impl<D: DB> ProofishExt<D> for Proof {
    async fn from_unproven(
        rng: impl CryptoRng + SplittableRng,
        resolver: &Resolver,
        tx: Transaction<ProofPreimage, D>,
    ) -> Transaction<Self, D> {
        tx_prove(rng, &tx, resolver).await.unwrap()
    }
}

pub async fn balance_tx<R: Rng + CryptoRng + SplittableRng, P: ProofishExt<D>, D: DB>(
    mut rng: R,
    tx: Transaction<P, D>,
    zswap_state: &mut ZswapLocalState<D>,
    secret_key: &SecretKeys,
    resolver: &Resolver,
) -> Result<Transaction<P, D>, MalformedTransaction> {
    let fees = tx.fees(&DUMMY_PARAMETERS)?;
    // Balance partial tx
    let imbalance = -tx.imbalances(true, Some(fees))[0].1 as u128 + BALANCING_OVERHEAD;
    let coin_in = zswap_state
        .coins
        .iter()
        .map(|a| a.1)
        .find(|c| c.type_ == NATIVE_TOKEN)
        .unwrap()
        .clone();
    let change = coin_in.value - imbalance;
    let coin_out = CoinInfo::new(&mut rng, change, NATIVE_TOKEN);
    let out =
        ZswapOutput::new(&mut rng, &coin_out, 0, &secret_key.coin_public_key(), None).unwrap();
    let (zswap_state2, in_) = zswap_state
        .spend(&mut rng, secret_key, &coin_in, 0)
        .unwrap();
    *zswap_state = zswap_state2.watch_for(&secret_key.coin_public_key(), &coin_out);
    let offer = ZswapOffer {
        inputs: vec![in_],
        outputs: vec![out],
        transient: Vec::new(),
        deltas: vec![(NATIVE_TOKEN, imbalance as i128)],
    };
    let unproven = Transaction::new(offer, None, None);

    let tx2 = P::from_unproven(rng.split(), resolver, unproven).await;
    tx.merge(&tx2)
}

#[cfg(feature = "proving")]
#[allow(clippy::type_complexity, clippy::result_large_err)]
pub fn well_formed_tx_builder<R: Rng + CryptoRng + SplittableRng, D: DB>(
    mut rng: R,
    secret_key: &SecretKeys,
    resolver: &Resolver,
) -> Result<Transaction<Proof, D>, TransactionProvingError<D>> {
    const MINT_AMOUNT: u128 = 5000000000;
    let coin = CoinInfo::new(&mut rng, MINT_AMOUNT, NATIVE_TOKEN);
    let out = ZswapOutput::new(&mut rng, &coin, 0, &secret_key.coin_public_key(), None).unwrap();
    let offer = ZswapOffer {
        inputs: Vec::new(),
        outputs: vec![out],
        transient: Vec::new(),
        deltas: vec![(NATIVE_TOKEN, -(MINT_AMOUNT as i128))],
    };

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        Transaction::new(offer, None, None)
            .prove(rng, &*PUBLIC_PARAMS, resolver)
            .await
    })
}
