use once_cell::sync::OnceCell;
use rustc_hash::FxHashMap;
use spin::Mutex;
use std::{collections::hash_map::Entry, ffi::c_void};

#[cfg(all(target_arch = "x86", target_os = "windows", target_env = "gnu"))]
#[allow(unused)]
#[no_mangle]
unsafe extern "C" fn _Unwind_Resume() {}

static LEDGER: OnceCell<Mutex<Ledger>> = OnceCell::new();

/// The current API version.
#[no_mangle]
pub static JLRS_LEDGER_API_VERSION: usize = 2;

/// Indicates whether an operation succeeded or failed.
#[derive(PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum LedgerResult {
    /// Operation succeeded, result is `false`.
    OkFalse = 0u8,
    /// Operation succeeded, result is `true`.
    OkTrue = 1u8,
    /// Operation failed.
    Err = 2u8,
}

/// Tracker for Julia data that is borrowed in Rust code.
pub struct Ledger {
    state: FxHashMap<usize, usize>,
}

impl Ledger {
    #[inline(always)]
    fn new() -> Self {
        Ledger {
            state: Default::default(),
        }
    }

    /// Initialize the ledger. Does nothing if the ledger has already been initialized.
    #[inline(always)]
    pub fn init() {
        LEDGER.get_or_init(|| Mutex::new(Ledger::new()));
    }

    /// Check if the given pointer is tracked as a shared borrow.
    ///
    /// Returns `LedgerResult::OkTrue` if it is borrowed sharedly, or `LedgerResult::OkFalse` if
    /// it isn't.
    ///
    /// Safety:
    ///
    /// `Ledger::init` must have been called before calling this function.
    #[inline(always)]
    pub unsafe fn is_borrowed_shared(ptr: *const c_void) -> LedgerResult {
        let ledger = unsafe { LEDGER.get_unchecked().lock() };
        let borrowed_shared = if let Some(state) = ledger.state.get(&(ptr as usize)) {
            state != &1
        } else {
            false
        };

        if borrowed_shared {
            return LedgerResult::OkTrue;
        } else {
            return LedgerResult::OkFalse;
        }
    }

    /// Check if the given pointer is tracked as an exclusive borrow.
    ///
    /// Returns `LedgerResult::OkTrue` if it is borrowed exclusively, or `LedgerResult::OkFalse`
    /// if it isn't.
    ///
    /// Safety:
    ///
    /// `Ledger::init` must have been called before calling this function.
    #[inline(always)]
    pub unsafe fn is_borrowed_exclusive(ptr: *const c_void) -> LedgerResult {
        let ledger = unsafe { LEDGER.get_unchecked().lock() };
        let borrowed_exclusive = if let Some(state) = ledger.state.get(&(ptr as usize)) {
            state == &1
        } else {
            false
        };

        if borrowed_exclusive {
            return LedgerResult::OkTrue;
        } else {
            return LedgerResult::OkFalse;
        }
    }

    /// Check if the given pointer is tracked in some way.
    ///
    /// Returns `LedgerResult::OkTrue` if it is borrowed, or `LedgerResult::OkFalse`
    /// if it isn't.
    ///
    /// Safety:
    ///
    /// `Ledger::init` must have been called before calling this function.
    #[inline(always)]
    pub unsafe fn is_borrowed(ptr: *const c_void) -> LedgerResult {
        let ledger = unsafe { LEDGER.get_unchecked().lock() };

        if ledger.state.contains_key(&(ptr as usize)) {
            return LedgerResult::OkTrue;
        } else {
            return LedgerResult::OkFalse;
        }
    }

    /// Try to track the pointer as a shared borrow.
    ///
    /// Returns `LedgerResult::Ok` on success, or `LedgerResult:Err` if an exclusive borrow
    /// already exists.
    ///
    /// Safety:
    ///
    /// `Ledger::init` must have been called before calling this function.
    #[inline(always)]
    pub unsafe fn try_borrow_shared(ptr: *const c_void) -> LedgerResult {
        LEDGER.get_unchecked().lock().try_add_borrow_shared(ptr)
    }

    /// Try to track the pointer as an exclusive borrow.
    ///
    /// Returns `LedgerResult::Ok` on success, or `LedgerResult:Err` if an exclusive borrow
    /// already exists.
    ///
    /// Safety:
    ///
    /// `Ledger::init` must have been called before calling this function.
    #[inline(always)]
    pub unsafe fn try_borrow_exclusive(ptr: *const c_void) -> LedgerResult {
        LEDGER.get_unchecked().lock().try_add_borrow_exclusive(ptr)
    }

    /// Track the pointer as a shared borrow without checking if it is already borrowed.
    ///
    /// Returns `LedgerResult::Ok`.
    ///
    /// Safety:
    ///
    /// `Ledger::init` must have been called before calling this function. The pointer must not
    /// already be tracked as an exclusive borrow.
    #[inline(always)]
    pub unsafe fn borrow_shared_unchecked(ptr: *const c_void) -> LedgerResult {
        LEDGER
            .get_unchecked()
            .lock()
            .state
            .entry(ptr as usize)
            .and_modify(|i| *i += 2)
            .or_insert(2);
        LedgerResult::OkTrue
    }

    /// Stop tracking the pointer as a shared borrow.
    ///
    /// Returns `LedgerResult::OkTrue` if it's no longer tracked, `LedgerResult::OkFalse` if other
    /// shared borrows still exist, and `LedgerResult::Err` if the pointer wasn't tracked.
    ///
    /// Safety:
    ///
    /// `Ledger::init` must have been called before calling this function. The pointer must
    /// have been tracked as a shared borrow. A `LedgerResult::Err` being returned by this
    /// function implies that the ledger has been corrupted.
    #[inline(always)]
    pub unsafe fn unborrow_shared(ptr: *const c_void) -> LedgerResult {
        let mut ledger = LEDGER.get_unchecked().lock();
        let entry = ledger.state.entry(ptr as usize);

        match entry {
            Entry::Occupied(mut o) => {
                if o.get() == &2 {
                    o.remove_entry();
                    LedgerResult::OkTrue
                } else {
                    *o.get_mut() -= 2;
                    LedgerResult::OkFalse
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
    /// `Ledger::init` must have been called before calling this function. The pointer must
    /// have been tracked as an exclusive borrow. A `LedgerResult::Err` being returned by this
    /// function implies that the ledger has been corrupted.
    #[inline(always)]
    pub unsafe fn unborrow_exclusive(ptr: *const c_void) -> LedgerResult {
        let mut ledger = LEDGER.get_unchecked().lock();
        if let Some(v) = ledger.state.remove(&(ptr as usize)) {
            if v == 1 {
                LedgerResult::OkTrue
            } else {
                LedgerResult::Err
            }
        } else {
            LedgerResult::Err
        }
    }

    /// Clear the entire ledger.
    ///
    /// Don't call this function. It only exists to clear the ledger between benchmarks and tests.
    #[doc(hidden)]
    #[inline(always)]
    pub unsafe fn clear() {
        let mut ledger = LEDGER.get_unchecked().lock();
        ledger.state.clear();
    }

    #[inline(always)]
    // Try to add an immutable borrow to the ledger
    unsafe fn try_add_borrow_shared(&mut self, ptr: *const c_void) -> LedgerResult {
        // Check if the data is already borrowed
        let entry = self.state.entry(ptr as usize);
        match &entry {
            Entry::Occupied(o) => {
                if o.get() == &1 {
                    return LedgerResult::Err;
                }
            }
            _ => (),
        }

        // Push the pointer to indicate it's borrowed
        entry.and_modify(|i| *i += 2).or_insert(2);
        LedgerResult::OkTrue
    }

    #[inline(always)]
    // Try to add a mutable borrow to the ledger
    unsafe fn try_add_borrow_exclusive(&mut self, ptr: *const c_void) -> LedgerResult {
        // Check if the borrow overlaps with any active mutable borrow
        let entry = self.state.entry(ptr as usize);
        match &entry {
            Entry::Occupied(_) => LedgerResult::Err,
            _ => {
                entry.or_insert(1);
                LedgerResult::OkTrue
            }
        }
    }
}

/* C-API */

/// Return the API version.
#[no_mangle]
pub unsafe extern "C" fn jlrs_ledger_api_version() -> usize {
    JLRS_LEDGER_API_VERSION
}

/// Initialize the ledger.
///
/// This function must be called before calling any other functions from the C-API of this
/// library, except [`jlrs_ledger_api_version`].
#[no_mangle]
pub unsafe extern "C" fn jlrs_ledger_init() {
    Ledger::init();
}

/// Call [`Ledger::is_borrowed_shared`].
#[no_mangle]
pub unsafe extern "C" fn jlrs_ledger_is_borrowed_shared(ptr: *const c_void) -> LedgerResult {
    Ledger::is_borrowed_shared(ptr)
}

/// Call [`Ledger::is_borrowed_exclusive`].
#[no_mangle]
pub unsafe extern "C" fn jlrs_ledger_is_borrowed_exclusive(ptr: *const c_void) -> LedgerResult {
    Ledger::is_borrowed_exclusive(ptr)
}

/// Call [`Ledger::is_borrowed`].
#[no_mangle]
pub unsafe extern "C" fn jlrs_ledger_is_borrowed(ptr: *const c_void) -> LedgerResult {
    Ledger::is_borrowed(ptr)
}

/// Call [`Ledger::borrow_shared_unchecked`].
#[no_mangle]
pub unsafe extern "C" fn jlrs_ledger_borrow_shared_unchecked(ptr: *const c_void) -> LedgerResult {
    Ledger::borrow_shared_unchecked(ptr)
}

/// Call [`Ledger::unborrow_shared`].
#[no_mangle]
pub unsafe extern "C" fn jlrs_ledger_unborrow_shared(ptr: *const c_void) -> LedgerResult {
    Ledger::unborrow_shared(ptr)
}

/// Call [`Ledger::unborrow_exclusive`].
#[no_mangle]
pub unsafe extern "C" fn jlrs_ledger_unborrow_exclusive(ptr: *const c_void) -> LedgerResult {
    Ledger::unborrow_exclusive(ptr)
}

/// Call [`Ledger::try_borrow_shared`].
#[no_mangle]
pub unsafe extern "C" fn jlrs_ledger_try_borrow_shared(ptr: *const c_void) -> LedgerResult {
    Ledger::try_borrow_shared(ptr)
}

/// Call [`Ledger::try_borrow_exclusive`].
#[no_mangle]
pub unsafe extern "C" fn jlrs_ledger_try_borrow_exclusive(ptr: *const c_void) -> LedgerResult {
    Ledger::try_borrow_exclusive(ptr)
}

#[cfg(test)]
mod tests {
    use std::ptr::NonNull;

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
            Ledger::init();
            Ledger::clear();

            let ptr = NonNull::<c_void>::dangling().as_ptr();
            assert_eq!(jlrs_ledger_is_borrowed_shared(ptr), LedgerResult::OkFalse);
            assert_eq!(
                jlrs_ledger_is_borrowed_exclusive(ptr),
                LedgerResult::OkFalse
            );
            assert_eq!(jlrs_ledger_is_borrowed(ptr), LedgerResult::OkFalse);
            assert_eq!(jlrs_ledger_try_borrow_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_is_borrowed_shared(ptr), LedgerResult::OkTrue);
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
            Ledger::init();
            Ledger::clear();

            let ptr = NonNull::<c_void>::dangling().as_ptr();
            assert_eq!(jlrs_ledger_is_borrowed_shared(ptr), LedgerResult::OkFalse);
            assert_eq!(
                jlrs_ledger_is_borrowed_exclusive(ptr),
                LedgerResult::OkFalse
            );
            assert_eq!(jlrs_ledger_is_borrowed(ptr), LedgerResult::OkFalse);
            assert_eq!(jlrs_ledger_try_borrow_exclusive(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_is_borrowed_shared(ptr), LedgerResult::OkFalse);
            assert_eq!(jlrs_ledger_is_borrowed_exclusive(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_is_borrowed(ptr), LedgerResult::OkTrue);
        }
    }

    #[test]
    fn borrow_exclusive_then_shared() {
        unsafe {
            Ledger::init();
            Ledger::clear();

            let ptr = NonNull::<c_void>::dangling().as_ptr();
            assert_eq!(jlrs_ledger_try_borrow_exclusive(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_try_borrow_shared(ptr), LedgerResult::Err);
        }
    }

    #[test]
    fn borrow_shared_then_exclusive() {
        unsafe {
            Ledger::init();
            Ledger::clear();

            let ptr = NonNull::<c_void>::dangling().as_ptr();
            assert_eq!(jlrs_ledger_try_borrow_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_try_borrow_exclusive(ptr), LedgerResult::Err);
        }
    }

    #[test]
    fn borrow_shared_twice() {
        unsafe {
            Ledger::init();
            Ledger::clear();

            let ptr = NonNull::<c_void>::dangling().as_ptr();
            assert_eq!(jlrs_ledger_try_borrow_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_try_borrow_shared(ptr), LedgerResult::OkTrue);
        }
    }

    #[test]
    fn unborrow_shared() {
        unsafe {
            Ledger::init();
            Ledger::clear();

            let ptr = NonNull::<c_void>::dangling().as_ptr();
            assert_eq!(jlrs_ledger_try_borrow_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_unborrow_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_is_borrowed_shared(ptr), LedgerResult::OkFalse);
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
            Ledger::init();
            Ledger::clear();

            let ptr = NonNull::<c_void>::dangling().as_ptr();
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
            Ledger::init();
            Ledger::clear();

            let ptr = NonNull::<c_void>::dangling().as_ptr();
            assert_eq!(jlrs_ledger_try_borrow_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_try_borrow_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_unborrow_shared(ptr), LedgerResult::OkFalse);
            assert_eq!(jlrs_ledger_is_borrowed_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(
                jlrs_ledger_is_borrowed_exclusive(ptr),
                LedgerResult::OkFalse
            );
            assert_eq!(jlrs_ledger_is_borrowed(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_unborrow_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_is_borrowed_shared(ptr), LedgerResult::OkFalse);
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
            Ledger::init();
            Ledger::clear();

            let ptr = NonNull::<c_void>::dangling().as_ptr();
            assert_eq!(jlrs_ledger_try_borrow_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_unborrow_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_unborrow_shared(ptr), LedgerResult::Err);
        }
    }

    #[test]
    fn unborrow_unborrowed_exclusive() {
        unsafe {
            Ledger::init();
            Ledger::clear();

            let ptr = NonNull::<c_void>::dangling().as_ptr();
            assert_eq!(jlrs_ledger_try_borrow_exclusive(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_unborrow_exclusive(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_unborrow_exclusive(ptr), LedgerResult::Err);
        }
    }
}
