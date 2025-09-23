#![deny(warnings)]

use base_crypto::data_provider::{self, MidnightDataProvider};
use base_crypto::rng::SplittableRng;
use coin_structure::coin::TokenType;
use coin_structure::coin::{Info as CoinInfo, NATIVE_TOKEN};
use coin_structure::storage::db::InMemoryDB;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use midnight_zswap::keys::SecretKeys;
use midnight_zswap::ledger::State as ZswapLedgerState;
use midnight_zswap::local::State as ZswapLocalState;
use midnight_zswap::prove::ZswapResolver;
use midnight_zswap::serialize::Deserializable;
use midnight_zswap::{Offer, Output as ZswapOutput, ZSWAP_EXPECTED_FILES};
use rand::SeedableRng;
use rand::rngs::OsRng;
use rand::rngs::StdRng;
use rand::{CryptoRng, Rng};
use transient_crypto::proofs::{
    ParamsProverProvider, Proof, ProofPreimage, ProvingError, Resolver, VerifierKey,
};

fn sync_prove(
    offer: &Offer<ProofPreimage>,
    rng: impl CryptoRng + SplittableRng,
    pp: &impl ParamsProverProvider,
    resolver: &impl Resolver,
) -> Result<Offer<Proof>, ProvingError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async { offer.prove(rng, pp, resolver).await })
}

pub fn zswap_ledger(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(0x42);
    let mut zswap_local_state = ZswapLocalState::<InMemoryDB>::new();
    let keys = SecretKeys::from_rng_seed(&mut rng);
    let zswap_state: ZswapLedgerState<InMemoryDB> = ZswapLedgerState::new();
    let coin = CoinInfo {
        nonce: OsRng.gen(),
        type_: TokenType(OsRng.gen()),
        value: OsRng.gen::<u64>() as u128,
    };
    let resolver = ZswapResolver(MidnightDataProvider::new(
        data_provider::FetchMode::Synchronous,
        data_provider::OutputMode::Log,
        ZSWAP_EXPECTED_FILES.to_owned(),
    ));
    let output = ZswapOutput::new(&mut rng, &coin, 0, &keys.coin_public_key(), None).unwrap();
    let offer = Offer {
        inputs: Vec::new(),
        outputs: vec![output],
        transient: Vec::new(),
        deltas: Vec::new(),
    };
    c.bench_function("ledger::application", |b| {
        b.iter(|| {
            zswap_state.try_apply(black_box(&offer), None).unwrap();
        })
    });
    zswap_state.try_apply(&offer, None).unwrap();
    zswap_local_state = zswap_local_state.watch_for(&keys.coin_public_key(), &coin);
    zswap_local_state = zswap_local_state.apply(&keys, &offer);
    let qc = zswap_local_state.coins.iter().next().unwrap().1;
    let (_, input) = zswap_local_state.spend(&mut rng, &keys, &qc, 0).unwrap();
    let offer = Offer {
        inputs: vec![input],
        outputs: Vec::new(),
        transient: Vec::new(),
        deltas: vec![(
            coin.type_,
            coin.value.try_into().expect("coin out of bounds"),
        )],
    };

    c.bench_function("ledger::proving", |b| {
        b.iter(|| {
            sync_prove(&offer, rng.split(), &resolver, &resolver).unwrap();
        })
    });

    let offer = sync_prove(&offer, rng.split(), &resolver, &resolver).unwrap();

    c.bench_function("ledger::validation", |b| {
        b.iter(|| {
            black_box(&offer).well_formed(0).unwrap();
        })
    });
}

pub fn key_deser(c: &mut Criterion) {
    c.bench_function("vk::deserialize::spend", |b| {
        let data = std::fs::read(concat!(
            env!("MIDNIGHT_PP"),
            "/zswap/",
            env!("CARGO_PKG_VERSION"),
            "/spend.verifier",
        ))
        .unwrap();
        b.iter(|| black_box(VerifierKey::deserialize(&mut black_box(&data[..]), 0)))
    });
    c.bench_function("vk::deserialize::output", |b| {
        let data = std::fs::read(concat!(
            env!("MIDNIGHT_PP"),
            "/zswap/",
            env!("CARGO_PKG_VERSION"),
            "/output.verifier",
        ))
        .unwrap();
        b.iter(|| black_box(VerifierKey::deserialize(&mut black_box(&data[..]), 0)))
    });
}

pub fn zswap_local(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(0x42);
    let mut zswap_state = ZswapLocalState::<InMemoryDB>::new();
    let keys = SecretKeys::from_rng_seed(&mut rng);
    let resolver = ZswapResolver(MidnightDataProvider::new(
        data_provider::FetchMode::Synchronous,
        data_provider::OutputMode::Log,
        ZSWAP_EXPECTED_FILES.to_owned(),
    ));

    const MINT_AMOUNT: u128 = 5000000000;
    let coin = CoinInfo::new(&mut rng, MINT_AMOUNT, NATIVE_TOKEN);
    let out = ZswapOutput::new(&mut rng, &coin, 0, &keys.coin_public_key(), None).unwrap();

    let offer = Offer {
        inputs: Vec::new(),
        outputs: vec![out],
        transient: Vec::new(),
        deltas: vec![(NATIVE_TOKEN, -(MINT_AMOUNT as i128))],
    };
    let offer = sync_prove(&offer, rng.split(), &resolver, &resolver).unwrap();

    c.bench_function("local::application", |b| {
        b.iter(|| {
            zswap_state = zswap_state.apply(black_box(&keys), black_box(&offer));
        })
    });
}

criterion_group!(
    name = benchmarking;
    config = Criterion::default().sample_size(10);
    targets = zswap_local, zswap_ledger, key_deser);
criterion_main!(benchmarking);
