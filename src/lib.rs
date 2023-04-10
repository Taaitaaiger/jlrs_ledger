use std::{ffi::c_void, sync::Mutex};

#[cfg(all(target_arch = "x86", target_os = "windows", target_env = "gnu"))]
#[allow(unused)]
#[no_mangle]
unsafe extern "C" fn _Unwind_Resume() {}

#[no_mangle]
pub static JLRS_LEDGER_API_VERSION: usize = 1;

struct Ledger {
    exclusive: Vec<*const c_void>,
    shared: Vec<*const c_void>,
}

unsafe impl Send for Ledger {}

#[derive(PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum LedgerResult {
    OkFalse = 0u8,
    OkTrue = 1u8,
    Err = 2u8,
    Poison = 3u8,
}

static LEDGER: Mutex<Ledger> = Mutex::new(Ledger::new());

impl Ledger {
    const fn new() -> Self {
        Ledger {
            exclusive: Vec::new(),
            shared: Vec::new(),
        }
    }

    fn is_borrowed_shared(ptr: *const c_void) -> LedgerResult {
        match LEDGER.lock() {
            Ok(ledger) => {
                if is_borrowed(ledger.shared.as_ref(), ptr) {
                    return LedgerResult::OkTrue;
                } else {
                    return LedgerResult::OkFalse;
                }
            }
            _ => LedgerResult::Poison,
        }
    }

    fn is_borrowed_exclusive(ptr: *const c_void) -> LedgerResult {
        match LEDGER.lock() {
            Ok(ledger) => {
                if is_borrowed(ledger.exclusive.as_ref(), ptr) {
                    return LedgerResult::OkTrue;
                } else {
                    return LedgerResult::OkFalse;
                }
            }
            _ => LedgerResult::Poison,
        }
    }

    fn is_borrowed(ptr: *const c_void) -> LedgerResult {
        match LEDGER.lock() {
            Ok(ledger) => {
                if is_borrowed(ledger.shared.as_ref(), ptr)
                    || is_borrowed(ledger.exclusive.as_ref(), ptr)
                {
                    return LedgerResult::OkTrue;
                } else {
                    return LedgerResult::OkFalse;
                }
            }
            _ => LedgerResult::Poison,
        }
    }

    // Dynamically check a slice conforms to borrow rules
    unsafe fn try_borrow_shared(ptr: *const c_void) -> LedgerResult {
        match LEDGER.lock() {
            Ok(mut ledger) => ledger.try_add_borrow_shared(ptr),
            _ => LedgerResult::Poison,
        }
    }

    // Dynamically check a mutable slice conforms to borrow rules before returning by
    // using interior mutability of the ledger.
    unsafe fn try_borrow_exclusive(ptr: *const c_void) -> LedgerResult {
        match LEDGER.lock() {
            Ok(mut ledger) => ledger.try_add_borrow_exclusive(ptr),
            _ => LedgerResult::Poison,
        }
    }

    unsafe fn borrow_shared_unchecked(ptr: *const c_void) -> LedgerResult {
        match LEDGER.lock() {
            Ok(mut ledger) => {
                ledger.shared.push(ptr);
                LedgerResult::OkTrue
            }
            _ => LedgerResult::Poison,
        }
    }

    unsafe fn unborrow_shared(ptr: *const c_void) -> LedgerResult {
        match LEDGER.lock() {
            Ok(mut ledger) => {
                if let Some(i) = ledger.shared.iter().copied().rposition(|r| r == ptr) {
                    ledger.shared.remove(i);
                    LedgerResult::OkTrue
                } else {
                    LedgerResult::Err
                }
            }
            _ => LedgerResult::Poison,
        }
    }

    unsafe fn unborrow_exclusive(ptr: *const c_void) -> LedgerResult {
        match LEDGER.lock() {
            Ok(mut ledger) => {
                if let Some(i) = ledger.exclusive.iter().copied().rposition(|r| r == ptr) {
                    ledger.exclusive.remove(i);
                    LedgerResult::OkTrue
                } else {
                    LedgerResult::Err
                }
            }
            _ => LedgerResult::Poison,
        }
    }

    // Try to add an immutable borrow to the ledger
    unsafe fn try_add_borrow_shared(&mut self, ptr: *const c_void) -> LedgerResult {
        // Check if the data is already borrowed
        if is_borrowed(&self.exclusive, ptr) {
            return LedgerResult::Err;
        }

        // Push the pointer to indicate it's borrowed
        self.shared.push(ptr);

        LedgerResult::OkTrue
    }

    // Try to add a mutable borrow to the ledger
    unsafe fn try_add_borrow_exclusive(&mut self, ptr: *const c_void) -> LedgerResult {
        // Check if the borrow overlaps with any active mutable borrow
        if is_borrowed(&self.exclusive, ptr) {
            return LedgerResult::Err;
        }

        // Check if the borrow overlaps with any active immutable borrow
        if is_borrowed(&self.shared, ptr) {
            return LedgerResult::Err;
        }

        // Record a record of the mutable borrow
        self.exclusive.push(ptr);

        LedgerResult::OkTrue
    }

    #[cfg(test)]
    unsafe fn clear() {
        match LEDGER.lock() {
            Ok(mut ledger) => {
                ledger.shared = Vec::new();
                ledger.exclusive = Vec::new();
            }
            _ => panic!(),
        }
    }
}

fn is_borrowed(existing: &[*const c_void], ptr: *const c_void) -> bool {
    if existing.iter().copied().all(|i| i != ptr) {
        false
    } else {
        true
    }
}

/* API */
#[no_mangle]
pub unsafe extern "C" fn jlrs_ledger_api_version() -> usize {
    JLRS_LEDGER_API_VERSION
}

#[no_mangle]
pub unsafe extern "C" fn jlrs_ledger_is_borrowed_shared(ptr: *const c_void) -> LedgerResult {
    Ledger::is_borrowed_shared(ptr)
}

#[no_mangle]
pub unsafe extern "C" fn jlrs_ledger_is_borrowed_exclusive(ptr: *const c_void) -> LedgerResult {
    Ledger::is_borrowed_exclusive(ptr)
}

#[no_mangle]
pub unsafe extern "C" fn jlrs_ledger_is_borrowed(ptr: *const c_void) -> LedgerResult {
    Ledger::is_borrowed(ptr)
}

#[no_mangle]
pub unsafe extern "C" fn jlrs_ledger_borrow_shared_unchecked(ptr: *const c_void) -> LedgerResult {
    Ledger::borrow_shared_unchecked(ptr)
}

#[no_mangle]
pub unsafe extern "C" fn jlrs_ledger_unborrow_shared(ptr: *const c_void) -> LedgerResult {
    Ledger::unborrow_shared(ptr)
}

#[no_mangle]
pub unsafe extern "C" fn jlrs_ledger_unborrow_exclusive(ptr: *const c_void) -> LedgerResult {
    Ledger::unborrow_exclusive(ptr)
}

#[no_mangle]
pub unsafe extern "C" fn jlrs_ledger_try_borrow_shared(ptr: *const c_void) -> LedgerResult {
    Ledger::try_borrow_shared(ptr)
}

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
            Ledger::clear();

            let ptr = NonNull::<c_void>::dangling().as_ptr();
            assert_eq!(jlrs_ledger_try_borrow_exclusive(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_try_borrow_shared(ptr), LedgerResult::Err);
        }
    }

    #[test]
    fn borrow_shared_then_exclusive() {
        unsafe {
            Ledger::clear();

            let ptr = NonNull::<c_void>::dangling().as_ptr();
            assert_eq!(jlrs_ledger_try_borrow_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_try_borrow_exclusive(ptr), LedgerResult::Err);
        }
    }

    #[test]
    fn borrow_shared_twice() {
        unsafe {
            Ledger::clear();

            let ptr = NonNull::<c_void>::dangling().as_ptr();
            assert_eq!(jlrs_ledger_try_borrow_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_try_borrow_shared(ptr), LedgerResult::OkTrue);
        }
    }

    #[test]
    fn unborrow_shared() {
        unsafe {
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
            Ledger::clear();

            let ptr = NonNull::<c_void>::dangling().as_ptr();
            assert_eq!(jlrs_ledger_try_borrow_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_try_borrow_shared(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_unborrow_shared(ptr), LedgerResult::OkTrue);
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
            Ledger::clear();

            let ptr = NonNull::<c_void>::dangling().as_ptr();
            assert_eq!(jlrs_ledger_try_borrow_exclusive(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_unborrow_exclusive(ptr), LedgerResult::OkTrue);
            assert_eq!(jlrs_ledger_unborrow_exclusive(ptr), LedgerResult::Err);
        }
    }
}
