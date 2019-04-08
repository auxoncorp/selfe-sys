extern crate alloc;
use core::alloc::{Alloc, Layout, AllocErr, Opaque, GlobalAlloc};
use core::ptr::NonNull;

pub static mut ALLOCATE: extern fn(Layout) -> Result<NonNull<Opaque>, AllocErr> = unset_allocate;
pub static mut DEALLOCATE: extern fn (NonNull<Opaque>, Layout) = unset_deallocate;
pub static mut REALLOCATE: extern fn (NonNull<Opaque>, Layout, usize) -> Result<NonNull<Opaque>, AllocErr> = unset_reallocate;

pub struct ScratchAlloc;

unsafe impl<'a> Alloc for &'a ScratchAlloc {
    unsafe fn alloc(&mut self, layout: Layout) -> Result<NonNull<Opaque>, AllocErr> {
        ALLOCATE(layout)
    }

    unsafe fn dealloc(&mut self, ptr: NonNull<Opaque>, layout: Layout) {
        DEALLOCATE(ptr, layout);
    }

    unsafe fn realloc(&mut self, ptr: NonNull<Opaque>, old_layout: Layout, new_size: usize) -> Result<NonNull<Opaque>, AllocErr> {
        REALLOCATE(ptr, old_layout, new_size)
    }
}

unsafe impl GlobalAlloc for ScratchAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut Opaque {
        ALLOCATE(layout).unwrap().as_ptr()
    }

    unsafe fn dealloc(&self, ptr: *mut Opaque, layout: Layout) {
        DEALLOCATE(NonNull::new(ptr).unwrap(), layout);
    }

    unsafe fn realloc(&self, ptr: *mut Opaque, old_layout: Layout, new_size: usize) -> *mut Opaque {
        REALLOCATE(NonNull::new(ptr).unwrap(), old_layout, new_size).unwrap().as_ptr()
    }
}

#[allow(unused_variables)]
extern fn unset_allocate(layout: Layout) -> Result<NonNull<Opaque>, AllocErr> {
    Err(AllocErr)
}

#[allow(unused_variables)]
extern fn unset_deallocate(ptr: NonNull<Opaque>, layout: Layout) {
    unimplemented!();
}

#[allow(unused_variables)]
extern fn unset_reallocate(ptr: NonNull<Opaque>, old_layout: Layout, new_size: usize) -> Result<NonNull<Opaque>, AllocErr> {
    Err(AllocErr)
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
extern fn scratch_allocate(layout: Layout) -> Result<NonNull<Opaque>, AllocErr> {
    unsafe {
        SCRATCH_PTR += SCRATCH_PTR % layout.align();
        let res = &mut SCRATCH_HEAP[SCRATCH_PTR];
        SCRATCH_PTR += layout.size();
        if SCRATCH_PTR <= SCRATCH_LEN_BYTES {
            Ok(NonNull::new(res).unwrap().as_opaque())
        } else {
            Err(AllocErr)
        }
    }
}

#[allow(unused_variables)]
extern fn scratch_deallocate(ptr: NonNull<Opaque>, layout: Layout) {
    unsafe {
        if SCRATCH_PTR - layout.size() == ptr.as_ptr() as usize {
            SCRATCH_PTR -= layout.size();
        }
    }
}

#[allow(unused_variables)]
extern fn scratch_reallocate(ptr: NonNull<Opaque>, old_layout: Layout, new_size: usize) -> Result<NonNull<Opaque>, AllocErr> {
    let res = Layout::from_size_align(new_size, old_layout.align());
    scratch_deallocate(ptr, old_layout);
    scratch_allocate(res.unwrap())
}

pub unsafe fn switch_to_scratch() {
    ALLOCATE = scratch_allocate;
    DEALLOCATE = scratch_deallocate;
    REALLOCATE = scratch_reallocate;
}
