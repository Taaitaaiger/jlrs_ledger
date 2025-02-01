use criterion::{black_box, criterion_group, criterion_main, Criterion};
use jlrs_ledger::*;
use std::{ffi::c_void, ptr::dangling};

fn benches(c: &mut Criterion) {
    jlrs_ledger_init();

    let ptr = dangling::<u32>() as *const c_void;

    c.bench_function("Track shared", |b| {
        b.iter(|| unsafe {
            let a = jlrs_ledger_try_borrow_shared(black_box(ptr));
            black_box(a)
        })
    });

    unsafe {
        clear_ledger();
    }

    c.bench_function("Is tracked shared false", |b| {
        b.iter(|| unsafe {
            let a = jlrs_ledger_is_borrowed_shared(black_box(ptr));
            black_box(a)
        })
    });

    c.bench_function("Is tracked exclusive false", |b| {
        b.iter(|| unsafe {
            let a = jlrs_ledger_is_borrowed_exclusive(black_box(ptr));
            black_box(a)
        })
    });

    c.bench_function("Is tracked false", |b| {
        b.iter(|| unsafe {
            let a = jlrs_ledger_is_borrowed(black_box(ptr));
            black_box(a)
        })
    });

    unsafe {
        jlrs_ledger_try_borrow_shared(ptr);
    }

    c.bench_function("Is tracked shared true", |b| {
        b.iter(|| unsafe {
            let a = jlrs_ledger_is_borrowed_shared(black_box(ptr));
            black_box(a)
        })
    });

    c.bench_function("Is tracked true", |b| {
        b.iter(|| unsafe {
            let a = jlrs_ledger_is_borrowed(black_box(ptr));
            black_box(a)
        })
    });

    unsafe {
        clear_ledger();
        jlrs_ledger_try_borrow_exclusive(ptr);
    }

    c.bench_function("Is tracked exclusive true", |b| {
        b.iter(|| unsafe {
            let a = jlrs_ledger_is_borrowed_exclusive(black_box(ptr));
            black_box(a)
        })
    });

    unsafe {
        clear_ledger();
    }

    c.bench_function("Track and untrack shared", |b| {
        b.iter(|| unsafe {
            let a = jlrs_ledger_try_borrow_shared(black_box(ptr));
            let b = jlrs_ledger_unborrow_shared(black_box(ptr));
            (black_box(a), black_box(b))
        })
    });

    unsafe {
        clear_ledger();
        jlrs_ledger_try_borrow_shared(black_box(ptr));
    }

    c.bench_function("Track and untrack shared second time", |b| {
        b.iter(|| unsafe {
            let a = jlrs_ledger_try_borrow_shared(black_box(ptr));
            let b = jlrs_ledger_unborrow_shared(black_box(ptr));
            (black_box(a), black_box(b))
        })
    });

    unsafe {
        clear_ledger();
    }

    c.bench_function("Track and untrack exclusive", |b| {
        b.iter(|| unsafe {
            let a = jlrs_ledger_try_borrow_exclusive(black_box(ptr));
            let b = jlrs_ledger_unborrow_exclusive(black_box(ptr));
            (black_box(a), black_box(b))
        })
    });
}

criterion_group! {
    name = track;
    config = Criterion::default();
    targets = benches
}

criterion_main!(track);
