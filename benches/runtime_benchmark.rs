use bytes::Bytes;
use criterion::{criterion_group, criterion_main, Criterion};

use rsync_rust::signature;
use rsync_rust::{delta, patch};

pub fn signature_benchmark(c: &mut Criterion) {
    let chunk_size = 100;

    let basis_file: Bytes = include_bytes!("test_files/file1").to_vec().into();

    c.bench_function("signature [1_000_000 bytes]", |b| {
        b.iter(|| signature::compute_signature(basis_file.clone(), chunk_size))
    });
}

pub fn delta_benchmark(c: &mut Criterion) {
    let chunk_size = 100;

    let basis_file: Bytes = include_bytes!("test_files/file1").to_vec().into();
    let signature = signature::compute_signature(basis_file, chunk_size);

    let updated_file: Bytes = include_bytes!("test_files/file2").to_vec().into();

    c.bench_function("delta from file and signature [1_000_000 bytes]", |b| {
        b.iter(|| {
            delta::compute_delta_to_our_file(signature.clone(), updated_file.clone(), chunk_size)
        })
    });
}

criterion_group!(benches, signature_benchmark, delta_benchmark);
criterion_main!(benches);
