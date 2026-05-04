use criterion::{Criterion, criterion_group, criterion_main};
use jlrs_ledger::*;
use std::{ffi::c_void, hint::black_box, ptr::dangling};

fn benches(c: &mut Criterion) {
    unsafe { jlrs_ledger_init() };

    let ptr = dangling::<u32>() as *const c_void;

    c.bench_function("Track shared", |b| {
        b.iter(|| unsafe {
            black_box(jlrs_ledger_try_borrow_shared(black_box(ptr)));
        })
    });

    unsafe {
        clear_ledger();
    }

    c.bench_function("Is tracked shared false", |b| {
        b.iter(|| unsafe {
            black_box(jlrs_ledger_is_borrowed_shared(black_box(ptr)));
        })
    });

    c.bench_function("Is tracked exclusive false", |b| {
        b.iter(|| unsafe {
            black_box(jlrs_ledger_is_borrowed_exclusive(black_box(ptr)));
        })
    });

    c.bench_function("Is tracked false", |b| {
        b.iter(|| unsafe {
            black_box(jlrs_ledger_is_borrowed(black_box(ptr)));
        })
    });

    unsafe {
        jlrs_ledger_try_borrow_shared(ptr);
    }

    c.bench_function("Is tracked shared true", |b| {
        b.iter(|| unsafe {
            black_box(jlrs_ledger_is_borrowed_shared(black_box(ptr)));
        })
    });

    c.bench_function("Is tracked true", |b| {
        b.iter(|| unsafe {
            black_box(jlrs_ledger_is_borrowed(black_box(ptr)));
        })
    });

    unsafe {
        clear_ledger();
        jlrs_ledger_try_borrow_exclusive(ptr);
    }

    c.bench_function("Is tracked exclusive true", |b| {
        b.iter(|| unsafe {
            black_box(jlrs_ledger_is_borrowed_exclusive(black_box(ptr)));
        })
    });

    unsafe {
        clear_ledger();
    }

    c.bench_function("Track and untrack shared", |b| {
        b.iter(|| unsafe {
            black_box(jlrs_ledger_try_borrow_shared(black_box(ptr)));
            black_box(jlrs_ledger_unborrow_shared(black_box(ptr)));
        })
    });

    unsafe {
        clear_ledger();
        jlrs_ledger_try_borrow_shared(black_box(ptr));
    }

    c.bench_function("Track and untrack shared second time", |b| {
        b.iter(|| unsafe {
            black_box(jlrs_ledger_try_borrow_shared(black_box(ptr)));
            black_box(jlrs_ledger_unborrow_shared(black_box(ptr)));
        })
    });

    unsafe {
        clear_ledger();
    }

    c.bench_function("Track and untrack exclusive", |b| {
        b.iter(|| unsafe {
            black_box(jlrs_ledger_try_borrow_exclusive(black_box(ptr)));
            black_box(jlrs_ledger_unborrow_exclusive(black_box(ptr)));
        })
    });
}

criterion_group! {
    name = track;
    config = Criterion::default();
    targets = benches
}

criterion_main!(track);
