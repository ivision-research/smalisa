use super::slab::Slab;

// TODO I haven't checked this implementation for memory leaks or other bad
// behavior in the case of panics.

// TODO Find out a good value for this
pub const DEFAULT_HEAP_SIZE: usize = 2048;

// A structure to allocate memory for &strs. This helps us not have to perform
// as many copys/allocations and all memory is managed in this one single
// place to centralize use of unsafe.
//
// This is intended to be used on a per file basis because NOTHING is
// deallocated until this struct is dropped.
pub(crate) struct StrStorage<'s> {
    cur: &'s mut [u8],
    idx: usize,
    end: usize,
    cap: usize,
    slabs: Vec<Slab<u8>>,
}

impl<'s> StrStorage<'s> {
    #[inline]
    pub fn new() -> Self {
        Self::new_cap(DEFAULT_HEAP_SIZE)
    }

    #[inline]
    pub fn new_cap(cap: usize) -> Self {
        let first = Slab::new(cap);
        let cur = first.as_mut_slice();
        Self {
            cur,
            idx: 0,
            end: 0,
            cap,
            slabs: vec![first],
        }
    }

    pub fn push_all(&mut self, val: &[u8]) {
        let len = val.len();
        if len <= self.cap - self.end {
            let new_end = self.end + len;
            self.cur[self.end..new_end].copy_from_slice(val);
            self.end = new_end;
            return;
        }
        // TODO
        for b in val {
            self.push(*b);
        }
    }

    pub fn push(&mut self, val: u8) {
        if self.end >= self.cap {
            if self.idx == 0 {
                let slab = self.slabs.last_mut().expect("should always have a slab");
                self.cur = slab.grow();
                self.cap = slab.get_cap();
                self.cur[self.end] = val;
                self.end += 1;
            } else {
                // We have to create a new slab because reallocing could be
                // a disaster.
                let slab = if self.idx == self.end {
                    // TODO Check this code because I wrote it just to fix a bug
                    let slab = Slab::new(self.cap);
                    self.cur = slab.as_mut_slice();
                    self.end = 0;
                    slab
                } else {
                    self.end -= self.idx;
                    Slab::new_copy(self.cap, &self.cur[self.idx..])
                };
                self.cap = slab.get_cap();
                self.cur = slab.as_mut_slice();
                self.slabs.push(slab);
                self.idx = 0;
                self.cur[self.end] = val;
                self.end += 1;
            }
        } else {
            self.cur[self.end] = val;
            self.end += 1;
        }
    }

    /// Returns the current buffer and advances the index so this memory is
    /// no longer writable. This function is a safe usage of view_slice.
    #[inline]
    fn take_slice(&mut self) -> &'s [u8] {
        let slice = self.view_slice();
        // This is the "take" part -- none of the functions on this type
        // should be able to modify the returned slice after this.
        self.idx = self.end;
        slice
    }

    //#[inline]
    //pub fn take_str(&mut self) -> Result<&'s str, std::str::Utf8Error> {
    //    let bytes = self.take_slice();
    //    std::str::from_utf8(bytes)
    //}

    #[inline]
    pub fn take_str_unchecked(&mut self) -> &'s str {
        let bytes = self.take_slice();
        // SAFETY: In the documentation for this crate we emphasize that the
        // input file MUST be UTF8 encoded: that is an invariant for this
        // crate. If that invariant is violated, then this block is definitely
        // not safe.
        unsafe { std::str::from_utf8_unchecked(&bytes) }
    }

    /// Clear the currently stored slice.
    #[inline]
    pub fn clear(&mut self) {
        self.end = self.idx;
    }

    /// Calls the given function with the currently stored string as the
    /// argument.
    ///
    /// This function is intended to allow the caller to view the current
    /// string without the risk of trying to view the raw underlying slice.
    /// The caller will still need to call `take_*` to actually get
    /// the underlying string if they need it.
    #[inline]
    pub fn check_str<T, F>(&self, f: F) -> T
    where
        F: FnOnce(&str) -> T,
    {
        let s = self.view_str_unchecked();
        f(s)
    }

    //  Just shows returns the current buffer without changing anything.
    //  Repeated calls to this should always return the same slice.
    fn view_slice(&self) -> &'s [u8] {
        let size = self.end - self.idx;
        if size == 0 {
            return &[];
        }
        let ptr = self.cur[self.idx..].as_ptr();
        // SAFETY: view_slice is only ever used internally to this file and
        // only over in `check_str` or `take_slice`. The `take_slice` call
        // is safe because it will increment the index, nothing can actually
        // modify the data after that. The `check_str` usage is safe because
        // a read only view of the memory is given to a function with a more
        // limited lifetime.
        unsafe { &*std::ptr::slice_from_raw_parts(ptr, size) }
    }

    #[inline]
    fn view_str_unchecked(&self) -> &'s str {
        // SAFETY: See view_slice.
        let bytes = self.view_slice();
        unsafe { std::str::from_utf8_unchecked(&bytes) }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn simple_take_slice() {
        let mut bb = StrStorage::new();
        bb.push(b'H');
        bb.push(b'i');
        assert_eq!(bb.take_slice(), &[b'H', b'i']);
        assert_eq!(bb.take_slice(), &[]);
        bb.push(b'H');
        bb.push(b'e');
        bb.push(b'l');
        bb.push(b'l');
        bb.push(b'o');
        assert_eq!(bb.take_slice(), &[b'H', b'e', b'l', b'l', b'o']);
    }

    #[test]
    fn take_str_unchecked() {
        let msg = "Hello World\n";
        let msg_b = msg.as_bytes();
        let mut bb = StrStorage::new();
        bb.push_all(msg_b);
        assert_eq!(bb.take_str_unchecked(), msg);
    }

    #[test]
    fn view_str_unchecked() {
        let msg = "Hello World\n";
        let msg_b = msg.as_bytes();
        let mut bb = StrStorage::new();
        bb.push_all(msg_b);
        assert_eq!(bb.view_str_unchecked(), msg);
        assert_eq!(bb.view_str_unchecked(), msg);
    }

    #[test]
    fn grows_on_alloc_too_small() {
        let msg = "Hello World Hello World\n";
        let msg_b = msg.as_bytes();
        let mut bb = StrStorage::new_cap(8);
        bb.push_all(msg_b);
        assert_eq!(bb.take_str_unchecked(), msg);
        assert_eq!(bb.cap, 32);
        assert_eq!(bb.slabs.len(), 1);
    }

    #[test]
    fn creates_new_slab() {
        let msg = "Hello 世界\n";
        let msg_b = msg.as_bytes();
        let first_cap = msg_b.len() + 2;
        let mut bb = StrStorage::new_cap(first_cap);
        bb.push_all(msg_b);
        assert_eq!(bb.take_str_unchecked(), msg);
        assert_eq!(bb.cap, first_cap);
        assert_eq!(bb.slabs.len(), 1);

        bb.push_all(msg_b);
        assert_eq!(bb.take_str_unchecked(), msg);
        assert_eq!(bb.cap, first_cap);
        assert_eq!(bb.slabs.len(), 2);
    }
}
