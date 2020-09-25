# The main problem: (bad) ergonomics.

Indeed, these `with_` / Continuation-Passing style is doubly cumbersome, both
for the caller and the callee:

  - ```rust
{{#include snippets/cps-nested-refcell.rs}}
    ```

### 1 - Cumbersome for the callee / the one defining the function

  - Indeed, all the shenanigans with the callback / closure / continuation add a
lot of noise to the code, drowning the meaningful info in it:

    ```rust,ignore
{{#include snippets/cps-nested-refcell.rs:16:26}}
    ```

  - Wouldn't it be better if our pseudo-return value was in a more
    "return value"-looking place?

    Something like:

    ```rust,ignore
    use ::with_locals::with;

    impl Struct {
        #[with('local, continuation_name = ret)]
        fn bar (self: &'_ Self) -> &'local Bar
        {
            let foo = &*self.foo.borrow();
            let bar = &*foo.bar.borrow();
            ret(bar)
        }
    }
    ```

___

But why stop there? Wouldn't it be better if we could "hide" the internal
continuation name, at least for the simple cases? Something that would
automatically inspect the "shape" of the code to replace explicit `return
<value>;` with `return ret(<value>);`.

Or go even further, replacing _implicit_ `return`s too!

```rust,ignore
use ::with_locals::with;

impl Struct {
    #[with('local)]
    fn bar (self: &'_ Self) -> &'local Bar
    {
        let foo = &*self.foo.borrow();
        let bar = &*foo.bar.borrow();
        bar
    }
}
```

Now we are talking! Now it looks like we are returning a _value referencing a
temporary_, and getting away with it! With no `unsafe` whatsoever!

![Ferris with sun glasses](assets/ferrisGlasses.png)

### 2 - Cumbersome for the caller

```rust,ignore
fn main ()
{
    let s1 = Struct { ... };
    let s2 = Struct { ... };
    s1.with_bar(|bar1| {
        s2.with_bar(|bar2| {
            assert_eq!(bar1, bar);
        })
    })
}
```

![Ferris eyes (skeptical)](assets/ferrisEyes.svg)

Well, let's not despair, since that's also something a procedural macro can
~~easily~~ edulcorate for us: _sweet_!

```rust,ignore
use ::with_locals::with;

#[with]
fn main ()
{
    let s1 = Struct { ... };
    let s2 = Struct { ... };
    #[with] let bar1 = s1.bar();
    #[with] let bar2 = s2.bar();
    assert_eq!(bar1, bar2);
}
```

  - <details><summary>Why the attribute on <code>main</code>?</summary>

    Well, for two reasons:

     1. Attributes on statements (such as a `let ...` binding) are _unstable_;

     1. And even if they weren't, such an attribute would only be able to
        transform that very statement, letting the rest of the block untouched.
        Which means we cannot implement the desired transformation.

    Indeed, the `#[with]` attribute, on a `let ...` binding statement, is
    expected to tranform:

    ```rust,ignore
    let foo = { ... };
    let bar = {
        ... // A = before the with, same scope
        #[with] let var = function(/* args */);
        ... // B = after the with, *same scope*
    };
    // C: after the with, outer scope
    let baz = { ... };
    ```

    into:

    ```rust,ignore
    let foo = { ... };
    let bar = {
        ... // A
        with_function(/* args */, |var| {
            ... // B
        })
    };
    // C
    let baz = { ... };
    ```

    So, as you can see, all the remainders of the block the `#[with]` statement
    is located in (`... // B`), needs to be moved inside that generate _ad-hoc_
    continuation closure, which thus requires the macro to be able to "butcher"
    these blocks as it sees fit. And the `#[with]` attribute applied to the
    `let` binding statement has no such power.

    To achieve that, we need an attribute or a macro taking, _at least_, both
    the `let` binding and the `... // B` remainder of the block.

    That is, something (an **extra macro**) located _at least_, at an _outer_
    scope. A _preprocessor_, we could say, that will inspect the inner code,
    looking for those `#[with] let ...` statements. At that point, it can
    apply the transformation, stripping, at the same time, the `#[with]`
    "attribute" itself (it turns out that the one located on `let`
    statements is thus not a true attribute, just a dummy syntactic quirk used
    to "mark" the `let ...` statements that the preprocessor needs to handle).

    > How "much outer"? How far?

    For a bunch of reasons, having it be an attribute macro annotating the
    function itself was the most appropriate choice.

    Indeed, it's "far enough" to cover all the statements located inside the
    function body; it is also convenient enough for the "preprocessor" to be
    merged with the other attribute (the one allowing `'self`-infected return
    values):

    ```rust,ignore
    use ::with_locals::with;
    use ::core::fmt::Display;

    #[with]
    fn to_str (x: i32) -> &'self str
    {
        ...
    }

    #[with]
    fn to_displayable (x: i32) -> &'self (dyn Display)
    {
        #[with] let s: &str = to_str(x);
        &s
    }
    ```

    </details>

___

## 🍬 Edulcorated code 🍬

```rust,ignore
use ::with_locals::with;
# use ::core::cell::RefCell;
#
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

impl Struct {
    #[with('local)]
    fn bar (self: &'_ Self) -> &'local Bar
    {
        &self.foo.borrow().bar.borrow()
    }
}

#[with]
fn main ()
{
    let s = Struct {
        # foo: RefCell::new(Foo {
        #     bar: RefCell::new( Bar { /* ... */ }),
        #     // ...
        # }),
        // ...
    };
    let bar: &'local Bar = s.bar();
    println!("bar = {:?}", bar);
}
