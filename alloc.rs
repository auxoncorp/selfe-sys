pub static mut ALLOCATE: extern fn(usize, usize) -> *mut u8 = unset_allocate;
pub static mut DEALLOCATE: extern fn (*mut u8, usize, usize) = unset_deallocate;
pub static mut REALLOCATE: extern fn (*mut u8, usize, usize, usize) -> *mut u8 = unset_reallocate;
pub static mut REALLOCATE_INPLACE: extern fn (*mut u8, usize, usize, usize) -> usize = unset_reallocate_inplace;
pub static mut USABLE_SIZE: extern fn (usize, usize) -> usize = unset_usable_size;

#[no_mangle]
pub extern fn __rust_allocate(size: usize, align: usize) -> *mut u8 {
    unsafe { ALLOCATE(size, align) }
}

#[no_mangle]
pub extern fn __rust_deallocate(ptr: *mut u8, old_size: usize, align: usize) {
    unsafe { DEALLOCATE(ptr, old_size, align) }
}

#[no_mangle]
pub extern fn __rust_reallocate(ptr: *mut u8, old_size: usize, size: usize,
                                align: usize) -> *mut u8 {
    unsafe { REALLOCATE(ptr, old_size, size, align) }
}

#[no_mangle]
pub extern fn __rust_reallocate_inplace(ptr: *mut u8, old_size: usize,
                                        size: usize, align: usize) -> usize {
    unsafe { REALLOCATE_INPLACE(ptr, old_size, size, align) }
}

#[no_mangle]
pub extern fn __rust_usable_size(size: usize, align: usize) -> usize {
    unsafe { USABLE_SIZE(size, align) }
}

#[allow(unused_variables)]
extern fn unset_allocate(size: usize, align: usize) -> *mut u8 {
    unimplemented!();
}

#[allow(unused_variables)]
extern fn unset_deallocate(ptr: *mut u8, old_size: usize, align: usize) {
    unimplemented!();
}

#[allow(unused_variables)]
extern fn unset_reallocate(ptr: *mut u8, old_size: usize, size: usize,
                                align: usize) -> *mut u8 {
    unimplemented!();
}

#[allow(unused_variables)]
extern fn unset_reallocate_inplace(_ptr: *mut u8, old_size: usize,
                                        size: usize, align: usize) -> usize {
    unimplemented!();
}

#[allow(unused_variables)]
extern fn unset_usable_size(size: usize, align: usize) -> usize {
    unimplemented!();
}

// Note: reduce to 1024 * 16 * 4 + 1024 * 16 eventually.
// Chosen because Hier needs 16KiB per paging level, and the max is 4 levels on x86_64. An
// additional 16KiB is tossed in "just in case", if for example the eventual malloc needs some
// out-of-line space for metadata.
//
// need scratch for reservations etc.
const SCRATCH_LEN_BYTES: usize = 1024 * 1024 * 16;
static mut SCRATCH_HEAP: [u8; SCRATCH_LEN_BYTES] = [0; SCRATCH_LEN_BYTES];
static mut SCRATCH_PTR: usize = 0;

#[allow(unused_variables)]
extern fn scratch_allocate(size: usize, align: usize) -> *mut u8 {
    unsafe {
        SCRATCH_PTR += SCRATCH_PTR % align;
        let res = &mut SCRATCH_HEAP[SCRATCH_PTR];
        SCRATCH_PTR += size;
        assert!(SCRATCH_PTR <= SCRATCH_LEN_BYTES);
        res
    }
}

#[allow(unused_variables)]
extern fn scratch_deallocate(ptr: *mut u8, old_size: usize, align: usize) {
    unsafe {
        if SCRATCH_PTR - old_size == ptr as usize {
            SCRATCH_PTR -= old_size;
        }
    }
}

#[allow(unused_variables)]
extern fn scratch_reallocate(ptr: *mut u8, old_size: usize, size: usize,
                                align: usize) -> *mut u8 {
    scratch_deallocate(ptr, old_size, align);
    scratch_allocate(size, align)
}

#[allow(unused_variables)]
extern fn scratch_reallocate_inplace(_ptr: *mut u8, old_size: usize,
                                        size: usize, align: usize) -> usize {
    0
}

#[allow(unused_variables)]
extern fn scratch_usable_size(size: usize, align: usize) -> usize {
    size
}

pub unsafe fn switch_to_scratch() {
    ALLOCATE = scratch_allocate;
    DEALLOCATE = scratch_deallocate;
    REALLOCATE = scratch_reallocate;
    REALLOCATE_INPLACE = scratch_reallocate_inplace;
    USABLE_SIZE = scratch_usable_size;
}
