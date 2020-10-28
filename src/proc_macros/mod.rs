#![allow(nonstandard_style)]

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
    punctuated::Punctuated,
    spanned::Spanned,
    visit_mut::{self, VisitMut},
};

type Result<Ok, Err = Error> = ::core::result::Result<Ok, Err>;

use ::core::{
    mem,
    ops::Not as _,
};

use self::{
    helpers::{Fields as __, FnLike, LifetimeVisitor},
};

#[macro_use]
mod helpers;

mod attrs;
include!("handle_returning_locals.rs");
mod handle_let_bindings;
mod wrap_statements_inside_closure_body;

type Str = ::std::borrow::Cow<'static, str>;

use attrs::Attrs;

/// See [the main documentation of the crate for info about this attribute](
/// https://docs.rs/with_locals).
#[proc_macro_attribute] pub
fn with (
    attrs: TokenStream,
    input: TokenStream,
) -> TokenStream
{
    let ref attrs = parse_macro_input!(attrs as Attrs);
    #[cfg(feature = "expand-macros")]
    let mut name = String::new();
    match parse::<TraitItemMethod>(input.clone()) {
        | Ok(mut method) => {
            #[cfg(feature = "expand-macros")] {
                name = method.sig.ident.to_string();
            }
            handle_fn_like(attrs, &mut method, None)
                .map(|()| method.into_token_stream())
        },
        | Err(_) => match parse(input) {
            | Ok(Item::Impl(item)) => {
                #[cfg(feature = "expand-macros")] {
                    name = item.self_ty.to_token_stream().to_string();
                }
                with_impl(attrs, item)
            },
            | Ok(Item::Trait(item)) => {
                #[cfg(feature = "expand-macros")] {
                    name = item.ident.to_string();
                }
                with_trait(attrs, item)
            },
            | Ok(Item::Fn(mut fun)) => {
                #[cfg(feature = "expand-macros")] {
                    name = fun.fields().sig.ident.to_string();
                }
                handle_fn_like(attrs, &mut fun, None)
                    .map(|()| fun.into_token_stream())
            },
            | _otherwise => Err(Error::new(Span::call_site(), "\
                `#[with]` can only be applied to \
                an `fn`, a `trait`, or an `impl`.\
            ")),
        },
    }
    .map_or_else(
        |err| err.to_compile_error().into(),
        |ret| {
            #[cfg(feature = "expand-macros")] {
                helpers::pretty_print_tokenstream(
                    &ret,
                    &name,
                );
            }

            ret.into()
        },
    )
}

fn handle_fn_like<Fun : FnLike> (
    attrs: &'_ Attrs,
    fun: &'_ mut Fun,
    outer_scope: Option<(&'_ Generics, ::func_wrap::ImplOrTrait<'_>)>
) -> Result<()>
{
    handle_returning_locals(fun, attrs, outer_scope)?;
    if let Some(block) = fun.fields().block {
        handle_let_bindings::f(block, attrs)?;
    }
    Ok(())
}

fn with_impl (outer_with_attrs: &'_ Attrs, mut impl_: ItemImpl)
  -> Result<TokenStream2>
{
    let outer_scope = (
        &impl_.generics,
        ::func_wrap::ImplOrTrait::ImplMethod {
            implementor: &impl_.self_ty,
            trait_name: impl_.trait_.as_ref().map(|(_, it, _)| it)
        },
    );
    impl_.items.iter_mut().try_for_each(|it| match it {
        | &mut ImplItem::Method(ref mut method) => {
            let mut attr = None;
            let mut err = None;
            method
                .attrs
                .retain(|cur_attr| if cur_attr.path.is_ident("with") {
                    let prev = attr.replace(cur_attr.clone());
                    if let Some(prev) = prev {
                        err = Some(Error::new_spanned(prev,
                            "Duplicate `#[with]` attribute",
                        ));
                    }
                    false // remove the attribute
                } else {
                    true
                })
            ;
            if let Some(err) = err { return Err(err); }
            let storage;
            let attrs: &Attrs = match attr {
                Some(attr) => {
                    storage = attr.parse_args::<Attrs>()?;
                    &storage
                },
                None => outer_with_attrs,
            };
            handle_fn_like(&attrs, method, Some(outer_scope))
        },
        | _ => Ok(()),
    })?;
    Ok(impl_.into_token_stream())
}


fn with_trait (outer_with_attrs: &'_ Attrs, mut trait_: ItemTrait)
  -> Result<TokenStream2>
{
    let outer_scope = (
        &trait_.generics,
        ::func_wrap::ImplOrTrait::DefaultMethod { trait_name: &trait_.ident },
    );
    trait_.items.iter_mut().try_for_each(|it| match it {
        | &mut TraitItem::Method(ref mut method) => {
            let mut attr = None;
            let mut err = None;
            method
                .attrs
                .retain(|cur_attr| if cur_attr.path.is_ident("with") {
                    let prev = attr.replace(cur_attr.clone());
                    if let Some(prev) = prev {
                        err = Some(Error::new_spanned(prev,
                            "Duplicate `#[with]` attribute",
                        ));
                    }
                    false // remove the attribute
                } else {
                    true
                })
            ;
            if let Some(err) = err { return Err(err); }
            let storage;
            let attrs: &Attrs = match attr {
                Some(attr) => {
                    storage = attr.parse_args::<Attrs>()?;
                    &storage
                },
                None => outer_with_attrs,
            };
            handle_fn_like(&attrs, method, Some(outer_scope))
        },
        | _ => Ok(()),
    })?;
    Ok(trait_.into_token_stream())
}
