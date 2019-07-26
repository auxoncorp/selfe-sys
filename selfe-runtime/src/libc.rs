//! libsel4.a contains code that depends on __assert fail and strcpy. Since you
//! don't have a libc if you're using sel4-start, provide primitive
//! implementations of them here.

use core::fmt;

/// A tiny cstr wrapper to enable printing assertion failures
struct CStr(*const u8);

impl fmt::Display for CStr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unsafe {
            let mut p = self.0;
            loop {
                if *p == 0 {
                    break;
                }

                write!(f, "{}", *p as char)?;
                p = p.offset(1);
            }
        }

        Ok(())
    }
}

#[no_mangle]
pub extern "C" fn __assert_fail(expr: *const u8, file: *const u8, line: i32, func: *const u8) -> ! {
    panic!(
        "ASSERT {} in {} at {}:{}",
        CStr(expr),
        CStr(func),
        CStr(file),
        line
    );
}

#[no_mangle]
pub unsafe extern "C" fn strcpy(dest: *mut u8, mut source: *const u8) -> *const u8 {
    let mut d = dest;
    loop {
        *d = *source;
        if *d == 0 {
            return dest;
        } else {
            source = source.offset(1);
            d = d.offset(1);
        }
    }
}
