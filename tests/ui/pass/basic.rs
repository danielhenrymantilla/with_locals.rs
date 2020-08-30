include!("../prelude.rs");

use ::core::fmt::Display;

#[with]
fn empty ()
{}

#[with]
fn returns_local (n: u32) -> &'self dyn Display
{
    &format_args!("{:#x}", n)
}

#[with]
fn uses_return_local (n: u32)
{
    #[with]
    let it: &dyn Display = returns_local(n);
    let _ = it.to_string();
}

#[with]
fn uses_return_local_and_returns_a_local_too (n: u32)
  -> &'self str
{
    #[with]
    let it: &dyn Display = returns_local(n);
    let s = it.to_string();
    &*s
}

#[with]
fn inside_if_uses_return_local_and_returns_a_local_too (n: u32)
  -> &'self str
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
  -> &'self str
{
    if true {
        #[with]
        let it: &dyn Display = returns_local(n);
        let s = it.to_string();
        return &*s;
    } else {
        return "";
    }
    ; // <- currently needed :(
    ""
}

#[with]
fn inside_match_uses_return_local_and_returns_a_local_too (n: u32)
  -> &'self str
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
