# Continuation-Passing Style, or:

# How I Learned to Stop Worrying and Return Data Referring to Locals

This is a tale about an oh so well-known compilation error:

> error[\[E0515\]]: cannot return value referencing temporary value

[\[E0515\]]: https://doc.rust-lang.org/stable/error-index.html#E0515

```text
  --> src/lib.rs:22:9
   |
22 |         &self.bar.borrow().baz.borrow().some_field
   |         ^------------------------------^^^^^^^^^^^
   |         ||
   |         |temporary value created here
   |         returns a value referencing data owned by the current function

```

Let's start with the basic code examples associated with [the explanation of
that error code][\[E0515\]]:

```rust,editable,compile_fail
fn get_dangling_reference<'a> ()
  -> &'a i32
{
    let x = 0;
    &x
}

fn get_dangling_iterator<'a> ()
  -> ::std::slice::Iter<'a, i32>
{
    let v = vec![1, 2, 3];
    v.iter()
}

fn main ()
{}
```

Across this series of posts we will see how I managed to feature a
procedural macro to allow writing that very pattern, all **without any `unsafe`
code whatsoever**:

```rust,ignore,noplayground
# #[macro_use] extern crate with_locals;
#
#[with]
fn local_reference ()
  -> &'ref i32
{
    let x = 0;
    &x
}

#[with]
fn local_iterator ()
  -> ::std::slice::Iter<'ref, i32>
{
    let v = vec![1, 2, 3];
    v.iter()
}

/// And an example of caller code:
#[with]
fn main ()
{
    #[with]
    let elems = local_iterator();
    for elem in elems {
        #[with]
        let x: &i32 = local_reference();
        println!("{}, {}", x, elem);
    }
}
```

> But how?

For that, let's see [the following chapter](./cps.md).
