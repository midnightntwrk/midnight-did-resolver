use criterion::{Criterion, black_box, criterion_group, criterion_main};
use midnight_transient_crypto::curve::Fr;
use midnight_transient_crypto::hash::{hash_to_curve, transient_hash};
use rand::{Rng, rngs::OsRng};

pub fn hash(c: &mut Criterion) {
    c.bench_function("transient_hash", |b| {
        b.iter(|| black_box(transient_hash(black_box(&[Fr::from(0u64), Fr::from(1u64)]))));
    });
    c.bench_function("hash_to_curve", |b| {
        b.iter(|| black_box(hash_to_curve(black_box(&[Fr::from(0u64), Fr::from(1u64)]))));
    });
}

pub fn fr_arith(c: &mut Criterion) {
    c.bench_function("fr_add", |bench| {
        let a: Fr = OsRng.gen();
        let b: Fr = OsRng.gen();
        bench.iter(|| black_box(a + b));
    });
    c.bench_function("fr_mul", |bench| {
        let a: Fr = OsRng.gen();
        let b: Fr = OsRng.gen();
        bench.iter(|| black_box(a * b));
    });
}

criterion_group!(hashes, hash, fr_arith);
criterion_main!(hashes);
