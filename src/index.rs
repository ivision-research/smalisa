macro_rules! index_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Copy, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub struct $name(u32);

        impl $name {
            #[inline]
            pub fn new(index: usize) -> Self {
                debug_assert!(index <= u32::MAX as usize);
                Self(index as u32)
            }

            #[inline]
            pub fn index(self) -> usize {
                self.0 as usize
            }
        }
    };
}

pub(crate) use index_type;
