use once_cell::sync::OnceCell;
use rustc_hash::FxHashMap;
use spin::Mutex;
use std::{collections::hash_map::Entry, ffi::c_void, num::NonZeroUsize};

#[cfg(all(target_arch = "x86", target_os = "windows", target_env = "gnu"))]
#[allow(unused)]
#[no_mangle]
unsafe extern "C" fn _Unwind_Resume() {}

/// The current API version.
#[no_mangle]
pub static JLRS_LEDGER_API_VERSION: usize = 3;

const EXCLUSIVE_BORROW_MASK: usize = 1;
const EXCLUSIVE_BORROW: NonZeroUsize =
    unsafe { NonZeroUsize::new_unchecked(EXCLUSIVE_BORROW_MASK) };
const SHARED_BORROW_MASK: usize = 2;
const SHARED_BORROW: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(SHARED_BORROW_MASK) };

static LEDGER: OnceCell<Mutex<FxHashMap<usize, NonZeroUsize>>> = OnceCell::new();

/// Indicates whether an operation succeeded, failed, or incorrect.
#[derive(PartialEq, Eq, Debug)]
#[repr(i32)]
pub enum LedgerResult {
    /// Operation succeeded and the result is `false`, or the operation failed.
    OkFalse = 0,
    /// Operation succeeded, result is `true`.
    OkTrue = 1,
    /// Operation is incorrect given the state of the ledger.
    Err = -1,
}

/// Return the API version.
#[no_mangle]
pub unsafe extern "C" fn jlrs_ledger_api_version() -> usize {
    JLRS_LEDGER_API_VERSION
}

/// Initialize the ledger.
///
/// This function must be called before calling any other functions from the C API of this
/// crate, except [`jlrs_ledger_api_version`].
#[no_mangle]
pub extern "C" fn jlrs_ledger_init() {
    LEDGER.get_or_init(|| Mutex::new(Default::default()));
}

/// Check if the given pointer is tracked as a shared borrow.
///
/// Returns `LedgerResult::OkTrue` if it is borrowed sharedly, or `LedgerResult::OkFalse` if
/// it isn't.
///
/// Safety:
///
/// `jlrs_ledger_init` must have been called before calling this function.
#[no_mangle]
pub unsafe extern "C" fn jlrs_ledger_is_borrowed_shared(ptr: *const c_void) -> LedgerResult {
    match LEDGER.get_unchecked().lock().get(&(ptr as usize)).copied() {
        None => LedgerResult::OkFalse,
        Some(s) if s != EXCLUSIVE_BORROW => LedgerResult::OkTrue,
        Some(_) => LedgerResult::OkFalse,
    }
}

/// Check if the given pointer is tracked as an exclusive borrow.
///
/// Returns `LedgerResult::OkTrue` if it is borrowed exclusively, or `LedgerResult::OkFalse`
/// if it isn't.
///
/// Safety:
///
/// `jlrs_ledger_init` must have been called before calling this function.
#[no_mangle]
pub unsafe extern "C" fn jlrs_ledger_is_borrowed_exclusive(ptr: *const c_void) -> LedgerResult {
    match LEDGER.get_unchecked().lock().get(&(ptr as usize)).copied() {
        None => LedgerResult::OkFalse,
        Some(s) if s == EXCLUSIVE_BORROW => LedgerResult::OkTrue,
        Some(_) => LedgerResult::OkFalse,
    }
}

/// Check if the given pointer is tracked in some way.
///
/// Returns `LedgerResult::OkTrue` if it is borrowed, or `LedgerResult::OkFalse`
/// if it isn't.
///
/// Safety:
///
/// `jlrs_ledger_init` must have been called before calling this function.
#[no_mangle]
pub unsafe extern "C" fn jlrs_ledger_is_borrowed(ptr: *const c_void) -> LedgerResult {
    if LEDGER.get_unchecked().lock().contains_key(&(ptr as usize)) {
        LedgerResult::OkTrue
    } else {
        LedgerResult::OkFalse
    }
}

/// The number of active shared borrows of `addr`.
///
/// Safety:
///
/// `jlrs_ledger_init` must have been called before calling this function.
#[no_mangle]
pub unsafe extern "C" fn jlrs_ledger_n_shared_borrows(ptr: *const c_void) -> usize {
    match LEDGER.get_unchecked().lock().get(&(ptr as usize)).copied() {
        Some(s) => s.get() >> 1,
        None => 0,
    }
}
/// Stop tracking the pointer as a shared borrow.
///
/// Returns `LedgerResult::OkTrue` if it's no longer tracked, `LedgerResult::OkFalse` if other
/// shared borrows still exist, and `LedgerResult::Err` if the pointer wasn't tracked or is
/// tracked as an exclusive borrow.
///
/// Safety:
///
/// `jlrs_ledger_init` must have been called before calling this function.
#[no_mangle]
pub unsafe extern "C" fn jlrs_ledger_unborrow_shared(ptr: *const c_void) -> LedgerResult {
    match LEDGER.get_unchecked().lock().entry(ptr as usize) {
        Entry::Occupied(mut o) => {
            let current = *o.get();
            if current == SHARED_BORROW {
                o.remove_entry();
                LedgerResult::OkTrue
            } else if current != EXCLUSIVE_BORROW {
                *o.get_mut() = NonZeroUsize::new_unchecked(current.get() - SHARED_BORROW_MASK);
                LedgerResult::OkFalse
            } else {
                LedgerResult::Err
            }
        }
        Entry::Vacant(_) => LedgerResult::Err,
    }
}

/// Stop tracking the pointer as an exclusive borrow.
///
/// Returns `LedgerResult::OkTrue` if it's no longer tracked, and `LedgerResult::Err` if the
/// pointer wasn't tracked or was tracked as a shared borrow.
///
/// Safety:
///
/// `jlrs_ledger_init` must have been called before calling this function.
#[no_mangle]
pub unsafe extern "C" fn jlrs_ledger_unborrow_exclusive(ptr: *const c_void) -> LedgerResult {
    let mut ledger = LEDGER.get_unchecked().lock();
    match ledger.remove(&(ptr as usize)) {
        Some(s) if s == EXCLUSIVE_BORROW => LedgerResult::OkTrue,
        Some(s) => {
            ledger.insert(ptr as _, s);
            LedgerResult::Err
        }
        _ => LedgerResult::Err,
    }
}

/// Try to track the pointer as a shared borrow.
///
/// Returns `LedgerResult::OkTrue` on success, or `LedgerResult:OkFalse` if it is already
/// tracked as an exclusive borrow.
///
/// Safety:
///
/// `jlrs_ledger_init` must have been called before calling this function.
#[no_mangle]
pub unsafe extern "C" fn jlrs_ledger_try_borrow_shared(ptr: *const c_void) -> LedgerResult {
    match LEDGER.get_unchecked().lock().entry(ptr as usize) {
        Entry::Vacant(v) => {
            v.insert(SHARED_BORROW);
            LedgerResult::OkTrue
        }
        Entry::Occupied(ref mut o) => {
            let current = *o.get();
            if current != EXCLUSIVE_BORROW {
                *o.get_mut() = NonZeroUsize::new_unchecked(current.get() + SHARED_BORROW_MASK);
                LedgerResult::OkTrue
            } else {
                LedgerResult::OkFalse
            }
        }
    }
}
/// Try to track the pointer as an exclusive borrow.
///
/// Returns `LedgerResult::Ok` on success, or `LedgerResult:OkFalse` if it is already
/// tracked.
///
/// Safety:
///
/// `jlrs_ledger_init` must have been called before calling this function.
#[no_mangle]
pub unsafe extern "C" fn jlrs_ledger_try_borrow_exclusive(ptr: *const c_void) -> LedgerResult {
    match LEDGER.get_unchecked().lock().entry(ptr as usize) {
        Entry::Vacant(v) => {
            v.insert(EXCLUSIVE_BORROW);
            LedgerResult::OkTrue
        }
        Entry::Occupied(_) => LedgerResult::OkFalse,
    }
}

/// Clear the ledger.
///
/// Don't call this function, it exists purely for testing and benchmarking purposes. It's not
/// part of the C API of this crate.
///
/// Safety:
///
/// `jlrs_ledger_init` must have been called before calling this function.
#[doc(hidden)]
pub unsafe fn clear_ledger() {
    LEDGER.get_unchecked().lock().clear();
}

#[cfg(test)]
mod tests {
    use std::ptr::dangling;

    use super::*;

    #[test]
    fn api_version() {
        unsafe {
            assert_eq!(JLRS_LEDGER_API_VERSION, jlrs_ledger_api_version());
        }
    }

    #[test]
    fn borrow_shared() {
        unsafe {
            jlrs_ledger_init();
            clear_ledger();

            let ptr = dangling();
            assert_eq!(jlrs_ledger_is_borrowed_shared(ptr), LedgerResult::OkFalse);
            assert_eq!(jlrs_ledger_n_shared_borrows(ptr), 0);
            assert_eq!(
                jlrs_ledger_is_borrowed_exclusive(ptr),
                LedgerResult::OkFalse
            );
            assert_eq!(jlrs_ledger_is_borrowed(ptr), LedgerResult::OkFalse);
            assert_eq!(jlrs_ledger_try_borrow_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_is_borrowed_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_n_shared_borrows(ptr), 1);
            assert_eq!(
                jlrs_ledger_is_borrowed_exclusive(ptr),
                LedgerResult::OkFalse
            );
            assert_eq!(jlrs_ledger_is_borrowed(ptr), LedgerResult::OkTrue);
        }
    }

    #[test]
    fn borrow_exclusive() {
        unsafe {
            jlrs_ledger_init();
            clear_ledger();

            let ptr = dangling();
            assert_eq!(jlrs_ledger_is_borrowed_shared(ptr), LedgerResult::OkFalse);
            assert_eq!(
                jlrs_ledger_is_borrowed_exclusive(ptr),
                LedgerResult::OkFalse
            );
            assert_eq!(jlrs_ledger_is_borrowed(ptr), LedgerResult::OkFalse);
            assert_eq!(jlrs_ledger_try_borrow_exclusive(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_is_borrowed_shared(ptr), LedgerResult::OkFalse);
            assert_eq!(jlrs_ledger_n_shared_borrows(ptr), 0);
            assert_eq!(jlrs_ledger_is_borrowed_exclusive(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_is_borrowed(ptr), LedgerResult::OkTrue);
        }
    }

    #[test]
    fn borrow_exclusive_then_shared() {
        unsafe {
            jlrs_ledger_init();
            clear_ledger();

            let ptr = dangling();
            assert_eq!(jlrs_ledger_try_borrow_exclusive(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_try_borrow_shared(ptr), LedgerResult::OkFalse);
            assert_eq!(jlrs_ledger_n_shared_borrows(ptr), 0);
        }
    }

    #[test]
    fn borrow_shared_then_exclusive() {
        unsafe {
            jlrs_ledger_init();
            clear_ledger();

            let ptr = dangling();
            assert_eq!(jlrs_ledger_try_borrow_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_try_borrow_exclusive(ptr), LedgerResult::OkFalse);
            assert_eq!(jlrs_ledger_n_shared_borrows(ptr), 1);
        }
    }

    #[test]
    fn borrow_shared_twice() {
        unsafe {
            jlrs_ledger_init();
            clear_ledger();

            let ptr = dangling();
            assert_eq!(jlrs_ledger_try_borrow_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_try_borrow_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_n_shared_borrows(ptr), 2);
        }
    }

    #[test]
    fn unborrow_shared() {
        unsafe {
            jlrs_ledger_init();
            clear_ledger();

            let ptr = dangling();
            assert_eq!(jlrs_ledger_try_borrow_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_unborrow_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_is_borrowed_shared(ptr), LedgerResult::OkFalse);
            assert_eq!(jlrs_ledger_n_shared_borrows(ptr), 0);
            assert_eq!(
                jlrs_ledger_is_borrowed_exclusive(ptr),
                LedgerResult::OkFalse
            );
            assert_eq!(jlrs_ledger_is_borrowed(ptr), LedgerResult::OkFalse);
        }
    }

    #[test]
    fn unborrow_exclusive() {
        unsafe {
            jlrs_ledger_init();
            clear_ledger();

            let ptr = dangling();
            assert_eq!(jlrs_ledger_try_borrow_exclusive(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_unborrow_exclusive(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_is_borrowed_shared(ptr), LedgerResult::OkFalse);
            assert_eq!(
                jlrs_ledger_is_borrowed_exclusive(ptr),
                LedgerResult::OkFalse
            );
            assert_eq!(jlrs_ledger_is_borrowed(ptr), LedgerResult::OkFalse);
        }
    }

    #[test]
    fn unborrow_shared_twice() {
        unsafe {
            jlrs_ledger_init();
            clear_ledger();

            let ptr = dangling();
            assert_eq!(jlrs_ledger_try_borrow_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_try_borrow_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_unborrow_shared(ptr), LedgerResult::OkFalse);
            assert_eq!(jlrs_ledger_n_shared_borrows(ptr), 1);
            assert_eq!(jlrs_ledger_is_borrowed_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(
                jlrs_ledger_is_borrowed_exclusive(ptr),
                LedgerResult::OkFalse
            );
            assert_eq!(jlrs_ledger_is_borrowed(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_unborrow_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_is_borrowed_shared(ptr), LedgerResult::OkFalse);
            assert_eq!(jlrs_ledger_n_shared_borrows(ptr), 0);
            assert_eq!(
                jlrs_ledger_is_borrowed_exclusive(ptr),
                LedgerResult::OkFalse
            );
            assert_eq!(jlrs_ledger_is_borrowed(ptr), LedgerResult::OkFalse);
        }
    }

    #[test]
    fn unborrow_unborrowed_shared() {
        unsafe {
            jlrs_ledger_init();
            clear_ledger();

            let ptr = dangling();
            assert_eq!(jlrs_ledger_try_borrow_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_unborrow_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_unborrow_shared(ptr), LedgerResult::Err);
        }
    }

    #[test]
    fn unborrow_unborrowed_exclusive() {
        unsafe {
            jlrs_ledger_init();
            clear_ledger();

            let ptr = dangling();
            assert_eq!(jlrs_ledger_try_borrow_exclusive(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_unborrow_exclusive(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_unborrow_exclusive(ptr), LedgerResult::Err);
        }
    }
}
