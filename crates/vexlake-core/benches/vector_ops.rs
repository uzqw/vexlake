//! Benchmarks for vector operations

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rand::Rng;

fn random_vector(dim: usize) -> Vec<f32> {
    let mut rng = rand::thread_rng();
    (0..dim).map(|_| rng.gen()).collect()
}

fn bench_cosine_similarity(c: &mut Criterion) {
    let mut group = c.benchmark_group("cosine_similarity");

    for dim in [128, 256, 512, 1024] {
        let a = random_vector(dim);
        let b = random_vector(dim);

        group.bench_with_input(BenchmarkId::from_parameter(dim), &dim, |bench, _| {
            bench.iter(|| vexlake_core::vector::cosine_similarity(black_box(&a), black_box(&b)));
        });
    }

    group.finish();
}

fn bench_l2_distance(c: &mut Criterion) {
    let mut group = c.benchmark_group("l2_distance");

    for dim in [128, 256, 512, 1024] {
        let a = random_vector(dim);
        let b = random_vector(dim);

        group.bench_with_input(BenchmarkId::from_parameter(dim), &dim, |bench, _| {
            bench.iter(|| vexlake_core::vector::l2_distance(black_box(&a), black_box(&b)));
        });
    }

    group.finish();
}

fn bench_brute_force_topk(c: &mut Criterion) {
    let mut group = c.benchmark_group("brute_force_topk");

    let dim = 128;
    let k = 10;

    for size in [1000, 10000] {
        let vectors: Vec<(u64, Vec<f32>)> = (0..size)
            .map(|i| (i as u64, random_vector(dim)))
            .collect();
        let query = random_vector(dim);

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |bench, _| {
            bench.iter(|| {
                vexlake_core::vector::brute_force_topk(
                    black_box(&query),
                    black_box(&vectors),
                    black_box(k),
                )
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_cosine_similarity,
    bench_l2_distance,
    bench_brute_force_topk
);
criterion_main!(benches);
