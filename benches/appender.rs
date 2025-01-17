// Copyright (c) 2020, KTH Royal Institute of Technology.
// SPDX-License-Identifier: AGPL-3.0-only

use arcon::prelude::{AppenderIndex, Backend, EagerAppender};
use arcon_state::{with_backend_type, BackendType};
use criterion::{criterion_group, criterion_main, Bencher, BenchmarkId, Criterion};
use std::sync::Arc;
use tempfile::tempdir;

//const CAPACITY: [usize; 3] = [5048, 12498, 20048];
const WINDOW_SIZE: usize = 20048;

fn appender(c: &mut Criterion) {
    let mut group = c.benchmark_group("appender");

    /*
    for capacity in CAPACITY.iter() {
        let window_size = WINDOW_SIZE;
        let description = format!("capacity: {}", capacity);
        #[cfg(feature = "rocks")]
        group.bench_with_input(
            BenchmarkId::new("Mean Index Rocks Backed", description.clone()),
            &(window_size, capacity),
            |b, (window_size, &capacity)| index_mean_rocks(b, *window_size, capacity),
        );
        #[cfg(feature = "sled")]
        group.bench_with_input(
            BenchmarkId::new("Mean Index Sled Backed", description.clone()),
            &(window_size, capacity),
            |b, (window_size, &capacity)| index_mean_sled(b, *window_size, capacity),
        );
    }
        */

    let window_size = WINDOW_SIZE;
    group.bench_with_input(
        BenchmarkId::new("Mean Index Eager Sled", ""),
        &(window_size),
        |b, window_size| appender_mean_eager(BackendType::Sled, *window_size, b),
    );

    #[cfg(feature = "rocks")]
    group.bench_with_input(
        BenchmarkId::new("Mean Index Eager Rocks", ""),
        &(window_size),
        |b, window_size| appender_mean_eager(BackendType::Rocks, *window_size, b),
    );

    group.finish()
}

#[inline(always)]
fn mean(numbers: &[u64]) -> f32 {
    let sum: u64 = numbers.iter().sum();
    sum as f32 / numbers.len() as f32
}

fn appender_mean_eager(backend: BackendType, window_size: usize, b: &mut Bencher) {
    let dir = tempdir().unwrap();
    with_backend_type!(backend, |B| {
        let backend = Arc::new(B::create(dir.as_ref(), "appender_backend".to_string()).unwrap());
        let mut eager_appender = EagerAppender::new("_appender", backend);

        b.iter(|| {
            for i in 0..window_size {
                let _ = eager_appender.append(i as u64).unwrap();
            }
            let consumed = eager_appender.consume().unwrap();
            mean(&consumed)
        });
    });
}

criterion_group!(benches, appender);
criterion_main!(benches);
