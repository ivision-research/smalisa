#[macro_export]
macro_rules! ptr_eq {
    ($lhs:ident, $rhs:ident, $ty:ty) => {
        ::std::ptr::eq(
            $lhs as *const $ty as *const u8,
            $rhs as *const $ty as *const u8,
        )
    };
}

#[inline(always)]
pub(crate) fn ptr_eq<T>(lhs: &T, rhs: &T) -> bool {
    ::std::ptr::eq(lhs as *const T as *const u8, rhs as *const T as *const u8)
}

#[macro_export]
macro_rules! simple_err_display {
    ($ty:ty) => {
        impl ::std::fmt::Display for $ty {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

#[macro_export]
macro_rules! simple_deref {
    ($ty:ty, $field:ident, $target:ty) => {
        simple_deref!($ty, $field, $target,);
    };
    ($ty:ty, $field:ident, $target:ty, $($lt:lifetime),*) => {
        impl<$($lt),*> ::std::ops::Deref for $ty {
            type Target = $target;
            fn deref(&self) -> &Self::Target {
                &self.$field
            }
        }
    };

    /*
    ($ty:ty, $field:ident, $target:ty, $lt:lifetime) => {
        impl<$lt> ::std::ops::Deref for $ty {
            type Target = $target;
            fn deref(&self) -> &Self::Target {
                &self.$field
            }
        }
    };
    */
}

#[macro_export]
macro_rules! slice_iter {
    ($name:ident, $ty:ident, $lt:lifetime) => {
        pub type $name<$lt> = ::std::slice::Iter<$lt, $ty<$lt>>;
    };
}
