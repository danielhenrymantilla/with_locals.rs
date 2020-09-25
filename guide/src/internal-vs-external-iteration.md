# _Addendum_ - Internal vs. External iteration

Granted, CPS may be a little unheard of, given that:

  - **garbage-collected languages** don't really have the concept of "locals",
    memory-wise, so no "value referencing a local" error;

  - **languages featuring [RAII] / destructors / finalizers / [drop glue]**
    manage to circumvent the issue for the most simple cases, which also
    happen to be the most frequent ones, hence rarely feeling the need for
    an apparently convoluted mechanic;

[RAII]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
[drop glue]: https://doc.rust-lang.org/core/mem/fn.needs_drop.html

  - **non-functional languages** (by that I mean languages not providing sugar
    for closures / lambdas), simply lack the expressivity or the minimum
    ergonomics to make working with continuations be a conceivable reality.

And I dare you find a language that does not belong to either of these three
categories.

But funnily enough, even these languages have felt the need for something akin
to CPS, when dealing with "returning" not _one_ value, but _several_:

> ![Iterators](assets/iterators.jpg)

Indeed, general (non-indexed, I mean), iteration is just a matter of API, and
that API can have one of two forms (I'll be using Rust for the code examples):

  - The [core / standard library `Iterator` trait](
      https://doc.rust-lang.org/core/iter/trait.Iterator.html):

    ```rust
    trait Iterator {
        type Item;

        /// Caller queries one item at a time; the callee **returns** it,
        /// so that the caller gets to do whatever it pleases.
        fn next (self: &'_ mut Self)
          -> Option<Self::Item>
        ;
    }
    ```

    > This is **external** iteration.

  - Any kind of API that offers `.for_each()`, `.try_fold()`, _etc._

    In practice, `.try_fold()` is sufficient to implement most of them, and
    could be seen as the representative of **internal** iteration:

    ```rust,ignore
    trait InternallyIterable : Sized {
        type Item;

        fn try_fold<Acc, Err, F> (
            self: Self,
            acc0: Acc,
            f: F
        ) -> Result<Acc, Err>
        where
            F : FnMut(acc, &'_ Self::Item) -> Result<Acc, Err>,
        ;

        fn for_each...

        ...
    }
    ```

      - <details><summary>Full trait</summary>

        ```rust,edition2018
        trait InternallyIterable : Sized {
            type Item;

            fn try_fold<Acc, Err, F> (
                self: Self,
                acc0: Acc,
                f: F,
            ) -> Result<Acc, Err>
            where
                F : FnMut(Acc, &'_ Self::Item) -> Result<Acc, Err>,
            ;

            fn fold<Acc, F> (
                self: Self,
                acc: Acc,
                mut f: F,
            ) -> Acc
            where
                F : FnMut(Acc, &'_ Self::Item) -> Acc,
            {
                use ::core::convert::Infallible;
                match
                    self.try_fold(
                        acc,
                        move |acc, item| Ok::<_, Infallible>(f(acc, item)),
                    )
                {
                    | Ok(acc) => acc,
                    | Err(infallible) => match infallible { /* ! */ },
                }
            }

            fn try_for_each<Err, F> (
                self: Self,
                mut f: F,
            ) -> Result<(), Err>
            where
                F : FnMut(&'_ Self::Item) -> Result<(), Err>,
            {
                self.try_fold((), move |(), item| f(item))
            }

            fn for_each<F> (
                self: Self,
                mut f: F,
            )
            where
                F : FnMut(&'_ Self::Item),
            {
                self.fold((), move |(), item| f(item))
            }
        }

        ```

          - [Playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=1222d9ab58b25b72235d20cd5853d658)

        ___

        </details>

      - <details><summary>Truly general definition using GAT</summary>

        ```rust
        trait InternallyIterable {
            type Item<'__>;

            fn try_fold<Acc, Err, F> (
                self: Self,
                acc0: Acc,
                f: F,
            ) -> Result<Acc, Err>
            where
                F : FnMut(Acc, Self::Item<'_>) -> Result<Acc, Err>,
            ;
        }
        ```

        ___

        </details>

      - <details><summary>Aside: iterator or iterable?</summary>

        We can also notice that with external iteration, there is a difference
        between being an `Iterator` or being _iterable_, _i.e._, being
        `IntoIterator`: one can directly call `.next()` on the former, whereas one
        needs to call (potential) setup / extra code on the latter, by calling
        `.into_iter()`.

        On the other hand, there is little difference between an internal iterator
        and an internally iterable. In both cases, the iterations happens within a
        single "shot", so if some setup code is needed, the iterable will be able
        to run it at the prelude of its `.try_fold()` function.

        ___

        </details>

Obviously, since `Iterator` already provides a `.try_fold()` method, we can
notice that external iteration is strictly stronger than internal iteration,
from the point of view of the caller. But that also means that from the point
of view of the implementor, it is harder to meet `Iterator`'s requirements than
`InternallyIterable`'s.

Indeed, try to solve the following challenge:

#### Challenge: provide a function to iterate over `&'_ Item`s out of a `&'_ [RefCell<Item>]`

  - <details><summary>(One) solution</summary>

    ```rust,ignore
    //! It is impossible to provide the following API:
    //! `&'_ [RefCell<T>] -> impl '_ + Iterator<Item = &'_ T>`

    /// However, one can trivially implement:
    impl<Item> InternallyIterable for &'_ [RefCell<Item>] {
        type Item = Item;

        fn try_fold<Acc, Err, F> (
            self: Self,
            mut acc: Acc,
            mut f: F,
        ) -> Result<Acc, Err>
        where
            F : FnMut(Acc, &'_ Self::Item) -> Result<Acc, Err>,
        {
            for refcell in self {
                acc = f(acc, &*refcell.borrow())?;
            }
            Ok(acc)
        }
    }
    ```

      - [Playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=5a8a63da3d1ed0fa4257b02a6996d1d2)

      - <details><summary>Aside: internal iteration and <code>yield</code> syntax</summary>

        see that `acc = f(acc, yielded_item)?;` line?

        We can factor that line within a `yield_!` macro, just for the kicks:

        ```rust,ignore
        fn try_fold<Acc, Err, F> (
            self: Self,
            mut acc: Acc,
            mut f: F,
        ) -> Result<Acc, Err>
        where
            F : FnMut(Acc, &'_ Self::Item) -> Result<Acc, Err>,
        {
            macro_rules! yield_ { ($value:expr) => ({
                acc = f(acc, &*refcell.borrow())?;
            })}

            for refcell in self {
                yield_!( &*refcell.borrow() );
            }

            Ok(acc)
        }
        ```

        Just saying...

        ___

        </details>
    ___

    </details>

## Conclusion

Did you notice how annoying it was to try to implement the

```text
&'_ [RefCell<T>] -> impl '_ + Iterator<Item = &'_ T>
```

API?

Indeed, we were hitting plenty of those _cannot return value
referencing a temporary value_ error. Whereas implementing internal iteration
was a breeze:

> everything worked in the intuitive manner we expected it to.

Now that you know of that, if you pay attention, you will notice some
situations in Rust where this kind of shift allows you to stop fighting Rust
and its "restrictive" ownership semantics. Internal iteration is just one case,
CPS is more of a generalization 😉.
