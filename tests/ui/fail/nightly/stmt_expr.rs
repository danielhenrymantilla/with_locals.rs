#![feature(proc_macro_hygiene, stmt_expr_attributes)]
include!("../../prelude.rs");

fn stmt ()
{
    #[with]
    print!();
}

fn expr ()
{
    let _ = #[with] expr();
}
