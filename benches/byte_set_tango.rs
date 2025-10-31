use std::hint::black_box;

use comtains::{ByteSet, byte_set};
use rand::{Rng, SeedableRng, rngs::StdRng};
use tango_bench::{IntoBenchmarks, benchmark_fn, tango_benchmarks, tango_main};

const HOT_SET: ByteSet = byte_set![b"\xA0\xB1", b"\xA1\xB2", b"\xA1\xB2\xC3", b"\xA1\xB2\xC4",];

const HOT_SEQUENCES: [&[u8]; 4] = [b"\xA0\xB1", b"\xA1\xB2", b"\xA1\xB2\xC3", b"\xA1\xB2\xC4"];

const LARGE_SET: ByteSet = byte_set![
    b"\x10\x00",
    b"\x10\x01",
    b"\x10\x02",
    b"\x10\x03",
    b"\x10\x04",
    b"\x10\x05",
    b"\x10\x06",
    b"\x10\x07",
    b"\x10\x08",
    b"\x10\x09",
    b"\x10\x0A",
    b"\x10\x0B",
    b"\x10\x0C",
    b"\x10\x0D",
    b"\x10\x0E",
    b"\x10\x0F",
    b"\x20\x00",
    b"\x20\x01",
    b"\x20\x02",
    b"\x20\x03",
    b"\x20\x04",
    b"\x20\x05",
    b"\x20\x06",
    b"\x20\x07",
    b"\x20\x08",
    b"\x20\x09",
    b"\x20\x0A",
    b"\x20\x0B",
    b"\x20\x0C",
    b"\x20\x0D",
    b"\x20\x0E",
    b"\x20\x0F",
    b"\x30\x00\xFF",
    b"\x30\x01\xFE",
    b"\x30\x02\xFD",
    b"\x30\x03\xFC",
    b"\x40\x00\xFF\xE0",
    b"\x40\x01\xFE\xE1",
    b"\x40\x02\xFD\xE2",
    b"\x40\x03\xFC\xE3",
];

const LARGE_SEQUENCES: [&[u8]; 40] = [
    b"\x10\x00",
    b"\x10\x01",
    b"\x10\x02",
    b"\x10\x03",
    b"\x10\x04",
    b"\x10\x05",
    b"\x10\x06",
    b"\x10\x07",
    b"\x10\x08",
    b"\x10\x09",
    b"\x10\x0A",
    b"\x10\x0B",
    b"\x10\x0C",
    b"\x10\x0D",
    b"\x10\x0E",
    b"\x10\x0F",
    b"\x20\x00",
    b"\x20\x01",
    b"\x20\x02",
    b"\x20\x03",
    b"\x20\x04",
    b"\x20\x05",
    b"\x20\x06",
    b"\x20\x07",
    b"\x20\x08",
    b"\x20\x09",
    b"\x20\x0A",
    b"\x20\x0B",
    b"\x20\x0C",
    b"\x20\x0D",
    b"\x20\x0E",
    b"\x20\x0F",
    b"\x30\x00\xFF",
    b"\x30\x01\xFE",
    b"\x30\x02\xFD",
    b"\x30\x03\xFC",
    b"\x40\x00\xFF\xE0",
    b"\x40\x01\xFE\xE1",
    b"\x40\x02\xFD\xE2",
    b"\x40\x03\xFC\xE3",
];

const MISS_SEQUENCES: [&[u8]; 8] = [
    b"",
    b"\xFF",
    b"\xA1",
    b"\xA1\xB3",
    b"\xA1\xB2\x00",
    b"\xA1\xB2\xC5",
    b"\xA0\xB1\x01",
    b"\xFF\xFF\xFF\xFF",
];

const MIXED_WORKLOAD: [&[u8]; 12] = [
    b"\xA1\xB2",
    b"\xA1\xB2\xC3",
    b"\xA1\xB2\xC5",
    b"\xA0\xB1",
    b"\xA2\xB0",
    b"\xA1\xB3",
    b"\xA1\xB2\xC4",
    b"\xA1\xB2\x00",
    b"\x00",
    b"\xA1",
    b"\xA1\xB2\xC3\xD0",
    b"",
];

fn naive_contains(set: &[&[u8]], candidate: &[u8]) -> bool {
    for seq in set {
        if *seq == candidate {
            return true;
        }
    }
    false
}

fn byte_set_benchmarks() -> impl IntoBenchmarks {
    [
        benchmark_fn("byte_set_hot_hit", |b| {
            b.iter(|| black_box(HOT_SET.contains(black_box(b"\xA1\xB2"))))
        }),
        benchmark_fn("byte_set_cold_hit", |b| {
            b.iter(|| black_box(HOT_SET.contains(black_box(b"\xA0\xB1"))))
        }),
        benchmark_fn("byte_set_short_miss", |b| {
            b.iter(|| black_box(HOT_SET.contains(black_box(b"\xFF"))))
        }),
        benchmark_fn("byte_set_long_miss", |b| {
            b.iter(|| black_box(HOT_SET.contains(black_box(b"\xA1\xB2\xFF"))))
        }),
        benchmark_fn("byte_set_mixed_workload", |b| {
            b.iter(|| {
                let mut hits = 0usize;
                for candidate in MIXED_WORKLOAD {
                    if HOT_SET.contains(black_box(candidate)) {
                        hits += 1;
                    }
                }
                black_box(hits)
            })
        }),
        benchmark_fn("naive_mixed_workload", |b| {
            b.iter(|| {
                let mut hits = 0usize;
                for candidate in MIXED_WORKLOAD {
                    if naive_contains(black_box(&HOT_SEQUENCES), black_box(candidate)) {
                        hits += 1;
                    }
                }
                black_box(hits)
            })
        }),
        benchmark_fn("byte_set_large_dense_hits", |b| {
            b.iter(|| {
                let mut hits = 0usize;
                for candidate in LARGE_SEQUENCES {
                    if LARGE_SET.contains(black_box(candidate)) {
                        hits += 1;
                    }
                }
                black_box(hits)
            })
        }),
        benchmark_fn("naive_large_dense_hits", |b| {
            b.iter(|| {
                let mut hits = 0usize;
                for candidate in LARGE_SEQUENCES {
                    if naive_contains(black_box(&LARGE_SEQUENCES), black_box(candidate)) {
                        hits += 1;
                    }
                }
                black_box(hits)
            })
        }),
        benchmark_fn("byte_set_large_pathological_miss", |b| {
            b.iter(|| black_box(LARGE_SET.contains(black_box(b"\xFF\xFF\xFF\xFF"))))
        }),
        benchmark_fn("byte_set_randomized_mix", |b| {
            let seed = b.seed;
            b.iter(move || {
                let mut rng = StdRng::seed_from_u64(seed ^ 0xBADC0FFE);
                let mut hits = 0usize;
                let hit_len = HOT_SEQUENCES.len();
                let miss_len = MISS_SEQUENCES.len();
                for _ in 0..512 {
                    let candidate = if rng.gen_bool(0.7) {
                        HOT_SEQUENCES[rng.gen_range(0..hit_len)]
                    } else {
                        MISS_SEQUENCES[rng.gen_range(0..miss_len)]
                    };
                    if HOT_SET.contains(black_box(candidate)) {
                        hits += 1;
                    }
                }
                black_box(hits)
            })
        }),
    ]
}

tango_benchmarks!(byte_set_benchmarks());
tango_main!();
