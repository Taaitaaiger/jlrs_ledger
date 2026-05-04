use spin::Mutex;
use std::ffi::c_void;

#[cfg(all(target_arch = "x86", target_os = "windows", target_env = "gnu"))]
#[allow(unused)]
#[unsafe(no_mangle)]
unsafe extern "C" fn _Unwind_Resume() {}

const INITIAL_CAPACITY: usize = 64;

/// The current API version.
#[unsafe(no_mangle)]
pub static JLRS_LEDGER_API_VERSION: usize = 3;

static LEDGER: Mutex<Ledger> = Mutex::new(Ledger {
    shared: vec![],
    shared_count: vec![],
    exclusive: vec![],
});

/// Indicates whether an operation succeeded, failed, or incorrect.
#[derive(PartialEq, Debug)]
#[repr(i32)]
pub enum LedgerResult {
    /// Operation succeeded and the result is `false`, or the operation failed.
    OkFalse = 0,
    /// Operation succeeded, result is `true`.
    OkTrue = 1,
    /// Operation is incorrect given the state of the ledger.
    Err = -1,
}

struct Ledger {
    // The addresses of the shared borrows.
    shared: Vec<usize>,
    // Number of times each share has been borrowed.
    shared_count: Vec<usize>,
    // The addresses of the exclusive borrows.
    exclusive: Vec<usize>,
}

impl Ledger {
    #[inline]
    fn reserve(&mut self, capacity: usize) {
        self.shared.reserve(capacity);
        self.shared_count.reserve(capacity);
        self.exclusive.reserve(capacity);
    }

    #[inline]
    fn is_borrowed_shared(&self, addr: *const c_void) -> LedgerResult {
        let addr = addr.addr();
        if self.shared.contains(&addr) {
            return LedgerResult::OkTrue;
        }

        LedgerResult::OkFalse
    }

    #[inline]
    fn is_borrowed_exclusive(&self, addr: *const c_void) -> LedgerResult {
        let addr = addr.addr();
        if self.exclusive.contains(&addr) {
            LedgerResult::OkTrue
        } else {
            LedgerResult::OkFalse
        }
    }

    #[inline]
    fn is_borrowed(&self, addr: *const c_void) -> LedgerResult {
        let addr = addr.addr();
        if self.exclusive.contains(&addr) || self.shared.contains(&addr) {
            LedgerResult::OkTrue
        } else {
            LedgerResult::OkFalse
        }
    }

    #[inline]
    fn n_shared_borrows(&self, addr: *const c_void) -> usize {
        let addr = addr.addr();
        match self.shared.iter().position(|x| x == &addr) {
            Some(idx) => self.shared_count[idx],
            None => 0,
        }
    }

    #[inline]
    fn unborrow_shared(&mut self, addr: *const c_void) -> LedgerResult {
        let addr = addr.addr();
        match self.shared.iter().position(|x| x == &addr) {
            Some(idx) => {
                self.shared_count[idx] -= 1;
                if self.shared_count[idx] == 0 {
                    self.shared.remove(idx);
                    self.shared_count.remove(idx);
                    LedgerResult::OkTrue
                } else {
                    LedgerResult::OkFalse
                }
            }
            None => LedgerResult::Err,
        }
    }

    #[inline]
    fn unborrow_exclusive(&mut self, addr: *const c_void) -> LedgerResult {
        let addr = addr.addr();
        match self.exclusive.iter().position(|x| x == &addr) {
            Some(idx) => {
                self.exclusive.remove(idx);
                LedgerResult::OkTrue
            }
            None => LedgerResult::Err,
        }
    }

    #[inline]
    fn try_borrow_shared(&mut self, addr: *const c_void) -> LedgerResult {
        let addr = addr.addr();
        if !self.exclusive.contains(&addr) {
            match self.shared.iter().position(|x| x == &addr) {
                Some(idx) => {
                    self.shared_count[idx] += 1;
                }
                None => {
                    self.shared.push(addr);
                    self.shared_count.push(1);
                }
            }
            LedgerResult::OkTrue
        } else {
            LedgerResult::OkFalse
        }
    }

    #[inline]
    fn try_borrow_exclusive(&mut self, addr: *const c_void) -> LedgerResult {
        let addr = addr.addr();

        if !self.shared.contains(&addr) && !self.exclusive.contains(&addr) {
            self.exclusive.push(addr);
            LedgerResult::OkTrue
        } else {
            LedgerResult::OkFalse
        }
    }

    #[inline]
    fn clear(&mut self) {
        self.shared.clear();
        self.shared_count.clear();
        self.exclusive.clear();
    }
}

/// Return the API version.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jlrs_ledger_api_version() -> usize {
    JLRS_LEDGER_API_VERSION
}

/// Initialize the ledger.
///
/// This function must be called before calling any other functions from the C API of this
/// crate, except [`jlrs_ledger_api_version`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jlrs_ledger_init() {
    LEDGER.lock().reserve(INITIAL_CAPACITY);
}

/// Check if the given pointer is tracked as a shared borrow.
///
/// Returns `LedgerResult::OkTrue` if it is borrowed sharedly, or `LedgerResult::OkFalse` if
/// it isn't.
///
/// Safety:
///
/// `jlrs_ledger_init` must have been called before calling this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jlrs_ledger_is_borrowed_shared(ptr: *const c_void) -> LedgerResult {
    LEDGER.lock().is_borrowed_shared(ptr)
}

/// Check if the given pointer is tracked as an exclusive borrow.
///
/// Returns `LedgerResult::OkTrue` if it is borrowed exclusively, or `LedgerResult::OkFalse`
/// if it isn't.
///
/// Safety:
///
/// `jlrs_ledger_init` must have been called before calling this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jlrs_ledger_is_borrowed_exclusive(ptr: *const c_void) -> LedgerResult {
    LEDGER.lock().is_borrowed_exclusive(ptr)
}

/// Check if the given pointer is tracked in some way.
///
/// Returns `LedgerResult::OkTrue` if it is borrowed, or `LedgerResult::OkFalse`
/// if it isn't.
///
/// Safety:
///
/// `jlrs_ledger_init` must have been called before calling this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jlrs_ledger_is_borrowed(ptr: *const c_void) -> LedgerResult {
    LEDGER.lock().is_borrowed(ptr)
}

/// The number of active shared borrows of `addr`.
///
/// Safety:
///
/// `jlrs_ledger_init` must have been called before calling this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jlrs_ledger_n_shared_borrows(ptr: *const c_void) -> usize {
    LEDGER.lock().n_shared_borrows(ptr)
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jlrs_ledger_unborrow_shared(ptr: *const c_void) -> LedgerResult {
    LEDGER.lock().unborrow_shared(ptr)
}

/// Stop tracking the pointer as an exclusive borrow.
///
/// Returns `LedgerResult::OkTrue` if it's no longer tracked, and `LedgerResult::Err` if the
/// pointer wasn't tracked or was tracked as a shared borrow.
///
/// Safety:
///
/// `jlrs_ledger_init` must have been called before calling this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jlrs_ledger_unborrow_exclusive(ptr: *const c_void) -> LedgerResult {
    LEDGER.lock().unborrow_exclusive(ptr)
}

/// Try to track the pointer as a shared borrow.
///
/// Returns `LedgerResult::OkTrue` on success, or `LedgerResult:OkFalse` if it is already
/// tracked as an exclusive borrow.
///
/// Safety:
///
/// `jlrs_ledger_init` must have been called before calling this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jlrs_ledger_try_borrow_shared(ptr: *const c_void) -> LedgerResult {
    LEDGER.lock().try_borrow_shared(ptr)
}
/// Try to track the pointer as an exclusive borrow.
///
/// Returns `LedgerResult::Ok` on success, or `LedgerResult:OkFalse` if it is already
/// tracked.
///
/// Safety:
///
/// `jlrs_ledger_init` must have been called before calling this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jlrs_ledger_try_borrow_exclusive(ptr: *const c_void) -> LedgerResult {
    LEDGER.lock().try_borrow_exclusive(ptr)
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
    LEDGER.lock().clear();
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
