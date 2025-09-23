#![deny(warnings)]

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use lazy_static::lazy_static;
use midnight_ledger::base_crypto::rng::SplittableRng;
use midnight_ledger::prove::Resolver;
use midnight_ledger::semantics::TransactionContext;
use midnight_ledger::structure::{LedgerState, OutputInstruction, SystemTransaction};
use midnight_ledger::test_utilities::{test_resolver, well_formed_tx_builder};
use midnight_ledger::verify::WellFormedStrictness;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use zswap::keys::SecretKeys;
use zswap::storage::db::InMemoryDB;

lazy_static! {
    static ref RESOLVER: Resolver = test_resolver("");
}

pub fn mint(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(0x42);
    fn mk_mint<R: Rng>(rng: &mut R, n: usize) -> SystemTransaction {
        let mut outputs = Vec::with_capacity(n);
        for _ in 0..n {
            outputs.push(OutputInstruction {
                amount: rng.gen::<u32>() as u128,
                target_key: coin_structure::coin::PublicKey(rng.gen()),
            });
        }
        SystemTransaction::Mint(outputs)
    }
    let ledger_state: LedgerState<InMemoryDB> = LedgerState::new();

    let mut group = c.benchmark_group("mint");
    for size in [100, 200, 300, 400, 500] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let tx = mk_mint(&mut rng, size);
            b.iter(|| black_box(ledger_state.apply_system_tx(&tx).unwrap()));
        });
    }
    group.finish();
}

pub fn transaction_validation(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(0x42);
    let ledger_state: LedgerState<InMemoryDB> = LedgerState::new();
    let sk = SecretKeys::from_rng_seed(&mut rng);
    let tx = well_formed_tx_builder(rng.split(), &sk, &RESOLVER).unwrap();

    let mut strictness = WellFormedStrictness::default();
    strictness.enforce_balancing = false;

    c.bench_function("verification", |b| {
        b.iter(|| {
            black_box(&tx)
                .well_formed(&ledger_state, strictness)
                .unwrap();
        })
    });
    let context = TransactionContext::default();

    c.bench_function("application", |b| {
        b.iter(|| black_box(ledger_state.apply(black_box(&tx), black_box(&context))))
    });
}

criterion_group!(
    name = benchmarking;
    config = Criterion::default().sample_size(10);
    targets = transaction_validation
);
criterion_group!(system_tx, mint);
criterion_main!(benchmarking, system_tx);
