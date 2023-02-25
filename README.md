# jlrs-ledger

This crate implements the ledger used by jlrs to track Julia data that is borrowed in Rust.

You should never use of this crate directly, it's only intended to be used internally by jlrs. It's distributed as a separate library to allow multiple Julia packages that use jlrs to expose functionality implemented in Rust to share the same ledger, even if different versions of jlrs are used or different versions of Rust have been used to compile the exported Rust code.

