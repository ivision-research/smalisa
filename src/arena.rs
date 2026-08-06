use bumpalo::Bump;

/// Backing storage for the strings a [Lexer](crate::Lexer) hands out.
///
/// Every `&str` in a parsed token, line, class or method borrows from an arena,
/// so the arena has to outlive them. Nothing is freed until it's dropped. One
/// arena per file is the intended use, and since it can't be shared across
/// threads, per thread as well.
pub struct Arena(Bump);

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

impl Arena {
    pub fn new() -> Self {
        Self(Bump::new())
    }

    /// Reset the arena to be reused. This may free some or none of the allocated memory.
    pub fn reset(&mut self) {
        self.0.reset();
    }

    /// Preallocates room for `bytes`, avoiding some growth for a file whose
    /// size is known up front.
    pub fn with_capacity(bytes: usize) -> Self {
        Self(Bump::with_capacity(bytes))
    }

    /// Bytes currently held. Only useful for diagnostics.
    pub fn allocated_bytes(&self) -> usize {
        self.0.allocated_bytes()
    }

    /// Copies `s` into the arena. Taking `&self` is what ties the returned
    /// lifetime to the arena rather than to a mutable borrow of the lexer.
    pub(crate) fn alloc_str<'a>(&'a self, s: &str) -> &'a str {
        self.0.alloc_str(s)
    }
}
