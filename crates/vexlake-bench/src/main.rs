//! VexLake Benchmark Tool

use std::time::Instant;
use rand::Rng;
use vexlake_core::vector::{cosine_similarity, brute_force_topk};

fn main() {
    println!("VexLake Benchmark Suite");
    println!("========================\n");

    // Run vector operation benchmarks
    bench_cosine_similarity();
    bench_topk_search();
}

fn bench_cosine_similarity() {
    println!("Benchmark: Cosine Similarity");
    println!("----------------------------");

    let mut rng = rand::thread_rng();
    let dimensions = [128, 256, 512, 1024];

    for dim in dimensions {
        let a: Vec<f32> = (0..dim).map(|_| rng.gen()).collect();
        let b: Vec<f32> = (0..dim).map(|_| rng.gen()).collect();

        let iterations = 100_000;
        let start = Instant::now();

        for _ in 0..iterations {
            let _ = cosine_similarity(&a, &b);
        }

        let elapsed = start.elapsed();
        let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();

        println!(
            "  dim={:4}: {:>10.0} ops/sec ({:.2?} for {} ops)",
            dim, ops_per_sec, elapsed, iterations
        );
    }
    println!();
}

fn bench_topk_search() {
    println!("Benchmark: TopK Search (Brute Force)");
    println!("------------------------------------");

    let mut rng = rand::thread_rng();
    let dimension = 128;
    let dataset_sizes = [1_000, 10_000, 100_000];
    let k = 10;

    for size in dataset_sizes {
        let vectors: Vec<(u64, Vec<f32>)> = (0..size)
            .map(|i| {
                let v: Vec<f32> = (0..dimension).map(|_| rng.gen()).collect();
                (i as u64, v)
            })
            .collect();

        let query: Vec<f32> = (0..dimension).map(|_| rng.gen()).collect();

        let iterations = if size >= 100_000 { 10 } else { 100 };
        let start = Instant::now();

        for _ in 0..iterations {
            let _ = brute_force_topk(&query, &vectors, k);
        }

        let elapsed = start.elapsed();
        let avg_latency = elapsed / iterations;

        println!(
            "  n={:>7}: avg latency = {:>8.2?} ({} iterations)",
            size, avg_latency, iterations
        );
    }
    println!();
}
