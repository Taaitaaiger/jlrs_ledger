use criterion::{black_box, criterion_group, criterion_main, Criterion};
use jlrs_ledger::*;
use std::{
    hint::spin_loop,
    ptr::dangling,
    sync::{Arc, Barrier},
    time::{Duration, Instant},
};

fn delay(duration: Duration) {
    let now = Instant::now();
    while now.elapsed() < duration {
        spin_loop();
    }
}

fn bench(c: &mut Criterion, n_threads: usize, max_threads: usize) {
    let name = format!("Contention Baseline {n_threads}");

    c.bench_function(&name, |b| {
        b.iter(|| {
            let barrier = Arc::new(Barrier::new(max_threads));
            let mut handles = Vec::with_capacity(max_threads);

            for _ in 0..n_threads {
                let b = barrier.clone();
                handles.push(std::thread::spawn(move || {
                    b.wait();
                    for _ in 0..10000 {
                        black_box(delay(Duration::from_nanos(10)));
                    }
                }));
            }

            for _ in 0..max_threads - n_threads {
                let b = barrier.clone();
                handles.push(std::thread::spawn(move || {
                    b.wait();
                }));
            }

            for handle in handles {
                handle.join().unwrap();
            }
        })
    });

    let name = format!("Contention {n_threads}");
    c.bench_function(&name, |b| {
        b.iter(|| {
            let barrier = Arc::new(Barrier::new(max_threads));
            let mut handles = Vec::with_capacity(max_threads);

            for _ in 0..n_threads {
                let b = barrier.clone();
                handles.push(std::thread::spawn(move || {
                    b.wait();
                    let ptr = dangling();
                    for _ in 0..10000 {
                        unsafe {
                            let a = jlrs_ledger_try_borrow_shared(black_box(ptr));
                            black_box(delay(Duration::from_nanos(10)));
                            black_box(a);
                        }
                    }
                }));
            }

            for _ in 0..max_threads - n_threads {
                let b = barrier.clone();
                handles.push(std::thread::spawn(move || {
                    b.wait();
                }));
            }

            for handle in handles {
                handle.join().unwrap();
            }
        })
    });
}

fn benches(c: &mut Criterion) {
    jlrs_ledger_init();

    bench(c, 1, 8);
    unsafe { clear_ledger() };
    bench(c, 2, 8);
    unsafe { clear_ledger() };
    bench(c, 4, 8);
    unsafe { clear_ledger() };
    bench(c, 8, 8);
}

criterion_group! {
    name = contention;
    config = Criterion::default();
    targets = benches
}

criterion_main!(contention);
