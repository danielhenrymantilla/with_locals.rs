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
    // fold::Fold,
    parse::{Nothing, Parse, Parser, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Result,
    visit_mut::VisitMut,
};

#[cfg(FALSE)]
macro_rules! parse_quote {(
    $($input:tt)*
) => (
    helper(
        ::quote::quote!($($input)*).to_string(),
        || ::syn::parse_quote!( $($input)* ),
    )
)}
#[cfg_attr(debug_assertions, allow(dead_code))]
fn helper<T : ::syn::parse::Parse> (
    msg: impl ::core::fmt::Display,
    it: impl FnOnce() -> T,
) -> T
{
    eprintln!(
        "Attempting to parse as a {} the following tokens:\n {}",
        ::core::any::type_name::<T>(),
        msg,
    );
    it()
}

#[proc_macro_attribute] pub
fn with (
    attrs: TokenStream,
    input: TokenStream,
) -> TokenStream
{
    let lifetime: Option<Lifetime> = parse_macro_input!(attrs);
    // dbg!(&lifetime);

    let mut fun: ItemFn = parse_macro_input!(input);

    let mut encountered_error = None;

    let mut visitor = Visitor {
        encountered_error: &mut encountered_error,
    };

    {
        use ::std::panic;
        if let Err(panic) = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            visitor.visit_item_fn_mut(&mut fun);
        }))
        {
            if let Some(err) = encountered_error {
                return err.to_compile_error().into();
            } else {
                panic::resume_unwind(panic);
            }
        }
    }

    struct Visitor<'__> {
        encountered_error: &'__ mut Option<::syn::Error>,
    }
    impl VisitMut for Visitor<'_> {
        fn visit_block_mut (
            self: &'_ mut Self,
            mut block: &'_ mut Block,
        )
        {
            macro_rules! throw {
                (
                    $span:expr => $err_msg:expr $(,)?
                ) => ({
                    self.encountered_error.replace(
                        Error::new($span, $err_msg)
                    );
                    panic!();
                });

                (
                    $err_msg:expr $(,)?
                ) => (
                    throw! { Span::call_site() => $err_smg }
                );
            }
            let mut with_idx = None;
            for (i, stmt) in (0 ..).zip(&mut block.stmts) {
                if let Stmt::Local(ref mut let_binding) = *stmt {
                    let mut has_with = false;
                    let_binding.attrs.retain(|attr| {
                        macro_rules! ATTR_NAME {() => ( "with" )}
                        if attr.path.is_ident(ATTR_NAME!()) {
                            has_with = true;
                            if ::syn::parse2::<Nothing>(attr.tokens.clone()).is_err() {
                                throw!(attr.tokens.span() =>
                                    concat!(
                                        "`#[",
                                        ATTR_NAME!(),
                                        "]` takes no attributes",
                                    ),
                                );
                            }
                            false // remove attr
                        } else {
                            true
                        }
                    });
                    if has_with {
                        with_idx = Some(i);
                        break;
                    }
                }
            }
            if let Some(i) = with_idx {
                let tail = block.stmts.split_off(i + 1);
                let mut let_assign =
                    if let Some(Stmt::Local(it)) = block.stmts.pop() {
                        it
                    } else {
                        unreachable!();
                    }
                ;
                let binding = let_assign.pat;
                let init = if let Some(it) = let_assign.init.take() { it } else {
                    throw!(let_assign.semi_token.span() =>
                        "Missing expression"
                    );
                };
                let mut call = *init.1;
                block.stmts.push(Stmt::Expr(call));
                let call = block.stmts.last_mut().map(|stmt| match *stmt {
                    | Stmt::Expr(ref mut it) => it,
                    | _ => unreachable!(),
                }).unwrap();
                let (attrs, args, func) = match *call {
                    | Expr::MethodCall(ExprMethodCall {
                        ref mut attrs,
                        ref mut method,
                        ref mut args,
                        ref mut turbofish,
                        ..
                    })
                    => {
                        if let Some(ref mut turbofish) = turbofish {
                            throw!(turbofish.span()=>
                                "Not yet implemented"
                            );
                        }
                        (attrs, args, method)
                    },

                    | Expr::Call(ExprCall {
                        ref mut attrs,
                        ref mut func,
                        ref mut args,
                        ..
                    }) => {
                        let path = match **func {
                            | Expr::Path(ref mut it) => it,
                            | _ => throw!(func.span() =>
                                "Expected a single-identifier"
                            ),
                        };
                        if let Some(extraneous) = path.attrs.first() {
                            throw!(extraneous.span() =>
                                "`#[with]` does not support attributes"
                            );
                        }
                        let at_last: &mut Ident = // pun intended
                            &mut path.path.segments.iter_mut().next_back().unwrap().ident
                        ;
                        (attrs, args, at_last)
                    },

                    | ref extraneous => throw!(extraneous.span() =>
                        "`fname(...)` or `<expr>.fname(...)` expected"
                    ),
                };

                // attrs: bail if present
                if let Some(extraneous) = attrs.first() {
                    throw!(extraneous.span() =>
                        "`#[with]` does not support attributes"
                    );
                }

                // func: prepend `with_` to the function name
                *func = format_ident!("with_{}", func);

                // args: append the continuation
                args.push(parse_quote! {
                    | #binding | {
                        #(#tail)*
                    }
                });
            }
            block.stmts.iter_mut().for_each(|stmt| self.visit_stmt_mut(stmt));
        }
    }

    handle_returning_local(&mut fun);
    fun .to_token_stream()
        .into()
}

fn handle_returning_local (fun: &'_ mut ItemFn)
{
    let ret_ty =
        if let ReturnType::Type(_, ref mut it) = fun.sig.output { it } else {
            // Nothing to do
            return;
        }
    ;
    let mut lifetimes = vec![]; {
        struct LifetimeVisitor<'__> /* = */ (
            &'__ mut Vec<Lifetime>,
        );
        impl VisitMut for LifetimeVisitor<'_> {
            fn visit_lifetime_mut (
                self: &'_ mut Self,
                lifetime: &'_ mut Lifetime,
            )
            {
                if lifetime.ident == "self" {
                    // lifetime.ident = format_ident!("__self_{}__", self.0.len());
                    lifetime.ident = format_ident!("_", span = lifetime.ident.span());
                    self.0.push(lifetime.clone());
                }
            }
        }

        LifetimeVisitor(&mut lifetimes)
            .visit_type_mut(ret_ty)
        ;
    }
    if lifetimes.is_empty() {
        // Nothing to do
        return;
    }

    // By now, there is at least one `'self` occurence in the return type:
    // transform the whole function into one using the `with_` continuation
    // pattern.
    let ItemFn {
        sig: Signature {
            ref mut ident,
            ref mut inputs,
            ref mut output,
            ref mut generics, .. },
        ref mut block,
        .. } = fun
    ;
    generics.params.push(parse_quote! {
        __Continuation_Return__
    });
    let ret = ::core::mem::replace(output, parse_quote! {
        -> __Continuation_Return__
    });
    let ret = match ret {
        | ReturnType::Type(_, ty) => *ty,
        | ReturnType::Default => unreachable!(),
    };
    generics.params.push(parse_quote! {
        __Continuation__
        :
            // for<#(#lifetimes),*>
            FnOnce(#ret) -> __Continuation_Return__
    });
    inputs.push(parse_quote! {
        __continuation__ : __Continuation__
    });
    *ident = format_ident!("with_{}", ident);
    struct ReturnMapper; impl VisitMut for ReturnMapper {
        fn visit_expr_mut (
            self: &'_ mut Self,
            expr: &'_ mut Expr,
        )
        {
            match *expr {
                | Expr::Async(_)
                | Expr::Closure(_)
                => {
                    // Stop visiting
                    return;
                },

                | Expr::Return(ExprReturn {
                    ref mut attrs,
                    expr: Some(ref mut expr),
                    ..
                }) => {
                    self.visit_expr_mut(expr);
                    *expr = parse_quote! {
                        __continuation__(#expr)
                    };
                },

                | _ => ::syn::visit_mut::visit_expr_mut(self, expr),
            }
        }
    }
    ReturnMapper.visit_block_mut(block);
    *block = parse_quote!({
        macro_rules! ret { ($expr:expr) => (
            match $expr { ret => {
                return __continuation__(ret);
            }}
        )}
        ret!(#block)
    });
}
