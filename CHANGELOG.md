#### v0.3.0

- The API version has been increased to 3.

- `LedgerResult` is no longer represented as `u8`, but is now an `i32`. `LedgerResult::OkTrue = 1`, `LedgerResult::OkFalse = 0`, and `LedgerResult::Err = -1`.

- `jlrs_ledger_try_borrow_shared` and `jlrs_ledger_try_borrow_exclusive` return `LedgerResult::OkFalse` instead of `LedgerResult::Err` if the pointer can't be borrowed due to existing borrows.

- `jlrs_ledger_borrow_shared_unchecked` has been removed.

- `jlrs_ledger_n_shared_borrows` has been added, it returns the number of existing shared borrows of the pointer.

- The ledger is no longer corrupted if a pointer is untracked with the incorrect unborrow function, the operation simply fails with `LedgerResult::Err`.

- The `Ledger` struct has been removed in favor of manipulating the backing `HashMap` directly.

#### v0.2.0

- API version 2: `jlrs_ledger_init` must be called before the ledger is used.

- Access to the ledger is protected by a `TicketLock` from spin instead of a `Mutex` from the standard library.

- The ledger uses a `FxHashMap` from rustc-hash to track active borrows instead of two `Vec`s.
