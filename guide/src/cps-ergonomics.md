# The main problem: (bad) ergonomics.

Indeed, these `with_` / Continuation-Passing style is doubly cumbersome, both
for the caller and the callee:

  - ```rust
{{#include snippets/cps-nested-refcell.rs}}
    ```

### 1 - Cumbersome for the callee / the one defining the function

  - Indeed, all the shenanigans with the callback / closure / continuation add
    a lot of **noise** to the code, **drowning the meaningful info inside it**:

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

Now we are talking! Now it _looks like_ we are **returning a value referencing a
temporary**, and getting away with it! With no `unsafe` whatsoever!

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

#[with] // -------------------------------+
fn main ()                             // |
{                                      // |
    let s1 = Struct { ... };           // |
    let s2 = Struct { ... };           // |
    let bar1: &'ref Bar = s1.bar(); // <-+ Transforms let bindings with
    let bar2: &'ref _ = s2.bar();   // <-+ a special lifetime annotation.
    assert_eq!(bar1, bar2);
}
```

  - <details><summary>How does that work?</summary>

    The `#[with]` attribute, is expected to tranform:

    ```rust,ignore
    let foo = { ... };
    let bar = {
        ... // A: before the special let, same scope
        let var: ... 'special ... = function(/* args */);
        ... // B: after the special let, *same* scope
    };
    // C: after the special let, *outer* scope
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

    So, as you can see, all the remainders of the block the special `let` is
    located in (`... // B`), need to be moved inside that generated _ad-hoc_
    continuation closure, which thus requires the macro to be able to "butcher"
    these blocks as it sees fit.

    To achieve that, we need an attribute or a macro taking, _at least_, both
    the `let` binding and the `... // B` remainder of the block.

    That is, something (an **extra macro**) located _at least_, at an _outer_
    scope. A _preprocessor_, we could say, that will inspect the inner code,
    looking for those `let ...: 'special ... =` statements. At that point, it
    can apply the transformation, stripping, at the same time, the `'special`
    lifetime itself .

    > How "much outer"? How far?

    For a bunch of reasons, having it be an attribute macro annotating the
    function itself was the most appropriate choice.

    Indeed, it's "far enough" to cover all the statements located inside the
    function body; it is also convenient enough for the "preprocessor" to be
    merged with the other attribute (the one allowing to do CPS while mocking
    classic return values):

    ```rust,ignore
    use ::with_locals::with;
    use ::core::fmt::Display;

    #[with] // ------------++++ transforms the function into
            //             vvvv `with_to_str`, which takes a callback.
    fn to_str (x: i32) -> &'ref str
    {
        ...
    }

    #[with] // ---+----------------++++ ditto
            //    |                vvvv
    fn to_displayable (x: i32) -> &'ref (dyn Display)
    {   //        | also transforms this `let` into a call to `with_to_str`
        //      vvvv                                  that uses a callback
        let s: &'ref str = to_str(x);
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

#[with('local)]
fn main ()
{
    let s = Struct {
        # foo: RefCell::new(Foo {
        #     bar: RefCell::new( Bar { /* ... */ }),
        #     // ...
        # }),
        // ...
    };
    let bar: &'local _ = s.bar();
    println!("bar = {:?}", bar);
}
