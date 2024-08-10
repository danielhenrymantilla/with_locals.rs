include!("../prelude.rs");

#[with('local)]
fn break_value ()
{
    #[with('local)] fn f () -> &'local () { &() }

    for n in 0 .. {
        let _: &'local _ = f();
        if n >= 5 {
            break ();
        }
    }
}
