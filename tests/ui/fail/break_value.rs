include!("../prelude.rs");

#[with]
fn break_value ()
{
    #[with] fn f () -> &'ref () { &() }

    for n in 0 .. {
        let _: &'ref _ = f();
        if n >= 5 {
            break ();
        }
    }
}
