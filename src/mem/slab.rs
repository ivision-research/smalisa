use std::alloc;
use std::ptr;

// TODO Use NonNull for the ptr?
#[derive(Clone, Debug)]
pub(crate) struct Slab<T> {
    ptr: *mut T,
    cap: usize,
}

impl<T> Drop for Slab<T> {
    fn drop(&mut self) {
        let layout = Self::get_layout(self.cap);
        // SAFETY: ptr allocation is only handled in here. Layout is always
        // created the same way.
        unsafe {
            alloc::dealloc(self.ptr as *mut u8, layout);
        }
    }
}

impl<T> Slab<T> {
    fn get_layout(cap: usize) -> alloc::Layout {
        alloc::Layout::array::<T>(cap).expect("Couldn't create layout")
    }

    #[inline]
    pub(crate) fn get_cap(&self) -> usize {
        self.cap
    }

    pub(crate) fn new(cap: usize) -> Self {
        let layout = Self::get_layout(cap);
        assert!(layout.size() > 0);
        // SAFETY: The above assert ensures that layout has a nonzero size.
        let ptr = unsafe { alloc::alloc(layout) } as *mut T;
        if ptr.is_null() {
            alloc::handle_alloc_error(layout);
        }
        Self { ptr, cap }
    }

    pub(crate) fn new_copy(cap: usize, sl: &[T]) -> Self {
        let count = sl.len();
        let mut actual_cap = cap;
        while actual_cap < count {
            actual_cap = actual_cap.checked_shl(1).expect("!! overflow !!");
        }
        let slab = Self::new(actual_cap);
        let src = sl.as_ptr();
        let dst = slab.ptr;
        // SAFETY: This function is not exported beyond the crate and we only
        // ever use it with non overlapping slices. The validity of the slices
        // is guaranteed elsewhere. We know they won't overlap since we just
        // allocated src in this same function.
        unsafe {
            ptr::copy_nonoverlapping(src, dst, count);
        }
        slab
    }

    #[inline]
    pub(crate) fn as_mut_slice<'a>(&self) -> &'a mut [T] {
        let p = ptr::slice_from_raw_parts_mut(self.ptr, self.cap);
        // SAFETY: Cap is set whenever memory is allocated or reallocated, so
        // the memory should always be the appropriate length.
        unsafe { (&mut *p) as &'_ mut [T] }
    }

    pub(crate) fn grow<'a>(&mut self) -> &'a mut [T] {
        // TODO Growth rate?
        let new_cap = self.cap.checked_shl(1).expect("too much memory");
        let layout = alloc::Layout::array::<T>(self.cap).expect("couldn't create layout");
        let new_size = new_cap * std::mem::size_of::<T>();
        // SAFETY: TODO <- lol
        let mem = unsafe { alloc::realloc(self.ptr as *mut u8, layout, new_size) } as *mut T;
        if mem.is_null() {
            alloc::handle_alloc_error(layout);
        }
        self.ptr = mem;
        self.cap = new_cap;
        self.as_mut_slice()
    }
}

//#[cfg(test)]
//mod test {
//    use super::*;
//}
