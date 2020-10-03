#![forbid(unsafe_code)]

include!("../prelude.rs");

use ::core::fmt::Display;

#[with]
fn empty ()
{}

#[with]
fn returns_local (n: u32) -> &'ref dyn Display
{
    &format_args!("{:#x}", n)
}

#[with]
fn uses_returns_local (n: u32)
{
    #[with]
    let it: &dyn Display = returns_local(n);
    let _ = it.to_string();
}

#[with]
fn uses_returns_local_and_returns_a_local_too (n: u32)
  -> &'ref str
{
    #[with]
    let it: &dyn Display = returns_local(n);
    let s = it.to_string();
    &*s
}

#[with]
fn inside_if_uses_return_local_and_returns_a_local_itself (n: u32)
  -> &'ref str
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

#[with]
fn inside_if_yadda_early_return (n: u32)
  -> &'ref str
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

#[with]
fn inside_match_uses_return_local_and_returns_a_local_too (n: u32)
  -> &'ref str
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

#[with]
fn results ()
{
    #[with]
    fn result () -> Result<&'ref (), ()>
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

    #[with]
    fn question_marks ()
      -> None
    {
        #[with]
        fn options ()
          -> Option<Option<Option<&'ref ()>>>
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
        None
    }
};

#[with]
fn loops ()
{
    #[with] fn f () -> &'ref () { &() }

    loop {
        let it: &'ref () = f();
        if false { continue; }
        if false { break; }
        if false { break (); }
        if true { return; }
        drop(it);
    }

    for _ in 0 .. {
        let it: &'ref () = f();
        if false { continue; }
        if false { break; }
        // if false { break (); }
        if true { return; }
        drop(it);
    }

    while false {
        let it: &'ref () = f();
        if false { continue; }
        if false { break; }
        // if false { break (); }
        if true { return; }
        drop(it);
    }

    while let 1 ..= 1 = 2 {
        let it: &'ref () = f();
        if false { continue; }
        if false { break; }
        // if false { break (); }
        if true { return; }
        drop(it);
    }
}
