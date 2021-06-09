#![cfg_attr(feature = "nightly",
    cfg_attr(all(), doc = include_str!("../README.md")),
)]

#![forbid(unsafe_code)]
#![no_std]


/// To avoid a bug when cross compiling
extern crate proc_macros;

pub use ::proc_macros::with;

/// For advanced users that manually write the `with` closure of `dyn_safe`
/// function.
pub
mod dyn_safe {
    /// Used to manually call `#[with(dyn_safe = true)]` functions.
    ///
    /// They need a fixed / non-generic return type, but using `()` would be
    /// error-prone when manually implementing such `with` functions. Using
    /// this is thus more type-safe.
    pub
    struct ContinuationReturn;
}

#[doc(hidden)] /** Not part of the public API **/ pub
mod __ {
    pub
    enum ControlFlow<Eval, Return, Break, Continue> {
        /// Classic block evaluation.
        Eval(Eval),

        /// Must `return` the value early.
        EarlyReturn(Return),

        /// Must `break` with the value.
        Break(Break),

        /// Must `continue`.
        Continue(Continue),
    }

    /// Custom *void type*
    pub
    enum Unreachable {}

    pub
    use ::core::{
        convert::Into,
        ops::{
            FnMut, FnOnce,
        },
        option::Option::{Some as Some_, None as None_},
        result::Result::{Ok as Ok_, Err as Err_},
    };

    pub
    trait Try {
        type Ok;
        type Error;

        fn into_result (self: Self)
          -> Result<Self::Ok, Self::Error>
        ;

        fn from_ok (ok: Self::Ok)
          -> Self
        ;

        fn from_err (err: Self::Error)
          -> Self
        ;
    }

    impl<Ok, Err> Try for Result<Ok, Err> {
        type Ok = Ok;
        type Error = Err;

        #[inline]
        fn into_result (self: Result<Ok, Err>)
          -> Result<Ok, Err>
        {
            self
        }

        #[inline]
        fn from_ok (ok: Ok)
          -> Result<Ok, Err>
        {
            Ok(ok)
        }

        #[inline]
        fn from_err (err: Err)
          -> Result<Ok, Err>
        {
            Err(err)
        }
    }

    mod hidden { pub struct NoneError; }
    use hidden::NoneError;

    impl<T> Try for Option<T> {
        type Ok = T;
        type Error = NoneError;

        #[inline]
        fn into_result (self: Option<T>)
          -> Result<T, NoneError>
        {
            self.ok_or(NoneError)
        }

        #[inline]
        fn from_ok (value: T)
          -> Option<T>
        {
            Some(value)
        }

        #[inline]
        fn from_err (NoneError: NoneError)
          -> Option<T>
        {
            None
        }
    }
}
