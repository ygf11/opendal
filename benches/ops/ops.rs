// Copyright 2022 Datafuse Labs.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use criterion::{BenchmarkId, Criterion};
use futures::io;
use futures::io::BufReader;
use futures::io::Cursor;
use rand::prelude::*;

use opendal::readers::SeekableReader;
use opendal::Operator;

use super::fs;
use super::s3;

pub fn bench(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut rng = thread_rng();

    let size = 16 * 1024 * 1024; // Test with 16MB.
    let content = gen_bytes(&mut rng, size);

    let cases = vec![
        ("fs", runtime.block_on(fs::init())),
        ("s3", runtime.block_on(s3::init())),
    ];

    for case in cases {
        if case.1.is_err() {
            continue;
        }
        let op = case.1.unwrap();
        let path = uuid::Uuid::new_v4().to_string();

        // Write file before test.
        runtime
            .block_on(
                op.write(&path, size as u64)
                    .run(Box::new(Cursor::new(content.clone()))),
            )
            .expect("write failed");

        let mut group = c.benchmark_group(case.0);
        group.throughput(criterion::Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::new("bench_read", &path),
            &(op.clone(), &path),
            |b, input| {
                b.to_async(&runtime)
                    .iter(|| bench_read(input.0.clone(), input.1))
            },
        );
        group.bench_with_input(
            BenchmarkId::new("bench_seekable_read", &path),
            &(op.clone(), &path, size),
            |b, input| {
                b.to_async(&runtime)
                    .iter(|| bench_seekable_read(input.0.clone(), input.1, input.2 as u64))
            },
        );
        group.finish();
    }
}

fn gen_bytes(rng: &mut ThreadRng, size: usize) -> Vec<u8> {
    let mut content = vec![0; size as usize];
    rng.fill_bytes(&mut content);

    content
}

pub async fn bench_read(op: Operator, path: &str) {
    let mut r = op.read(path).run().await.unwrap();
    io::copy(&mut r, &mut io::sink()).await.unwrap();
}

pub async fn bench_seekable_read(op: Operator, path: &str, total: u64) {
    let r = SeekableReader::new(op, path, total);
    let mut r = BufReader::with_capacity(1024 * 1024, r);

    io::copy(&mut r, &mut io::sink()).await.unwrap();
}
