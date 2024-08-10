#![forbid(unsafe_code)]

include!("../prelude.rs");

use ::core::fmt::Display;

#[with('local)]
fn empty ()
{}

#[with('local)]
fn returns_local (n: u32) -> &'local dyn Display
{
    &format_args!("{:#x}", n)
}

#[with('local)]
fn uses_returns_local (n: u32)
{
    #[with]
    let it: &dyn Display = returns_local(n);
    let _ = it.to_string();
}

#[with('local)]
fn uses_returns_local_and_returns_a_local_too (n: u32)
  -> &'local str
{
    #[with]
    let it: &dyn Display = returns_local(n);
    let s = it.to_string();
    &*s
}

#[with('local)]
fn inside_if_uses_return_local_and_returns_a_local_itself (n: u32)
  -> &'local str
{
    if true {
        #[with]
        let it: &dyn Display = returns_local(n);
        let s = it.to_string();
        &*s
    } else {
        ""
    }
}

#[with('local)]
fn inside_if_yadda_early_return (n: u32)
  -> &'local str
{
    if true {
        #[with]
        let it: &dyn Display = returns_local(n);
        let s = it.to_string();
        return &*s;
    } else {
        return "";
    }
    ""
}

#[with('local)]
fn inside_match_uses_return_local_and_returns_a_local_too (n: u32)
  -> &'local str
{
    match true {
        | true => {
            #[with]
            let it: &dyn Display = returns_local(n);
            let s = it.to_string();
            &*s
        },
        | _ => "",
    }
}

#[with('local)]
fn results ()
{
    #[with('local)]
    fn result () -> Result<&'local (), ()>
    {
        Err(())?;
        Ok(&())
    }

    let _ = (|| Ok::<(), ()>({
        #[with] let it = result();
        it?;
    }))();
}

const _: () = {
    enum Void {}
    type None = Option<Void>;

    #[with('local)]
    fn question_marks ()
      -> None
    {
        #[with('local)]
        fn options ()
          -> Option<Option<Option<&'local ()>>>
        {
            fn _item_inside_function_body ()
              -> Option<()>
            {
                None?;
                return None;
            }

            #[with]
            let _it = options()???;
            {
                fn _item_inside_trailing_stmts ()
                  -> Option<()>
                {
                    None?;
                    return None;
                }
            }

            None
        }

        #[with]
        let _it = options()???;
        #[with]
        let _snd = options()?;
        None
    }
};

#[with('local)]
fn loops ()
{
    #[with('local)] fn f () -> &'local () { &() }

    loop {
        let it: &'local () = f();
        if false { continue; }
        if false { break; }
        if false { break (); }
        if true { return; }
        let _ = (it, );
    }

    for _ in 0 .. {
        let it: &'local () = f();
        if false { continue; }
        if false { break; }
        // if false { break (); }
        if true { return; }
        let _ = (it, );
    }

    while false {
        let it: &'local () = f();
        if false { continue; }
        if false { break; }
        // if false { break (); }
        if true { return; }
        let _ = (it, );
    }

    while let 1 ..= 1 = 2 {
        let it: &'local () = f();
        if false { continue; }
        if false { break; }
        // if false { break (); }
        if true { return; }
        let _ = (it, );
    }
}
