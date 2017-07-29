extern crate alloc;
use self::alloc::heap::{Alloc, Layout, AllocErr};

pub static mut ALLOCATE: extern fn(Layout) -> Result<*mut u8, AllocErr> = unset_allocate;
pub static mut DEALLOCATE: extern fn (*mut u8, Layout) = unset_deallocate;
pub static mut REALLOCATE: extern fn (*mut u8, Layout, Layout) -> Result<*mut u8, AllocErr> = unset_reallocate;

pub struct ScratchAlloc;

unsafe impl<'a> Alloc for &'a ScratchAlloc {
    unsafe fn alloc(&mut self, layout: Layout) -> Result<*mut u8, AllocErr> {
        ALLOCATE(layout)
    }

    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        DEALLOCATE(ptr, layout);
    }

    unsafe fn realloc(&mut self, ptr: *mut u8, old_layout: Layout, new_layout: Layout) -> Result<*mut u8, AllocErr> {
        REALLOCATE(ptr, old_layout, new_layout)
    }
}

#[allow(unused_variables)]
extern fn unset_allocate(layout: Layout) -> Result<*mut u8, AllocErr> {
    Err(AllocErr::Unsupported { details: "No heap available. Call sel4_start::switch_to_scratch() to use the static scratch heap." })
}

#[allow(unused_variables)]
extern fn unset_deallocate(ptr: *mut u8, layout: Layout) {
    unimplemented!();
}

#[allow(unused_variables)]
extern fn unset_reallocate(ptr: *mut u8, old_layout: Layout, new_layout: Layout) -> Result<*mut u8, AllocErr> {
    Err(AllocErr::Unsupported { details: "No heap available. Call sel4_start::switch_to_scratch() to use the static scratch heap." })
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
extern fn scratch_allocate(layout: Layout) -> Result<*mut u8, AllocErr> {
    unsafe {
        SCRATCH_PTR += SCRATCH_PTR % layout.align();
        let res = &mut SCRATCH_HEAP[SCRATCH_PTR];
        SCRATCH_PTR += layout.size();
        if SCRATCH_PTR <= SCRATCH_LEN_BYTES {
            Ok(res)
        } else {
            Err(AllocErr::Exhausted { request: layout })
        }
    }
}

#[allow(unused_variables)]
extern fn scratch_deallocate(ptr: *mut u8, layout: Layout) {
    unsafe {
        if SCRATCH_PTR - layout.size() == ptr as usize {
            SCRATCH_PTR -= layout.size();
        }
    }
}

#[allow(unused_variables)]
extern fn scratch_reallocate(ptr: *mut u8, old_layout: Layout, new_layout: Layout) -> Result<*mut u8, AllocErr> {
    scratch_deallocate(ptr, old_layout);
    scratch_allocate(new_layout)
}

pub unsafe fn switch_to_scratch() {
    ALLOCATE = scratch_allocate;
    DEALLOCATE = scratch_deallocate;
    REALLOCATE = scratch_reallocate;
}
