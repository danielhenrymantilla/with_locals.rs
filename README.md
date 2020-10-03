# `::with_locals`

Let's start with a basic example: returning / yielding a `format_args` local.

```rust
use ::core::fmt::Display;
use ::with_locals::with;

#[with('local)]
fn hex (n: u32) -> &'local dyn Display
{
    &format_args!("{:#x}", n)
}
```

The above becomes:

```rust
use ::core::fmt::Display;

fn with_hex <R, F> (n: u32, f: F) -> R
where           F : FnOnce(&'_     dyn Display) -> R,
 // for<'local> F : FnOnce(&'local dyn Display) -> R,
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
so before it cleans its locals and makes things that refer to it dangle.**

> This is the whole point of all this strategy!

Now, to call / use the above function, one can no longer bind the "result"
of that function to a variable using a `let` binding, since that mechanism
is reserved for actual returns, and the actual code running in the caller's
stack.

Instead, one calls / uses that `with_hex` function using
closure / callback syntax:

```rust,ignore
with_hex(66, |s| {
    println!("{}", s);
})
```

This is extremely powerful, but incurs in a rightward drift everytime
such a binding is created:

```rust,ignore
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

```rust,ignore
let one = hex(1);
let two = hex(2);
let three = hex(3);
```

Operating in this fashion is called Continuation-Passing Style, and
cannot be done implicitly in Rust.
But that doesn't mean one cannot get sugar for it.

## Enters `#[with]`!

```rust,ignore
#[with] let one = hex(1);
#[with] let two = hex(2);
#[with] let three = hex(3);
```

  - This can also be written as:

    ```rust,ignore
    let one: &'ref _ = hex(1);
    let two: &'ref _ = hex(2);
    let three: &'ref _ = hex(3);
    ```

    That is, `let` bindings that feature a ["special lifetime"].

When applied to a function, it will tranform all its so-annotated
`let` bindings into nested closure calls, where all the statements that
follow the binding (within the same scope) are moved into the
continuation.

Here is an example:

```rust
# use ::with_locals::with; #[with] fn hex (n: u32) -> &'ref dyn ::core::fmt::Display { &format_args!("{:#x}", n) }
#
#[with]
fn hex_example ()
{
    let s: String = {
        println!("Hello, World!");
        #[with]
        let s_hex = hex(66);
        println!("s_hex = {}", s_hex); // Outputs `s_hex = 0x42`
        let s = s_hex.to_string();
        assert_eq!(s, "0x42");
        s
    };
    assert_eq!(s, "0x42");
}
```

The above becomes:

```rust
# use ::with_locals::with; #[with] fn hex (n: u32) -> &'ref dyn ::core::fmt::Display { &format_args!("{:#x}", n) }
#
fn hex_example ()
{
    let s: String = {
        println!("Hello, World!");
        with_hex(66, |s_hex| {
            println!("s_hex = {}", s_hex); // Outputs `s_hex = 0x42`
            let s = s_hex.to_string();
            assert_eq!(s, "0x42");
            s
        })
    };
    assert_eq!(s, "0x42");
}
```

#### Trait methods

Traits can have `#[with]`-annotated methods too.

```rust,ignore
# use ::with_locals::with;
#
trait ToStr {
    #[with('local)]
    fn to_str (self: &'_ Self) -> &'local str
    ;
}
```

Example of an implementor:

```rust
# use ::with_locals::with; trait ToStr { #[with] fn to_str (self: &'_ Self) -> &'ref str ; }
#
impl ToStr for u32 {
    #[with('local)]
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
        let mut cursor = buf.len();
        while x > 0 {
            cursor -= 1;
            buf[cursor] = b'0' + (x % 10) as u8;
            x /= 10;
        }
        // return f(
        ::core::str::from_utf8(&buf[cursor ..]) // refers to a local!
            .unwrap()
        // );
    }
}
# #[with]
# fn main ()
# {
#     let s: &'ref str = 42.to_str();
#     assert_eq!(s, "42");
# }
```

Example of a user of the trait (≠ an implementor).

```rust
# use ::with_locals::with; trait ToStr { #[with] fn to_str (self: &'_ Self) -> &'ref str ; }
#
impl<T : ToStr> ::core::fmt::Display for __<T> {
    #[with] // you can #[with]-annotate classic function,
            // in order to get the `let` assignment magic :)
    fn fmt (self: &'_ Self, fmt: &'_ mut ::core::fmt::Formatter<'_>)
      -> ::core::fmt::Result
    {
        //      You can specify the
        //      special lifetime instead of applying `[with]`
        //      vvvv
        let s: &'ref str = self.0.to_str();
        fmt.write_str(s)
    }
}
// (Using a newtype to avoid coherence issues)
struct __<T : ToStr>(T);
```

See [`examples/main.rs`](https://github.com/danielhenrymantilla/with_locals.rs/blob/master/examples/main.rs)
for more detailed examples within a runnable file.

<span id="special-lifetime"></span>

## Usage and the "Special lifetime".

["special lifetime"]: #special-lifetime

Something important to understand _w.r.t._ how `#[with]` operates, is that
sometimes it must perform transformations (such as changing a `foo()` call into
a `with_foo(...)` call), and sometimes it must not; it depends on the semantics
the programmer wants to write (that is, not _all_ function calls rely on CPS!).
Since _a procedural macro only operates on syntax_, it cannot understand such
_semantics_ (_e.g._, it is not possible for a proc-macro to replace `foo()`
with `with_foo()` if, and only if, `foo` does not exist). Because of that,
**the macro expects some syntactic marker / hints that tell it when (and
where!) to work**:

 1. Obviously, the attribute itself needs to have been applied,

    _on the enscoping function_:

    ```rust,ignore
    #[with('special)]
    fn ...
    ```

      - If no override is provided, `#[with]` defaults to `#[with('ref)]`.

 1. Then, the macro will inspect to see if **there is a ["special lifetime"]
    within the return type of the function**.

    ```rust,ignore
    //        +-------------+
    //        |             |
    //     --------         V
    #[with('special)] // vvvvvvvv
    fn foo (...)   -> ...'special...
    ```

    That will trigger the transformation of `fn foo` into `fn with_foo`, with
    all the taking-a-callback-parameter shenanigans.

    Otherwise _it won't change the prototype of the function_.

 1. Finally, the macro will also inspect the function body, to perform the
    call-site transformations (_e.g._, `let x = foo(...)` into
    `with_foo(..., |x| { ... })`).

    These transformations are only applied:

      - On the `#[with]`-annotated statements: `[with] let ...`;

      - _Or_, on the statements carrying a type annotation that mentions the
        ["special lifetime"]:

        ```rust,ignore
        let x: ... 'special ... = foo(...);
        ```

### Remarks

  - By default, the ["special lifetime"] is `'ref`. Indeed, since `ref` is a
    Rust keyword, it is not a legal lifetime name, so it is impossible for it
    to conflict with some real lifetime parameter equally named.

  - But `#[with]` allows you to rename that lifetime to one of your liking, by
    providing it as the first parameter of the attribute (the one applied to
    the function, of course):

    ```rust
    use ::core::fmt::Display;
    use ::with_locals::with;

    #[with('local)]
    fn hello (name: &'_ str) -> &'local dyn Display
    {
        &format_args!("Hello {}!", name)
    }
    ```

## Advanced usage

If you are well acquainted with all this CPS / callback style, and would just
like to have some sugar when defining callback-based functions, but do not want
the attribute to mess up with the code inside the function body (_i.e._, if
you want to opt-out of the magic continuation calls at `return` sites _& co._),
for instance, because you are interacting with other macros (since they lead to
opaque code as far as `#[with]` is concerned, making it unable to "fix" the
code inside, which may lead to uncompilable code), then, know that you can:

  - directly call the `with_foo(...)` functions with hand-written closures.

    This is kind of obvious given how the functions end up defined, and is
    definitely a possibility that should not be overlooked.

  - and/or you can add a `continuation_name = some_identifier` parameter to the
    `#[with]` attribute to disable the automatic `return continuation(<expr>)`
    transformations;

      - Note that `#[with]` will then provide a `some_identifier!` macro that
        can be used as a shorthand for `return some_identifier(...)`. This is
        especially neat if the identifier used is, for instance, `return_`.
        You can then write `return_! { value }` where a classic function would
        have written `return value`, and it will correctly expand to
        `return return_(value)` (return the value returned by the continuation).

#### Example

```rust
use ::core::fmt::Display;
use ::with_locals::with;

#[with(continuation_name = return_)]
fn display_addr (addr: usize) -> &'ref dyn Display
{
    if addr == 0 {
        return_!( &"NULL" );
    }
    with_hex(addr, |hex| {
        return_(&format_args!("0x{}", hex))
    })
}
// where
#[with]
fn hex (n: usize) -> &'ref dyn Display
{
    &format_args!("{:x}", n)
}
```

## Powerful unsugaring

Since some statements are wrapped inside closures, that basic transformation
alone would make control flow statements such as `return`, `?`, `continue` and
`break` to stop working when located in the scope of a `#[with] let ...`
statement (after it).

```rust,compile_fail
use ::core::fmt::Display;
use ::with_locals::with;

#[with]
fn hex (n: u32) -> &'ref dyn Display
{
    &format_args!("{:#x}", n)
}

fn main ()
{
    for n in 0 .. { // <- `break` cannot refer to this:
        with_hex(n, |s| { // === closure boundary ===
            println!("{}", s);     // ^ Error!
            if n >= 5 {            // |
                break; // ------------+
            }
        })
    }
}
```

And yet, when using the `#[with] let` sugar the above pattern seems to work:

```rust
use ::core::fmt::Display;
use ::with_locals::with;

#[with]
fn hex (n: u32) -> &'ref dyn Display
{
    &format_args!("{:#x}", n)
}

#[with]
fn main ()
{
    for n in 0 .. {
        #[with]
        let s = hex(n);
        println!("{}", s);
        if n >= 5 {
            break;
        }
    };
}
```

  - <details><summary>Click here to see how this is done</summary>

    This is achieved by bundling the expected control flow information within
    the return value of the provided closure:

    ```rust,ignore
    for n in 0 .. {
        use ::with_locals::...::ControlFlow; // helper `enum`.

        match with_hex(n, |s| ControlFlow::Eval({
            println!("{}", s);
            if n >= 5 {
                return ControlFlow::Break;
            }
        }))
        {
            ControlFlow::Eval(it) => it,
            ControlFlow::Break => break,
        }
    }
    ```

    ___

    </details>

### Debugging / Macro expansion

If, for some reason, you are interested in seeing what's the actual code
generated / emitted by a `#[with]` attribute invocation, then all you have to
do is to enable the `expand-macros` Cargo feature:

```toml
[dependencies]
# ...
with_locals = { version = "...", features = ["expand-macros"] }
```

This will display the emitted code with a style very similar to `cargo-expand`,
but with two added benefits:

  - It does not expand _all_ the macros, just the `#[with]` one. So, if within
    the body of a function there is something like a `println!` call, the
    actual internal formatting logic / machinery will remain hidden and not
    clobber the code.

  - Once the Cargo feature is enabled, a special env var can be used to
    **filter the desired expansions**:

    ```bash
    WITH_LOCALS_DEBUG_FILTER=pattern cargo check
    ```

      - This will then only display the expansions for functions whose name
        _contains_ the given pattern. Note that this does _not_ involve the
        fully qualified name (with the outer modules), it's the bare name only.

  - That being said, this only works when the procedural macro is evaluated,
    and `rustc` will try to cache the result of such invocations. If that's the
    case, all you have to do is perform some dummy change within the involved
    file, and _save_.
