extern crate proc_macro;

use ::proc_macro::{
    TokenStream,
};
use ::proc_macro2::{
    Span,
    TokenStream as TokenStream2,
};
use ::quote::{
    format_ident,
    quote,
    quote_spanned,
    ToTokens,
};
use ::syn::{*,
    parse::{
        // Nothing,
        Parse,
        Parser,
        ParseStream,
    },
    // punctuated::Punctuated,
    spanned::Spanned,
    Result,
    visit_mut::{self, VisitMut},
};

use ::core::{
    mem,
    ops::Not as _,
};

use self::{
    helpers::{Fields as __, LifetimeVisitor},
};

#[macro_use]
mod helpers;

include!("handle_returning_locals.rs");
mod handle_let_bindings;
mod parse;
mod wrap_statements_inside_closure_body;

enum Input {
    TraitItemMethod(TraitItemMethod),
    ImplItemMethod(ImplItemMethod),
    ItemFn(ItemFn),
}

type Str = ::std::borrow::Cow<'static, str>;

struct Attrs {
    lifetime: Str,
    continuation: Option<Ident>,
    recursive: bool,
}

/// See [the main documentation of the crate for info about this attribute](
/// https://docs.rs/with_locals).
#[proc_macro_attribute] pub
fn with (
    attrs: TokenStream,
    input: TokenStream,
) -> TokenStream
{
    let (ref attrs, ref mut fun) = (
        parse_macro_input!(attrs as Attrs),
        parse_macro_input!(input as Input),
    );

    let ty = handle_returning_locals(&mut *fun, attrs);
    if let Err(err) = handle_let_bindings::f(&mut *fun, attrs, ty) {
        return err.to_compile_error().into();
    }

    let ret = fun.to_token_stream();

    #[cfg(feature = "expand-macros")] {
        helpers::pretty_print_tokenstream(
            &ret,
            &fun.fields().sig.ident,
        );
    }

    ret.into()
}
