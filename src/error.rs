
use alloc::{boxed::Box, string::String};

pub trait Error: core::fmt::Debug + core::fmt::Display{}

impl<E: Error> Error for Box<E>{}

impl<E: Error+Send+Sync+'static> From<E> for Box<dyn Error+Send+Sync>{
    fn from(err: E) -> Self{
        Box::new(err)
    }
}

impl From<String> for Box<dyn Error+Send+Sync>{
    fn from(s: String) -> Self{
        pub struct WrappedError(String);
        impl core::fmt::Debug for WrappedError{
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.0.fmt(f)
            }
        }
        impl core::fmt::Display for WrappedError{
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.0.fmt(f)
            }
        }
        impl Error for WrappedError{}

        Box::new(WrappedError(s))
    }
}


impl From<&'static str> for Box<dyn Error+Send+Sync>{
    fn from(s: &'static str) -> Self{
        pub struct WrappedError(&'static str);
        impl core::fmt::Debug for WrappedError{
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.0.fmt(f)
            }
        }
        impl core::fmt::Display for WrappedError{
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.0.fmt(f)
            }
        }
        impl Error for WrappedError{}

        Box::new(WrappedError(s))
    }
}
