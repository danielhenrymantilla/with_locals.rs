#![forbid(unsafe_code)]

include!("../prelude.rs");

/// Renaming the `'ref` lifetime to something else
const _: () = {
    trait ToStr {
        #[with('local)]
        fn to_str (self: &'_ Self)
          -> &'local str
        ;
    }
    impl ToStr for u32 {
        #[with('with)]
        fn to_str (self: &'_ u32)
          -> &'with str
        {
            let mut x = *self;
            if x == 0 { return "0"; }
            let mut buf = [b' '; 1 + 3 + 3 + 3]; // u32::MAX ~ 4_000_000_000
            let mut cursor = &mut buf[..];
            while x > 0 {
                let (last, cursor_) = cursor.split_last_mut().unwrap();
                cursor = cursor_;
                *last = b'0' + (x % 10) as u8;
                x /= 10;
            }
            let len = cursor.len();
            ::core::str::from_utf8(&buf[len ..])
                .unwrap()
        }
    }
};

/// Manually hand-rolling the continuation
#[with(continuation_name = ret)]
fn inside_if_yadda_early_return (n: u32)
  -> &'ref str
{
    use ::core::fmt::Display;

    #[with]
    fn returns_local (n: u32) -> &'ref dyn Display
    {
        &format_args!("{:#x}", n)
    }

    if true {
        #[with] let _it: &dyn Display = returns_local(n);
        let it: &'ref dyn Display = returns_local(n);
        let s = it.to_string();
        return ret(&*s);
    } else {
        ret!("");
    }
    ret("")
}

/// Combination of both
#[with('some_name, continuation_name = with)]
fn foo ()
  -> &'some_name ()
{
    with(&())
}

/// Recursive
#[with(recursive = true)]
fn rec (recurse: bool) -> &'ref ()
{
    if recurse {
        let _: &'ref _ = rec(false);
    }
    &()
}

/// Trailing commas
const _: () = {
    const _: () = {
        #[with('some_name ,)]
        fn __ ()
        {}
    };
    const _: () = {
        #[with(
            'some_name , continuation_name = whatever ,
        )]
        fn __ ()
        {}
    };
    const _: () = {
        #[with(
            'some_name , recursive = true ,
        )]
        fn __ ()
        {}
    };
    const _: () = {
        #[with(
            'some_name , continuation_name = whatever , recursive = false ,
        )]
        fn __ ()
        {}
    };
    const _: () = {
        #[with(
            'some_name , recursive = false , continuation_name = whatever ,
        )]
        fn __ ()
        {}
    };
    const _: () = {
        #[with(
            'some_name , continuation_name = whatever , recursive = false
        )]
        fn __ ()
        {}
    };
    const _: () = {
        #[with(
            recursive = false , continuation_name = whatever
        )]
        fn __ ()
        {}
    };
    const _: () = {
        #[with(
            continuation_name = whatever, recursive = false
        )]
        fn __ ()
        {}
    };
};
