use bytes::Bytes;
use criterion::{criterion_group, criterion_main, Criterion};

use rsync_rust::delta;
use rsync_rust::signature;

pub fn signature_benchmark(c: &mut Criterion) {
    let bytes = include_bytes!("file1");

    c.bench_function("signature [1_000_000 bytes]", |b| {
        b.iter(|| signature::compute_signature(bytes.to_vec().into(), 100))
    });
}

pub fn delta_benchmark(c: &mut Criterion) {
    let bytes = include_bytes!("file1");
    let signature = signature::compute_signature(bytes.to_vec().into(), 100);

    let desired: Bytes = include_bytes!("file2").to_vec().into();

    c.bench_function("delta from file and signature [1_000_000 bytes]", |b| {
        b.iter(|| delta::compute_delta_to_our_file(signature.clone(), desired.clone(), 100))
    });
}

criterion_group!(benches, signature_benchmark, delta_benchmark);
criterion_main!(benches);
