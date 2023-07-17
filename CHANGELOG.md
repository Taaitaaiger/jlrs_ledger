#### v0.2.0

- API version 2: `jlrs_ledger_init` must be called before the ledger is used.

- Access to the ledger is protected by a `TicketLock` from spin instead of a `Mutex` from the standard library. 

- The ledger uses a `FxHashMap` from rustc-hash to track active borrows instead of two `Vec`s.
