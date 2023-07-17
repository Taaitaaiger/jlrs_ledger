use criterion::{criterion_group, criterion_main, Criterion};
use jlrs_ledger::Ledger;
use pprof::{
    criterion::{Output, PProfProfiler},
    flamegraph::Options,
};
use std::ptr::null;

// Thanks to the example provided by @jebbow in his article
// https://www.jibbow.com/posts/criterion-flamegraphs/

fn benches(c: &mut Criterion) {
    Ledger::init();

    c.bench_function("Track shared", |b| {
        b.iter(|| unsafe { Ledger::try_borrow_shared(null()) })
    });

    unsafe {
        Ledger::clear();
    }

    c.bench_function("Is tracked shared false", |b| {
        b.iter(|| unsafe { Ledger::is_borrowed_shared(null()) })
    });

    unsafe {
        Ledger::clear();
    }

    c.bench_function("Is tracked exclusive false", |b| {
        b.iter(|| unsafe { Ledger::is_borrowed_exclusive(null()) })
    });

    unsafe {
        Ledger::clear();
    }

    c.bench_function("Is tracked false", |b| {
        b.iter(|| unsafe { Ledger::is_borrowed(null()) })
    });

    unsafe {
        Ledger::clear();
        Ledger::borrow_shared_unchecked(null());
    }

    c.bench_function("Is tracked shared true", |b| {
        b.iter(|| unsafe { Ledger::is_borrowed_shared(null()) })
    });

    c.bench_function("Is tracked true", |b| {
        b.iter(|| unsafe { Ledger::is_borrowed(null()) })
    });

    unsafe {
        Ledger::clear();
        Ledger::try_borrow_exclusive(null());
    }

    c.bench_function("Is tracked exclusive true", |b| {
        b.iter(|| unsafe { Ledger::is_borrowed_exclusive(null()) })
    });

    unsafe {
        Ledger::clear();
    }

    c.bench_function("Track and untrack shared", |b| {
        b.iter(|| unsafe {
            Ledger::try_borrow_shared(null());
            Ledger::unborrow_shared(null());
        })
    });

    unsafe {
        Ledger::clear();
    }

    c.bench_function("Track and untrack exclusive", |b| {
        b.iter(|| unsafe {
            Ledger::try_borrow_exclusive(null());
            Ledger::unborrow_exclusive(null());
        })
    });
}

fn opts() -> Option<Options<'static>> {
    let mut opts = Options::default();
    opts.image_width = Some(1920);
    opts.min_width = 0.01;
    Some(opts)
}

criterion_group! {
    name = track;
    config = Criterion::default().with_profiler(PProfProfiler::new(1000000, Output::Flamegraph(opts())));
    targets = benches
}

criterion_main!(track);
