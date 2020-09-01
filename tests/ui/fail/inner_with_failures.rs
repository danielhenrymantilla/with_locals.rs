include!("../prelude.rs");

#[with]
fn stmt ()
{
    #[with]
    print!();
}

#[with]
fn expr ()
{
    let _ = #[with] expr();
}

#[with]
fn uncomplete_let ()
{
    #[with]
    let _incomplete;
}

#[with]
fn bad_rhs_of_let_not_a_function_name ()
{
    #[with]
    let _ = {foo}();
}

#[with]
fn bad_rhs_of_let_not_even_a_function_call ()
{
    #[with]
    let _ = 42;
}

#[with]
fn bad_rhs_has_attr ()
{
    #[with]
    let _ = #[extraneous] foo();
}
