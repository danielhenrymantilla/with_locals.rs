# `::with_locals`

Let's start with a basic example: returning / yielding a `format_args` local.

```rust
#[with]
fn hex (n: u32) -> &'self dyn Display
{
    &format_args!("{:#x}", n)
}
```

The above becomes:

```rust
fn with_hex<R, F> (n: u32, f: F) -> R
where
    F : for<'any> FnOnce(&'any dyn Display) -> R,
{
    f(&format_args!("{:#x}", n))
}
```

`f: F`, here, is called a continuation:
instead of having a function return / yield some element / object,
the function takes, instead, the "logic" of what the caller would have liked
to do with that element (once it would have received it), so that it is the
callee who handles that object instead.

By shifting the logic like so, it is the callee and not the caller
who runs that logic, **which thus happens before the callee returns,
cleaning its locals and making things that refer to it dangle.**

> This is the whole point of all this strategy!

Now, to call / use the above function, one can no longer bind the "result"
of that function to a variable using a `let` binding, since that mechanism
is reserved for actual returns, and the actual code running in the caller's
stack.

Instead, one calls / uses that `with_hex` function using
closure / callback syntax:

```rust
with_hex(66, |s| {
    println!("{}", s);
})
```

This is extremely powerful, but incurs in a rightward drift everytime
such a binding is created:

```rust
with_hex(1, |one| {
    with_hex(2, |two| {
        with_hex(3, |three| {
            // ughhh ..
        })
    })
})
```

Instead, it would be nice if the compiler / the language provided a way
for `let` bindings to magically perform that transformation:

```rust
let one = hex(1);
let two = hex(2);
let three = hex(3);
```

Operating in this fashion is called Continuation-Passing Style, and
cannot be done implicitly in Rust.
But that doesn't mean one cannot get sugar for it.

## Enters `#[with]`!

```rust
#[with] let one = hex(1);
#[with] let two = hex(2);
#[with] let three = hex(3);
```

When applied to a function, it will tranform all its so-annotated
`let` bindings into nested closure calls, where all the statements that
follow the binding (within the same scope) are moved into the
continuation.

Here is an example:

```rust
#[with]
fn hex_example ()
{
    let s: String = {
        println!("Hello, World!");
        #[with] let s_hex = hex(66);
        println!("s_hex = {}", s_hex);
        let s = s_hex.to_string();
        assert_eq!(s, "0x42");
        s
    };
    assert_eq!(s, "0x42");
}
```

The above becomes:

```rust
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
```

Traits can have `#[with]`-annotated methods too.

```rust
trait ToStr {
    #[with]
    fn to_str (self: &'_ Self) -> &'self str
    ;
}
```

Example of a user of of the trait (≠ an implementor).

```rust
impl<T : ToStr> Display for __<T> {
    #[with] // you can #[with]-annotate classic function,
            // in order to get the `let` assignment magic :)
    fn fmt (self: &'_ Self, fmt: &'_ mut ::core::fmt::Formatter<'_>)
      -> ::core::fmt::Result
    {
        #[with]
        let s: &str = self.0.to_str();
        fmt.write_str(s)
    }
}
// (Using a newtype to avoid coherence issues)
struct __<T : ToStr>(T);
```

Example of an implementor:

```rust
impl ToStr for u32 {
    #[with('local)] // At any point, you can choose to use another name
                    // for the special lifetime that tells the attribute to
                    // transform the function into a `with_...` one.
                    // By default, that name is `'self`, since it is currently
                    // forbidden by the compiler, and I find it quite on point.
                    //
                    // But when `self` receivers are involved, this `'self`
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
```

See [`examples/main.rs`](https://github.com/danielhenrymantilla/with_locals.rs/blob/master/examples/main.rs)
for more detailed examples within a runnable file.
