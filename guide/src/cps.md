# The core idea: Continuation-Passing Style

The reason why

```rust,compile_fail,editable
fn get_dangling_reference<'a> ()
  -> &'a i32
{
    let x = 0;
    &x
}
```

fails, is that we are asking the function to produce some kind of reference,
and in order to do so, the function needs to setup some local state.

This is a problem, since, although the function does reach, internally, a state
where it can easily manipulate that reference, it cannot, however,
**`return`** it. Indeed, a function, right before returning, needs to clean its
own ~~mess~~ stack / its own set of local variables.

To better illustrate this, let's inline the following call:

```rust,compile_fail
# use ::core::mem::drop as stuff;
fn caller ()
{
    let r = get_dangling_reference();
    stuff(r);
}
```

This, when inlined, becomes:

```rust,compile_fail
# use ::core::mem::drop as stuff;
fn caller ()
{
    //  +------------------------+
    //  | so that this reference |
    //  | is dangling as soon as |
    //  |       it exists.       |
    //  v                        |
    let r = {                 // |
        let x = 0;            // |
        &x                    // |
    }; // <- x is dropped here |-+
    stuff(r); // <- dangling!
}
```

Hence the problem.

> Well, that function / pattern is stupid anyways, why not return the value
> directly rather than a reference?

That may look like it, but this kind of situation does arise once we reach more
complex types, such as a type behind some `RefCell/RwLock` lock:

```rust,compile_fail
# use ::core::cell::RefCell;
# type Foo = (/* ... */);
#
struct Struct {
    foo: RefCell<Foo>,
}

impl Struct {
    fn foo (self: &'_ Self)
      -> &'_ Foo
    {
        &*self.foo.borrow()
    }
}
```

If you click on the <button class="fa fa-play play-button" hidden="" title="Run this code" aria-label="Run this code"></button> button above, you will stumble upon our good ol' `error[E0515]` yadda
yadda.

Indeed, our `x = 0` variable from before is now the `.foo.borrow()` [guard](
https://doc.rust-lang.org/core/cell/struct.Ref.html): that simple situation
did represent a valid use case!

> But wait, a clever usage of owned guards and destructors allow us to have
> the following pattern, so I don't see the problem:
>
> ```rust
> # use ::core::{cell::RefCell, ops::Deref};
> # type Foo = (/* ... */);
> #
> struct Struct {
>     foo: RefCell<Foo>,
> }
>
> impl Struct {
>     fn foo (self: &'_ Self)
>       -> impl '_ + Deref<Target = Foo> // this could be `Ref<'_, Field>`,
>                                        // but `impl` generalizes better (`RefCell` / `RwLock` agnostic)
>     {
>         self.foo.borrow()
>     }
> }
> ```

Well, that return type doesn't look super obvious, but fair enough, for that
use case any human can do the
`&'lifetime Type -> impl 'lifetime + Deref<Target = Type>`
transformation.

  - <details><summary>Heck, we can even define a macro for that!</summary>

    ```rust
    macro_rules! Pseudo {(
        & $lifetime:lifetime $Pointee:ty
    ) => (
        impl $lifetime + ::core::ops::Deref<Target = $Pointee>
    )}
    # use ::core::{cell::RefCell, ops::Deref};
    # type Foo = (/* ... */);
    #
    struct Struct {
        foo: RefCell<Foo>,
    }

    impl Struct {
        /// Borrow `.foo`
        fn foo (self: &'_ Self)
          -> Pseudo!(&'_ Foo)
        {
            self.foo.borrow()
        }
    }
    ```

    ___

    </details>


But things do get a bit uglier once we encounter the limitations of these guards,
that need to carry sub-borrows all within their own ownership.

For instance, what happens if `Foo` is a `struct` with some `bar` subfield we
wish to access

```rust
# use ::core::{cell::RefCell, ops::Deref};
# type Bar = (/* ... */);
#
struct Struct {
    foo: RefCell<Foo>,
}

struct Foo {
    bar: Bar,
    // ...
}

# macro_rules! Pseudo {(
#     & $lifetime:lifetime $Pointee:ty
# ) => (
#     impl $lifetime + ::core::ops::Deref<Target = $Pointee>
# )}
#
impl Struct {
    /// Borrow `.foo.bar`
    fn bar (self: &'_ Self)
      -> Pseudo!(&'_ Bar)
    {
        ::core::cell::Ref::map(
            self.foo.borrow(),
            |foo| &foo.bar,
        )
    }
}
```

Hmm, I think we can all agree this is becoming less and less natural, and more
and more contrived.

Time for the _coup de grâce_:

> _Quid_ of nested `RefCell`s (and the like)?

That is, let's imagine that in the situation above, we have `bar: RefCell<Bar>`
instead.

### Challenge: a single getter having to go through _two_ layers of `RefCell`

You will see this is **not possible to implement**,

  - unless you use some kind of self-referential wrapper that requires `unsafe`,
    or a crate that does this for you (such as [`::owning_ref`'s
    `OwningHandle`](
    https://docs.rs/owning_ref/0.4.1/owning_ref/struct.OwningHandle.html)).

    But these are still hard to generalize to even more complex and nested
    cases of referential structs, with the now added very real risk of Undefined
    Behavior, whereby hard-to-reproduce bugs may occur, or even security
    vulnerabilities may happen ⚠️

## Conclusion

> The case where a function wants to return data referring to a local is _real_.

___

So let's go back to our simple example:

```rust,compile_fail
# use ::core::mem::drop as stuff;
fn caller ()
{
    fn get_dangling_reference<'a> ()
      -> &'a i32
    {
        let x = 0;
        &x
    }

    let r = get_dangling_reference();
    stuff(r);
}
/// Inlined as:
# mod inlined { use ::core::mem::drop as stuff;
fn caller ()
{
    //  +------------------------+
    //  | so that this reference |
    //  | is dangling as soon as |
    //  |       it exists.       |
    //  v                        |
    let r = {                 // |
        let x = 0;            // |
        &x                    // |
    }; // <- x is dropped here |-+
    stuff(r); // <- dangling!
}
# }
```

Once we focus on the inlined case, we can come up with two easy workarounds:

### Option 1: store the local variables in the outer (caller) scope:

```rust,editable
# use ::core::mem::drop as stuff;
#
fn caller ()
{
    let x; // this local is dropped -+
    let r = {                     // |
        x = 0;                    // |
        &x                        // |
    };                            // |
    stuff(r) // does not dangle! 🙌  |
} // <-- here -----------------------+
```

This works, and is actually

<details><summary>a neat trick to keep up one's sleeve 😏</summary>

A very recurring situation in Rust is that of:

  - having an `Option` of an owned heap-allocated value, such as a `String` or
    a `Vec`,

  - and having a default value to be used for the `None` case that is **known
    at compile-time**, and which can thus **be allocated in `static` memory**.

Heap-allocating the latter is thus, when performance is important,
an anti-pattern.

```rust,editable
// Don't do this!
let name: String =
    ::std::env::var("NAME").ok()
        .unwrap_or("default_name".into()) // Heap-allocates even when present ;_;
;
// Slightly better, but is still nevertheless unnecessarily expensive
// in the missing case:
let name: String =
    ::std::env::var("NAME")
        .unwrap_or_else(|_| "default_name".into())
;
println!("name = {:?}", name);
```

Instead, the right way to save an allocation is by using something like
[`Cow`], such as
`type Str = Cow<'static, str>`:

[`Cow`]: https://doc.rust-lang.org/alloc/borrow/enum.Cow.html

```rust,editable
type Str = ::std::borrow::Cow<'static, str>;

let name: Str =
    ::std::env::var("NAME").ok()
        .map_or_else(|| "default_name".into(), Into::into)
;
println!("name = {:?}", name);
```

The issue with this pattern is that every `str` operation on our `Str` will
incur in a branching operation. Indeed, `Cow<'static, str>` `derefs` to `str`
by doing:

```rust,ignore,noplayground
match *self {
    | Cow::Borrowed(str /*: &'static str */) => str,
    | Cow::Owned(ref string /*: String */) => string.as_str(),
} // : &'_ str
```

So, to solve this, it is good practice to immediately perform the `.deref()`
once, early, to get the resulting `&'_ str` immediately:

```rust,editable
type Str = ::std::borrow::Cow<'static, str>;

let name: Str =
    ::std::env::var("NAME").ok()
        .map_or_else(|| "default_name".into(), Into::into)
;
let name: &'_ str = &*name;
println!("name = {:?}", name);
```

Well, the **trick** is that **one does not need `Cow` for the above, the same
can be achieved using long(er)-lived storage, through _delayed_ (and optional)
initialization**:

```rust,editable
let storage: String;
let name: &'_ str = match ::std::env::var("NAME").ok() {
    | Some(name) => { storage = name; storage.as_str() },
    | None => "default_name",
};
println!("{}", name);
```

The above trick has the advantage of supporting more complex types that [`Cow`]
does not support 😎.

___

</details>

But it does have drawbacks:

  - it requires that the caller know and hold storage for each and every local
    that the reference may (transitively) refer to;

  - it does not chain well: now if `caller()` has a parent caller / scope
    itself, we can't move the stuff anymore (in a way the set of locals + the
    reference represent a self-referential entity, making it "hard to move").

For what it's worth, this is how we'd translate the above pattern as a a helper
function:

```rust,editable
# use ::core::mem::drop as stuff;
#
fn caller ()
{
    let mut storage_for_x = None;
    let r = get_reference(&mut storage_for_x);
    stuff(r);
}
// where
fn get_reference<'x> (out_x: &'x mut Option<i32>)
  -> &'x i32
{
    debug_assert!(out_x.is_none());
    let at_x: &'x mut i32 = out_x.get_or_insert(0); // &mut x
    at_x
}
```

  - Needless to say, this does not look very nice, and can get very dirty, very
    quickly, once such functions are chained (plus, if logic bugs are involved,
    we may initialise `*out_x` multiple times and panic).

<details><summary>This pattern applied to the nested <code>RefCell</code> challenge</summary>

```rust
# use ::core::cell::{Ref, RefCell};

# struct Struct {
#     foo: RefCell<Foo>,
#     // ...
# }
#
# struct Foo {
#     bar: RefCell<Bar>,
#     // ...
# }
#
# #[derive(Debug)]
# struct Bar { /* ... */ }
#
impl Struct {
    fn bar<'_0, '_1, '_2> (
        self: &'_0 Self,
        foo_guard: &'_1 mut Option<  Ref<'_0, Foo>  >,
        bar_guard: &'_2 mut Option<  Ref<'_1, Bar>  >,
    ) -> &'_2 Bar
    where
        '_0 : '_1, // for `foo_guard`'s type to be well-formed.
        '_1 : '_2, // for `bar_guard`'s type to be well-formed.
    {
        let foo = foo_guard.get_or_insert(self.foo.borrow());
        let bar = bar_guard.get_or_insert(foo.bar.borrow());
        &*bar
    }
}

fn main ()
{
    let s = Struct {
        foo: RefCell::new(Foo {
            bar: RefCell::new(Bar { /* ... */ }),
            // ...
        }),
        // ...
    };
    {
        let mut slot1 = None;
        let mut slot2 = None;
        let bar = s.bar(&mut slot1, &mut slot2);
        println!("bar = {:?}", bar);
    }
}
```

  - [Playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=92eb4443f717636e93942d6cd02c3af4)


___

</details>

### Option 2: simply move the usage of the reference inside the inner scope

```rust,editable
fn caller ()
{
    let return_value_of_stuff = {
        let x = 0;
        let r = &x;
        stuff(r)
    };
}
```

> Duh! That's not a solution, that's skipping the problem altogether! You
> are changing the rules of the problem as you go!
>
> And how are you gonna translate that into a factored out function?

Well, in a way that's true. I am not providing an _exact_ solution for the
problem at hand, I am rather [XY](http://xyproblem.info/)-ing it. In a way,
the actual problem at hand is:

> **How do I factor out this snippet of code / logic into its own function
> without `cannot return value referencing a local value`  problems?**

And regarding the idea of factoring out that logic into its own function, it's
actually pretty easy, albeit cumbersome:

```rust,editable
# use ::core::mem::drop as stuff;
#
fn caller_1 ()
{
    let return_value_of_stuff = {
        let x = 0;
        let r = &x;
        stuff(r)
    };
}
// becomes
fn caller_2 ()
{
    let return_value_of_stuff = (|then_| {
        # let _: fn(&'_ i32) -> _ = then_;
        let x = 0;
        then_(&x)
    })(|r| {     // <-|
        stuff(r) //   |+-- this is `then_`
    });          // <-|
}
```

That is:

```rust,editable
fn caller ()
{
    fn with_local_reference<R> (
        /* args ..., */
        stuff_to_do_with_the_returned_value: impl FnOnce(&'_ i32) -> R,
    ) -> R
    {
        let x = 0;
        stuff_to_do_with_the_returned_value(&x)
    }

    let return_value_of_stuff = with_local_reference(|r| {
        // ...
        stuff(r)
    });
}
```

  - <details><summary>Alternatives names for the closure / callback / continuation</summary>

    ```rust,ignore,noplayground
    // another name:
    fn with_local_reference<R> (
        /* args ..., */
        f: impl FnOnce(&'_ i32) -> R,
    ) -> R
    {
        let x = 0;
        f(&x)
    }
    // another one:
    fn with_local_reference<R> (
        /* args ..., */
        ret: impl FnOnce(&'_ i32) -> R,
    ) -> R
    {
        let x = 0;
        ret(&x)
    }
    // another one:
    fn with_local_reference<R> (
        /* args ..., */
        with: impl FnOnce(&'_ i32) -> R,
    ) -> R
    {
        let x = 0;
        with(&x)
    }
    // technical name
    fn with_local_reference<R> (
        /* args ..., */
        continuation: impl FnOnce(&'_ i32) -> R,
    ) -> R
    {
        let x = 0;
        continuation(&x)
    }
    ```

    ___

    </details>

If you now stare at the two previous snippets long enough (inlined and factored
out versions), we can notice how "moving the logic into a part where the locals
are still alive", which was the simultaneously obvious and brilliant idea,
represents, when dealing with a factored out function, a shift between:

  - querying **the callee to give** us (the caller) a value **we can
    work** with,

  - _v.s._ **giving the callee** the **work** (with the value) we would have
    wished to run, so that **it is the callee who does the work** (instead of
    us).

Such a simple shift, but with so big consequences: since the callee gets to
work with the value before it has to `return` and clean up its own locals, it
gets to keep them, making everything _Just Work™️_.

<div style="width:100%;height:0;padding-bottom:60%;position:relative;"><iframe src="https://giphy.com/embed/2rqEdFfkMzXmo" width="100%" height="100%" style="position:absolute" frameBorder="0" class="giphy-embed" allowFullScreen></iframe></div><p><a href="https://giphy.com/gifs/stress-i-need-a-drink-brain-explode-2rqEdFfkMzXmo">via GIPHY</a></p>

This shift, when generalized as a language or just a programming pattern, is
called:

> [**Continuation-Passing Style** (CPS)](
    https://en.wikipedia.org/wiki/Continuation-passing_style)

  - ### Application to the nested `RefCell` challenge.

    ```rust,editable
{{#include snippets/cps-nested-refcell.rs}}
    ```

      - [Playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=00251031213759c82f789d8887b5eabc)


  - If you have never heard of it, you may, on the other hand, have heard of
    [_internal vs. external_ iteration](internal-vs-external-iteration.md).
