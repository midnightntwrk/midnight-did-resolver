use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use midnight_storage::storage::Map;
use pprof::criterion::{Output, PProfProfiler};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

pub fn map_insert(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(0x42);

    let mut group = c.benchmark_group("map_insert");
    for size in [10, 100, 1_000, 10_000] {
        let mut map: Map<u64, u64> = Map::new();
        for _ in 0..size {
            map = map.insert(rng.gen(), rng.gen());
        }

        group.bench_with_input(BenchmarkId::from_parameter(size), &map, |b, map| {
            b.iter(|| {
                black_box(map.insert(rng.gen(), rng.gen()));
            });
        });
    }
    group.finish();
}

criterion_group!(
    name = benchmarking;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = map_insert
);
criterion_main!(benchmarking);
