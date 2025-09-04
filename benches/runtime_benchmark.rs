use bytes::Bytes;
use criterion::{criterion_group, criterion_main, Criterion};

use rsync_rust::domain::{delta, patch, signature};

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

pub fn patch_benchmark(c: &mut Criterion) {
    let chunk_size = 100;

    let basis_file: Bytes = include_bytes!("test_files/file1").to_vec().into();
    let signature = signature::compute_signature(basis_file.to_vec().into(), chunk_size);
    let updated_file: Bytes = include_bytes!("test_files/file2").to_vec().into();
    let delta = delta::compute_delta_to_our_file(signature, updated_file, chunk_size);

    c.bench_function("applying delta to basis file [1_000_000 bytes]", |b| {
        b.iter(|| patch::apply_delta(basis_file.clone(), delta.clone(), chunk_size))
    });
}

criterion_group!(
    benches,
    signature_benchmark,
    delta_benchmark,
    patch_benchmark
);
criterion_main!(benches);
