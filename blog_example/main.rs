#[macro_use]
extern crate with_locals;

use ::core::cell::RefCell;

#[with('_, continuation_name = ret)]
fn iter_refcells<Item> (
    refcells: &'_ [RefCell<Item>],
) -> &'_ mut dyn Iterator<Item = &'_ Item>
{
    let guards = refcells.iter().map(|it| it.borrow()).collect::<Vec<_>>();
    ret(guards.iter().map(|guard| &**guard).by_ref())
}

#[with]
fn sum<T : Default + for<'__> ::core::ops::Add<&'__ T, Output = T>> (
    refcells: &'_ [RefCell<T>],
) -> T
{
    let mut ret = T::default();
    #[with]
    let iter = iter_refcells::<T>(refcells);
    for x in iter {
        ret = ret + x;
    }
    ret
}

#[with]
fn main ()
{
    let refcells = &[RefCell::new(42), RefCell::new(27)];
    #[with] let iter = iter_refcells(refcells);
    dbg!(iter.fold(0, |x, y| x + y));
}






trait WithNext {
    type Item;

    #[with]
    fn next (self: &'_ mut Self) -> &'self Self::Item;
}

#[with]
while let Some(x) = iter.next() {

}

loop {
    #[with]
    if let Some(x) = iter.next() {
        continue;
        break ...;
        return ...;
        ...
    } else {
        break;
    }
}

match iter.with_next(|it| {
    if let Some(x) = it {
        ControlFlow::EvaluatedTo({
            return ControlFlow::Continue;
            return ControlFlow::Break(...);
            return ControlFlow::Return((...))
            ...
        })
    } else {
        ControlFlow::Break(None)
    }
})
{
    | ControlFlow::Break(None) => break,
    | ControlFlow::Break(Some(thing)) => break thing,
    | ControlFlow::Continue => continue,
    | ControlFlow::Return(thing) => thing,
}

#[with]
if let Some(x) = iter.next() {
    ...
} else {
    break;
}
