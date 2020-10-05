use ::core::fmt::Display;

use ::with_locals::with;

/// A basic example: returning / yielding a `format_args` local.
#[with]
fn hex (n: u32) -> &'ref dyn Display
{
    &format_args!("{:#x}", n)
}

no_run! {
    /// The above becomes:
    fn with_hex<R, F> (n: u32, f: F) -> R
    where
        F : for<'local> FnOnce(&'local dyn Display) -> R,
    {
        f(&format_args!("{:#x}", n))
    }
    // `f: F`, here, is called a continuation:
    // instead of having a function return / yield some element / object,
    // the function takes the "logic" of what the caller would have liked to
    // do with that element, once it would have received it.
    //
    // By shifting the logic like so, it is the callee and not the caller
    // who runs that logic, **which thus happens before the callee returns,
    // cleaning its locals and making things that refer to it dangle.**
    //
    // THIS IS THE WHOLE POINT of the strategy!.

    // Now, to call / use the above function, one can no longer bind the "result"
    // of that function to a variable using a `let` binding, since that mechanism
    // is reserved for actual returns, and the actual code running in the caller's
    // stack.
    //
    // Instead, one calls / uses that `with_hex` function using
    // closure / callback syntax:
    with_hex(66, |s| {
        println!("{}", s);
    })

    // This is extremely powerful, but incurs in a rightward drift everytime
    // such a binding is created:

    with_hex(1, |one| {
        with_hex(2, |two| {
            with_hex(3, |three| {
                // ughhh ..
            })
        })
    })

    // Instead, it would be nice if the compiler / the language provided a way
    // for `let` bindings to magically perform that transformation:
    let one = hex(1);
    let two = hex(2);
    let three = hex(3);

    // Operating in this fashion is called Continuation-Passing Style, and
    // cannot be done implicitly in Rust.
    // But that doesn't mean one cannot get sugar for it.

    // Enters `#[with]`!

    #[with] let one = hex(1);
    #[with] let two = hex(2);
    #[with] let three = hex(3);

    // Or, equivalently:

    let one: &'ref _ = hex(1);
    let two: &'ref _ = hex(2);
    let three: &'ref _ = hex(3);
    //          ^^^^^
    //          special lifetime is equivalent to marking the
    //          `let` binding with `#[with]`.

    // When applied to a function, it will tranform all the so-annotated
    // `let` bindings into a closure call, where all the statements that
    // follow the binding (within the same scope) are moved into the
    // continuation.

    // Here is an example:
}

#[with]
fn hex_example ()
{
    let s: String = {
        println!("Hello, World!");
        let s_hex: &'ref _ = hex(66);
        println!("s_hex = {}", s_hex);
        let s = s_hex.to_string();
        assert_eq!(s, "0x42");
        s
    };
    assert_eq!(s, "0x42");
}

no_run! {
    // The above becomes:
    let s: String = {
        println!("Hello, World!");
        with_hex(66, |s_hex| {
            println!("s_hex = {}", s_hex);
            let s = s_hex.to_string();
            assert_eq!(s, "0x42");
            s
        })
    };
    assert_eq!(s, "0x42");
}

/// Traits can have `#[with]`-annotated methods too.
trait ToStr {
    #[with]
    fn to_str (self: &'_ Self) -> &'ref str
    ;
}

/// Example of a user of of the trait (≠ an implementor).
impl<T : ToStr> Display for Displayable<T> {
    #[with] // you can #[with]-annotate classic function,
            // in order to get the `let` assignment magic :)
    fn fmt (self: &'_ Self, fmt: &'_ mut ::core::fmt::Formatter<'_>)
      -> ::core::fmt::Result
    {
        let s: &'ref str = self.0.to_str();
        fmt.write_str(s)
    }
}
// (Using a newtype to avoid coherence issues)
struct Displayable<T : ToStr>(T);

/// Example of an implementor
impl ToStr for u32 {
    #[with('local)] // At any point, you can choose to use another name
                    // for the special lifetime that tells the attribute to
                    // transform the function into a `with_...` one.
                    // By default, that name is `'ref`, since it is currently
                    // forbidden by the compiler, and I find it quite on point.
                    //
                    // But when `self` receivers are involved, this `'ref`
                    // name may be confusing. If you feel that's the case,
                    // feel free to rename it :)
    fn to_str (self: &'_ u32) -> &'local str
    {
        let mut x = *self;
        if x == 0 {
            // By default, the macro tries to be quite smart and replaces
            // both implicitly returned and explicitly returned values, with
            // what the actual return of the actual `with_...` function must
            // be: `return f("0");`.
            return "0";
        }
        let mut buf = [b' '; 1 + 3 + 3 + 3]; // u32::MAX ~ 4_000_000_000
        let mut cursor = &mut buf[..];
        while x > 0 {
            let (last, cursor_) = cursor.split_last_mut().unwrap();
            cursor = cursor_;
            *last = b'0' + (x % 10) as u8;
            x /= 10;
        }
        let len = cursor.len();
        // return f(
        ::core::str::from_utf8(&buf[len ..]) // refers to a local!
            .unwrap()
        // );
    }
}

#[with]
fn main ()
{
    hex_example();

    let n: &'ref str = ::core::u32::MAX.to_str();
    dbg!(n);
    assert_eq!(n.parse(), Ok(::core::u32::MAX));

    romans();
}

// below goes just a more advanced code, leet-challenge-like, of transforming
// a (small) integer into its roman numeral representation.
//
// The transformation itself isn't that interesting (unless you are curious!)
// but the fact that it is achieved both without (heap) allocations and without
// unsafe is ;)

#[with]
fn roman (mut n: u8) -> &'ref str
{
    if n == 0 {
        panic!("Vade retro!");
    }
    let mut buf = [b' '; 1 + 4 + 4]; // C LXXX VIII (or CC XXX VIII)
    let mut start = buf.len();
    let mut prepend = |b| {
        start = start.checked_sub(1).expect("Out of capacity!");
        buf[start] = b;
    };
    const DIGITS: [u8; 7] = *b"IVXLCDM";
    (0 ..= 2).for_each(|shift| {
        #[allow(nonstandard_style)]
        let (I, V, X) = (
            DIGITS[2 * shift],
            DIGITS[2 * shift + 1],
            DIGITS[2 * shift + 2],
        );
        match n % 10 {
            | i @ 0 ..= 3 => {
                (0 .. i).for_each(|_| prepend(I));
            },
            | 4 => {
                prepend(V);
                prepend(I);
            },
            | i @ 5 ..= 8 => {
                (0 .. (i - 5)).for_each(|_| prepend(I));
                prepend(V);
            },
            | 9 => {
                prepend(X);
                prepend(I);
            },
            | _ => unreachable!(),
        }
        n /= 10;
    });
    ::core::str::from_utf8(&buf[start ..]) // refers to a local!
        .unwrap()
}

#[with]
fn romans ()
{
    for n in 1 ..= ::core::u8::MAX {
        let s: &'ref str = roman(n);
        println!("{:3} = {}", n, s);
    }
}

/// Allow us to add comments and ignored code snippets anywhere
#[macro_export] macro_rules! no_run {($($tt:tt)*) => ()}
