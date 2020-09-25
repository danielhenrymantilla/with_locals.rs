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
    #
    impl Struct {
        fn with_bar<R> (
            self: &'_ Self,
            ret: impl for<'local> FnOnce(&'local Bar) -> R,
        ) -> R
        {
            let foo = &*self.foo.borrow();
            let bar = &*foo.bar.borrow();
            ret(bar)
        }
    }

    fn main ()
    {
        let s = Struct {
            foo: RefCell::new(Foo {
                bar: RefCell::new( Bar { /* ... */ }),
                // ...
            }),
            // ...
        };
        s.with_bar(|bar| {
            println!("bar = {:?}", bar);
        });
    }
