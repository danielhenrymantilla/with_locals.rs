# Applications

## 1 - "Returning" trait objects (`dyn Trait`) _without_ (heap) allocations

### 1.1 - Lazy `String` formatting and `dyn/impl Display`

Imagine wanting to have a function to wrap an object with some metadata that
tells it how it should be formatted. In other words, a **lazily** formatted
`String`:

```diff
  fn hello_hex (n: u64)
-   -> String
+   -> impl Display
  {
-     format!("Hello, {:#x}!", n)
+     format_args!("Hello, {:#x}!", n)
  }
```

Sadly the intuitive approach fails here (click on the <button class="fa fa-play
play-button" hidden="" title="Run this code" aria-label="Run this code">
</button> below to see the error).

```rust_compile_fail,editable
use ::core::fmt::Display;

fn hello_hex (n: u64)
  -> impl Display
{
    format_args!("Hello, {:#x}!", n)
}

fn main ()
{}
```

Granted, there are [some](https://docs.rs/lazy_format/1.8.3/lazy_format/)
[crates](https://docs.rs/join-lazy-fmt/0.9.2/join_lazy_fmt/) out there that are
specialized for this task (with the main "advantage" of using static dispatch
for maximally fast (and bloated!) binaries).

```rust,ignore
use ::lazy_format::lazy_format;
# use ::core::fmt::Display;

fn hello_hex (n: u64)
  -> impl Display
{
    lazy_format!("Hello, {:#x}!", n)
}
```

> But wouldn't it be simpler if we could "just" **return `format_args!`**?

For sure, it would, but the current implementation of `format_args!` generates
ephemeral temporaries, which makes even just _holding_ onto such a value within
a function body become a nightmare!

For instance, just try and see how absurdly difficult the following challenge
is:

```rust,compile_fail,editable
// EDITABLE CHALLENGE: bind a `format_args!` call to
// a variable name, and then use it:

fn main ()
{
    let x = 42;
    let s = format_args!("{}", x);
    println!("{}", s);
}
```

  - <details><summary>Solutions</summary>

    ```rust,editable
    fn main ()
    {
        let x = 42;
        match format_args!("{}", x) { s => {
            println!("{}", s);
        }}
    }
    ```

    ```rust,editable
    fn main ()
    {
        let x = 42;
        fun(format_args!("{}", x));
        // where
        fn fun (s: impl ::core::fmt::Display)
        {
            println!("{}", s);
        }
    }
    ```

    </details>

And, of course, the specialized crates may be good to handle _that_ particular
problem of returning _ad-hoc_ `Display`ables, but what about other _similar
but not quite equal_ situations? Isn't the whole point of software
_engineering_, to _design_ the right abstractions to avoid code duplication?
To _factor_ out code? Then these specialized solutions are flawed by their very
excessive specialization! Too niche! For sure, nothing really worth of a
blog-post.

  - <details><summary>For those curious</summary>

    `Display` features the very same API that a

    ```rust,ignore
                 Fn  (&mut fmt::Formatter<'_>) -> fmt::Result
    //           ^^
    //         vvvvv
    // fn fmt (&self, &mut fmt::Formatter<'_>) -> fmt::Result
    ```

    closure does, and we can generate _ad-hoc_ instances and implementations of the
    latter.

    So the only thing to do is to provide:

      - a new-`type` bridging those two APIs together (to handle the
        implementation part),

      - and a macro call (to handle the instanciation).

    ___

    </details>

So, back to our objective, the more idiomatic tool here is to have a solution
that would support, for instance, the _ad-hoc_ instances that `format_args!`
generates (more "idiomatic" than an external crate):

```rust,ignore
# use ::core::fmt::Display;
#
#[with('ref)]
fn hello_hex (n: 64)
  -> impl 'ref + Display
{
    format_args!("Hello, {:#x}!", n)
}
```

as well as `dyn Trait`s, such as `dyn Display` here (_e.g._ imagine that
function being within a trait):

```rust,ignore
# use ::core::fmt::Display;
#
#[with]
fn hello_hex (n: 64)
  -> &'ref (dyn Display)
{
    &format_args!("Hello, {:#x}!", n)
}
```

  - and for a caller wanting an _owned_ `String`, one can always go from

    [`&'_ (impl ?Sized + Display)` to a `String` thanks to `ToString`](
    https://doc.rust-lang.org/1.46.0/std/string/trait.ToString.html#impl-ToString-4):

    ```rust,ignore
    #[with]
    fn hello_hex_owned (n: u64)
      -> String
    {
        let s: &'ref (dyn Display) = hello_hex(n);
        s.to_string()
    }

    assert_eq!(hello_hex_owned(66), "Hello, 0x42!");
    ```

___

Following from the "returning" a `dyn Trait` topic (we've just seen
`&dyn Trait`), here comes:

### 1.2 - "Return" a `FnMut` within a _trait_ method

Indeed, remember that the following kind of trait definitions fails:

```rust,compile_fail
trait GetHandler {
    type Item;

    fn handler (self: &'_ mut Self)
      -> impl '_ + FnMut(Self::Item)
    ;
}
```

That's because [existential types (`-> impl Trait`) are ambiguous when used
within a trait](https://users.rust-lang.org/t/rewriting-loops-containing-as-chains-of-iterators/48962/15?u=yandros).
Sometimes, an extra associated type can help circumvent the issues with
existential types:

```rust,ignore
trait GetHandler {
    type Item;
    // Generic associated type (GAT):
    type Handler<'__> : '__ + FnMut(Self::Item);

    fn handler (self: &'_ mut Self)
      -> Self::Handler<'_>
    ;
}
```

But, on top of requiring GATs,

  - <details>
    <summary>
    or the "infect the trait with the lifetime of the method" workaround
    </summary>

    ```rust
    trait GetHandler<'lt> {
        type Item;
        type Handler : 'lt + FnMut(Self::Item);

        fn handler (self: &'lt mut Self)
          -> Self::Handler
        ;
    }
    ```

    which seems nice, except when users now have to use the
    `for<'any> GetHandler<'any>` bound instead of the more natural `GetHandler`.

    ___

    </details>

this solution becomes unusable when dealing with unnameable types, such as when
closures or `async` blocks are involved:

```rust,ignore
impl GetHandler for Foo {
    type Item = ...;

    fn handler (self: &'_ mut Self)
      -> Self::Handler<'_>
    {
        |item: Self::Item| { ... }
    }

    type Handler<'__> = ??????;
}
```

  - That being said, there is a feature in the works that would enable
    expressing existential types with more control over the quantifiers /
    the types and lifetimes the existential type may be allowed to depend on,
    thus solving the issue with the ambiguities of the current syntax.
    See the aforementioned post for more info:

    <details><summary>Example</summary>

    ```rust,editable
    #![feature(
        generic_associated_types,
        type_alias_impl_trait,
    )]
    #![allow(incomplete_features)] // 😬

    trait GetHandler {
        type Item;

        fn with_handler (
            self: &'_ mut Self,
        ) -> Self::Handler<'_>
        ;

        type Handler<'__> : FnMut(Self::Item);
    }

    struct Adder {
        sum: i64,
    }

    impl GetHandler for Adder {
        type Item = i64;

        fn with_handler (
            self: &'_ mut Self,
        ) -> Self::Handler<'_>
        {
            move |x| {
                self.sum += x;
            }
        }

        type Handler<'__> = impl FnMut(i64);
    }

    fn main ()
    {}
    ```

    ___

    </details>

This is when trait objects and their virtual (vtable method lookup) dispatch
become a handy workaround:

`dyn FnMut(Self::Item)`

but ...

... trait objects require a level of indirection, such as:

  - `Box<dyn FnMut...>`, which requires a heap allocation 😭

  - `&'_ mut (dyn FnMut...)`, which **does not require a heap allocation** 🎉
    (since replaced with a stack / local allocation), <details><summary>but does require ...</summary>

    ... being able to return a local!

    </details>

    ```rust,compile_fail,editable
    trait GetHandler {
        type Item;

        fn handler (self: &'_ mut Self)
          -> &'_ mut (dyn FnMut(Self::Item))
        //    ^^
        //    wops, wrong lifetime!
        ;
    }

    struct Adder {
        sum: i64,
    }
    impl GetHandler for Adder {
        type Item = i64;

        fn handler (self: &'_ mut Adder)
          -> &'_ mut (dyn FnMut(i64))
        //    ^^
        //    wops, wrong lifetime!
        {
            &mut |x: i64| {
                self.sum += x
            }
        }
    }

    fn main ()
    {}
    ```

    If only we had a tool to "return" data referring to our own stack-allocated
    data 🤔🤔... 😁

    ```rust,ignore
    use ::with_locals::with;

    trait GetHandler {
        type Item;

        #[with]
        fn handler (self: &'_ mut Self)
          -> &'ref mut (dyn FnMut(Self::Item))
        //    ^^^^
        //     👌
        ;
    }

    struct Adder {
        sum: i64,
    }
    impl GetHandler for Adder {
        type Item = i64;

        #[with]
        fn handler (self: &'_ mut Adder)
          -> &'ref mut (dyn FnMut(i64))
        {
            &mut |x: i64| {
                self.sum += x
            }
        }
    }
    ```

      - [Playground (with unsugared code): It Just Works™](https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=f5315d43f08b7653b6e2d133a6eeb26a)

# Also mention ad-hoc wrapper types with destructors

# And self-referential types
