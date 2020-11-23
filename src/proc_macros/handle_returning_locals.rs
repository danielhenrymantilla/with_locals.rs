use super::*;

pub(in super) use handle_returning_locals as f;

pub(in super)
fn handle_returning_locals (
    fun: &'_ mut impl helpers::FnLike,
    with_attrs: &'_ Attrs,
    outer_scope: Option<(&'_ Generics, ::func_wrap::ImplOrTrait<'_>)>,
) -> Result<()>
{Ok({
    let         &Attrs {
        ref lifetime,
        ref continuation,
        dyn_safe,
        recursive,
                } = with_attrs
    ;
    fun.fields().attrs.push(parse_quote! {
        #[allow(
            nonstandard_style,
            unreachable_code,
            unused_braces,
            unused_parens,
        )]
    });
    let not_dyn_safe = dyn_safe.not();
    // Note: currently, the necessary `dyn`-safe transformations also allow
    // preventing the recursive function issue, so no need to apply any extra
    // transformations.
    let recursive = recursive && not_dyn_safe;
    let continuation_name =
        if let Some(ref continuation_name) = continuation {
            format_ident!("{}", continuation_name)
        } else {
            format_ident!("__continuation__")
        }
    ;
    let fun = fun.fields();
    unelide_lifetimes(fun.sig, lifetime)?;
    let ret_ty =
        if let ReturnType::Type(_, ref mut it) = fun.sig.output { it } else {
            // Nothing to do
            return Ok(());
        }
    ;
    let mut lifetimes = vec![]; {
        LifetimeVisitor { lifetimes: &mut lifetimes, lifetime: &*lifetime }
            .visit_type_mut(ret_ty)
        ;
    }
    if lifetimes.is_empty() {
        // Nothing to do
        return Ok(());
    }

    // By now, there is at least one `'self` occurence in the return type:
    // transform the whole function into one using the `with_` continuation
    // pattern.
    let __ { sig, block, .. } = fun;
    let &mut Signature {
        ref mut ident,
        ref mut inputs,
        ref mut output,
        ref mut generics, .. } = sig;
    // Add the <R, F : FnOnce(OutputReferringToLocals) -> R> generic params.
    // (Or use the `dyn`-safe equivalent).
    let R = if not_dyn_safe {
        let new_ty_param = quote!(
            __Continuation_Return__
        );
        generics.params.push(parse_quote!( #new_ty_param ));
        new_ty_param
    } else {
        quote!(
            ::with_locals::dyn_safe::ContinuationReturn
        )
    };
    let ret =
        match ::core::mem::replace(output, parse_quote!( -> #R )) {
            | ReturnType::Type(_, ty) => *ty,
            | ReturnType::Default => unreachable!(),
        }
    ;
    proc_macro_use! {
        use $krate::{FnMut, FnOnce};
    }
    let F = if not_dyn_safe {
        let new_ty_param = quote!(
            __Continuation__
        );
        generics.params.push(parse_quote! {
            #new_ty_param
            :
            // for<#(#lifetimes),*>
            #FnOnce(#ret) -> #R
        });
        new_ty_param
    } else {
        quote!(
            &'_ mut (dyn '_ + #FnMut(#ret) -> #R)
        )
    };
    inputs.push(parse_quote!(
        #continuation_name : #F
    ));
    *ident = format_ident!("with_{}", ident);
    if let Some(block) = block {
        // Only apply `return <expr> -> return cont(<expr>)` magic
        // if no continuation name has been provided.
        if continuation.is_none() {
            // Replace any terminating `expr` with `return <expr>`:
            #[derive(Default)]
            struct AddExplicitReturns {
                done: bool,
            }
            impl VisitMut for AddExplicitReturns {
                fn visit_item_mut (
                    self: &'_ mut Self,
                    _: &'_ mut Item,
                )
                {
                    // Stop recursing.
                }

                fn visit_block_mut (
                    self: &'_ mut Self,
                    block: &'_ mut Block,
                )
                {
                    match block.stmts.last_mut() {
                        | Some(&mut Stmt::Expr(ref mut expr)) => {
                            self.visit_expr_mut(expr);
                            if self.done.not() {
                                *expr = parse_quote! {
                                    return #expr
                                };
                                self.done = true;
                            }
                        },

                        | _ => {
                            // Do nothing (do not recurse):
                            // the return type cannot be `()`
                            // and yet the block does not end with an expr, so
                            // unless the last expr diverges there will be a
                            // type error anyways.
                        },
                    }
                }

                fn visit_expr_mut (
                    self: &'_ mut Self,
                    expr: &'_ mut Expr,
                )
                {
                    match *expr {
                        | Expr::Block(ExprBlock {
                            ref mut block,
                            ..
                        }) => {
                            self.visit_block_mut(block);
                        },

                        | Expr::If(ExprIf {
                            ref mut then_branch,
                            else_branch: ref mut mb_else_branch,
                            ..
                        }) => {
                            self.visit_block_mut(then_branch);
                            self.done = false;
                            if let Some((_, else_)) = mb_else_branch {
                                self.visit_expr_mut(else_);
                            } else {
                                // Do nothing, the return type cannot be `()`
                                // and yet the block ends with an else-less
                                // if block, so there will be a type error
                                // anyways.
                            }
                            self.done = true;
                        },

                        | Expr::Match(ExprMatch {
                            ref mut arms,
                            ..
                        }) => {
                            for arm in arms {
                                let body = &mut arm.body;
                                self.visit_expr_mut(body);
                                if self.done.not() {
                                    // handle the non-braced body arm case.
                                    *body = parse_quote! {
                                        return #body
                                    };
                                }
                                self.done = false;
                            }
                            self.done = true;
                        },

                        | _ => {
                            // Do nothing (do not recurse)
                        }
                    }
                }
            }
            AddExplicitReturns::default().visit_block_mut(block);

            // Then map `return <expr>` to `return cont(<expr>)`.
            struct ReturnMapper; impl VisitMut for ReturnMapper {
                fn visit_item_mut (
                    self: &'_ mut Self,
                    _: &'_ mut Item,
                )
                {
                    // Stop recursing.
                }

                fn visit_expr_mut (
                    self: &'_ mut Self,
                    expr: &'_ mut Expr,
                )
                {
                    proc_macro_use! {
                        use $krate::{
                            Into,
                            Ok_, Err_,
                            Try,
                        };
                    }
                    match *expr {
                        | Expr::Async(_)
                        | Expr::Closure(_)
                        => {
                            // Stop visiting
                            return;
                        },

                        // `return <expr>` ...
                        | Expr::Return(ExprReturn {
                            expr: Some(ref mut expr),
                            ..
                        }) => {
                            // recurse
                            self.visit_expr_mut(expr);
                            // ... becomes `return cont(<expr>)`
                            *expr = parse_quote! {
                                __continuation__(#expr)
                            };
                        },

                        // `<expr>?` carries a hidden `return Err(err.into())`
                        // inside it, we need to change it:
                        | Expr::Try(ExprTry {
                            expr: ref mut inner_expr,
                            // to span the error-related logic
                            question_token: _, // FIXME(spans)?
                            ..
                        }) => {
                            // recurse
                            self.visit_expr_mut(inner_expr);
                            *expr = parse_quote! {
                                match #inner_expr {
                                    it => match #Try::into_result(it) {
                                        | #Ok_(it) => it,
                                        | #Err_(err) => {
                                            return __continuation__(
                                                #Try::from_err(
                                                    #Into::into(err)
                                                )
                                            );
                                        },
                                    }
                                }
                            };
                        },

                        | _ => {
                            // sub-recurse
                            visit_mut::visit_expr_mut(self, expr);
                        },
                    }
                }
            }
            ReturnMapper.visit_block_mut(block);
        }
        proc_macro_use! {
            use $krate::{Some_};
        }
        if recursive {
            /* We currently have something like:
            ```rust
            fn with_foo<R, F : FnOnce(&'_ ()) -> R> (
                recurse: bool,
                with: F,
            ) -> R
            {
                if recurse {
                    with_foo(false, |ret| {
                        with(ret)
                    })
                } else {
                    with(&())
                }
            }
            ```

            The objetive, to avoid the infinite type recursion, is to replace
            `<F>` with an `&mut dyn FnMut`, and `<R>` with a return value of
            `()` (the value will be "returned" through a mutate out slot,
            thus type-erased by the `dyn FnMut` type erasure).

              - EDIT: Actually, returning `()` is less type-safe, in that a
                      caller may forget to call the continuation (especially
                      if they opt out of sugar).
                      So we only replace `R` with `()` at call site.

            ```rust
            fn with_foo<R, F : FnOnce(&'_ ()) -> R> (
                recurse: bool,
                with: F,
            ) -> R
            {
                let mut ret_slot = None;
                let mut with = Some(with);

                // wrapped_func…
                fn __recurse_with_foo<R> (
                    recurse: bool,
                    with: &'_ mut (dyn FnMut(&'_ ())) -> R,
                ) -> R
                {
                    if recurse {
                        with_foo(false, |ret| {
                            with(ret)
                        })
                    } else {
                        with(&())
                    }
                }
                // …_call
                __recurse_with_foo(recurse, &mut |ret| -> () {
                    ret_slot = Some(with.take().unwrap()(ret));
                });

                ret_slot.unwrap()
            }
            ``` */
            proc_macro_use! {
                use $krate::{FnMut, None_};
            }
            let mut wrapped_func_call = match ::func_wrap::func_wrap(
                sig,
                ::core::mem::replace(block, parse_quote!( {} )),
                outer_scope,
            )
            {
                | Some(it) => it,
                | None => return Err(Error::new(Span::call_site(), "\
                    Missing `#[with]` on the enscoping `impl` or `trait` block\
                ")),
            };
            handle_let_bindings::f(&mut wrapped_func_call.block, with_attrs)?;
            wrapped_func_call.sig.ident = format_ident!(
                "__recurse_{}",
                wrapped_func_call.sig.ident,
            );
            let _ = wrapped_func_call.sig.generics.params.pop(); // <…, F>
            match wrapped_func_call.sig.inputs.last_mut() {
                | Some(&mut FnArg::Typed(ref mut pat_ty)) => {
                    *pat_ty.ty = parse_quote!(
                        &'_ mut (dyn #FnMut(#ret) -> __Continuation_Return__)
                    );
                },
                | _ => unreachable!(),
            }
            *wrapped_func_call.call_site_args.last_mut().unwrap() =
                parse_quote!(
                    &mut |__ret__: #ret| -> (/* Ensure this isn't generic */) {
                        __ret_slot__ = #Some_(#continuation_name(__ret__));
                    }
                )
            ;
            *block = parse_quote!({
                let mut __ret_slot__ = #None_;
                type __Continuation_Return__ = ();
                let () = #wrapped_func_call;
                __ret_slot__.expect("\
                    Fatal `with_locals` error: \
                    failed to call the continuation.\
                ")
            });
        } // end of recursive-related tranformations.
        let mut block_prefix = if dyn_safe { quote!() } else { quote!(
            /// Some user-provided code patterns, once transformed, may scare
            /// Rust into thinking we are calling an `FnOnce()` multiple times.
            /// Since that _shouldn't_ be the case, we defer to a runtime check,
            /// hoping that, in practice, it will end up being optimized away.
            extern {}
            let mut #continuation_name = {
                let mut #continuation_name =
                    #Some_(#continuation_name)
                ;
                move |__ret__: #ret| {
                    #continuation_name
                        .take()
                        .expect("\
                            Fatal `with_locals` error: \
                            attempted to call an `FnOnce()` multiple times.\
                        ")
                        (__ret__)
                }
            };
        )};
        if continuation.is_some() {
            // Requires Rust 1.40.0
            block_prefix.extend(quote! {
                macro_rules! #continuation_name { ($expr:expr) => (
                    return #continuation_name($expr)
                )}
            });
        }
        *block = parse_quote!({
            #block_prefix
            #block
        });
    }
})}

fn unelide_lifetimes (sig: &'_ mut Signature, special_lifetime: &'_ str)
  -> Result<()>
{
    fn mk_named_lifetime (span: Span)
      -> Lifetime
    {
        Lifetime::new("'__elided", span)
    }

    let has_receiver = sig.receiver().is_some();
    let         &mut Signature {
        ref mut inputs,
        ref mut output,
        ref mut generics,
                ..} = sig
    ;
    let mut unit: Type;
    let output: &mut Type =
        if let ReturnType::Type(_, ref mut ty) = *output {
            &mut **ty
        } else {
            unit = parse_quote!( () );
            &mut unit
        }
    ;
    if has_receiver {
        let mut elided_lifetime_in_receiver_case =
            |and: &'_ Token![&], mb_lifetime: &'_ mut Option<Lifetime>| {
                let storage;
                let lifetime = match *mb_lifetime {
                    | Some(ref lt) if lt.ident != "_" => lt,
                    | _ => {
                        storage = mk_named_lifetime(
                            mb_lifetime
                                .as_ref()
                                .map_or_else(|| and.span(), |lt| lt.span())
                        );
                        let lifetime = &storage;
                        generics.params.push(parse_quote!(
                            #lifetime
                        ));
                        *mb_lifetime = Some(lifetime.clone());
                        lifetime
                    },
                };
                replace_each_lifetime_with(output, &mut |span, name| {
                    if name.is_some() { return None; }
                    Some(Lifetime {
                        apostrophe: span,
                        ident: lifetime.ident.clone(),
                    })
                });
                Ok(())
            }
        ;
        match inputs.iter_mut().next() {
            | Some(FnArg::Receiver(Receiver {
                reference: Some((ref and, ref mut mb_lifetime)),
                ..
            }))
            => {
                return elided_lifetime_in_receiver_case(and, mb_lifetime);
            },

            | Some(FnArg::Typed(PatType {
                pat: _, /* if we are here we know the `pat` is `self` */
                ref mut ty,
                ..
            }))
            => match **ty {
                | Type::Reference(TypeReference {
                    and_token: ref and,
                    lifetime: ref mut mb_lifetime,
                    ..
                })
                => {
                    return elided_lifetime_in_receiver_case(and, mb_lifetime);
                },
                | _ => {},
            },

            | _ => {},
        }
    }

    // Vec with spans pointing to elided lifetimes.
    let (mut first_span, mut last_span) = (None, None);
    let mut elided_lifetimes_count = 0;
    let ref mut named_lifetimes = ::std::collections::HashSet::<Ident>::new();
    inputs
        .iter_mut()
        .for_each(|fn_arg| match *fn_arg {
            | FnArg::Typed(PatType { ref mut ty, .. }) => {
                replace_each_lifetime_with(ty, &mut |span, name| {
                    let _ = first_span.get_or_insert(span);
                    last_span = Some(span);
                    if let Some(name) = name {
                        named_lifetimes.insert(name.clone());
                        None
                    } else {
                        elided_lifetimes_count += 1;
                        Some(mk_named_lifetime(span))
                    }
                });
            },
            | FnArg::Receiver(_) => {
                // No interesting lifetimes to be encountered here,
                // otherwise the `if has_receiver` short-circuiting branch
                // would have caught it.
                // Thus, continue / pass.
                return;
            },
        })
    ;
    if let Some(span) = first_span {
        // There was at least one elided lifetime replaced with
        // `mk_named_lifetime`, thus introduce that name within the generic
        // lifetime params.
        let lifetime = mk_named_lifetime(span);
        generics.params.push(parse_quote!(
            #lifetime
        ));
    }
    let input_lifetime_params_count = <usize as ::core::ops::Add>::add(
        elided_lifetimes_count,
        named_lifetimes.len(),
    );
    let mut lifetime_cannot_be_derived_from_arguments = None;
    replace_each_lifetime_with(output, &mut |span, name| {
        if name.is_some() { return None; }
        if input_lifetime_params_count != 1 {
            lifetime_cannot_be_derived_from_arguments = Some(span);
            None
        } else {
            Some(if let Some(ident) = named_lifetimes.iter().next() {
                Lifetime {
                    apostrophe: span,
                    ident: ident.clone(),
                }
            } else {
                mk_named_lifetime(span)
            })
        }
    });
    if let Some(err_span) = lifetime_cannot_be_derived_from_arguments {
        Err(match (first_span, last_span) {
            | (Some(first_span), Some(last_span)) => {
                let mut err = Error::new(err_span, "\
                    \n\
                    error[E0106]: missing lifetime specifier
                ");
                let extra_err_msg = format!(
                    "\
                    \n\
                    help: this function's return type contains a borrowed \
                    value, but the signature does not say which one of the \
                    arguments' {} lifetimes it is borrowed from\n\
                    help: specify it using explicitly named lifetime \
                    parameters\
                    ",
                    input_lifetime_params_count,
                );
                // Hacks to give the error message more complex / advanced spans
                // (ideally it should be pointing to the elided lifetimes).
                use ::proc_macro2::*;
                fn mk_dummy_token (span: Span)
                  -> TokenTree
                {
                    let mut p = Punct::new('.', Spacing::Alone);
                    p.set_span(span);
                    p.into()
                }
                err.combine(Error::new_spanned(
                    Iterator::chain(
                        ::core::iter::once(mk_dummy_token(first_span)),
                        ::core::iter::once(mk_dummy_token(last_span)),
                    ).collect::<TokenStream2>(),
                    extra_err_msg,
                ));
                err
            },
            #[cfg(debug_assertions)]
            | (Some(_), None) | (None, Some(_)) => unreachable!(),
            | _ => {
                let err_msg = format!(
                    "\
                    \n\
                    error[E0106]: missing lifetime specifier\n \
                    help: this function's return type contains a borrowed \
                    value, but there is no value for it to be borrowed from\n \
                    help: consider using the `'static` lifetime, or the \
                    function-local lifetime, `'{}`\
                    ",
                    special_lifetime,
                );
                Error::new(err_span, err_msg)
            },
        })
    } else {
        Ok(())
    }
}

type Cb<'__> = &'__ mut (
    dyn FnMut(Span, /* name: */ Option<&'_ Ident>) -> Option<Lifetime>
);

fn replace_each_lifetime_with (
    ty: &'_ mut Type,
    f: Cb<'_>,
)
{
    struct LifetimeVisitor<'__>(Cb<'__>);
    impl VisitMut for LifetimeVisitor<'_> {
        fn visit_lifetime_mut (
            self: &'_ mut Self,
            lifetime: &'_ mut Lifetime,
        )
        {
            let name =
                if lifetime.ident == "_" {
                    None
                } else {
                    Some(&lifetime.ident)
                }
            ;
            if let Some(new_lifetime) = (self.0)(lifetime.span(), name) {
                *lifetime = new_lifetime;
            }
        }

        fn visit_type_mut (
            self: &'_ mut Self,
            ty: &'_ mut Type,
        )
        {
            match *ty {
                | Type::Reference(TypeReference {
                    and_token: ref and,
                    lifetime: ref mut implicitly_elided_lifetime @ None,
                    elem: ref mut referee,
                    ..
                }) => {
                    if let Some(new_lifetime) = (self.0)(and.span(), None) {
                        *implicitly_elided_lifetime = Some(new_lifetime);
                        // subrecurse only on the referee, not the newly
                        // introduced lifetime.
                        self.visit_type_mut(referee);
                        return;
                    }
                },
                | Type::BareFn(_) => {
                    /* do nothing */
                    return; // do not sub-recurse
                },
                | Type::TraitObject(ref mut trait_object) => {
                    trait_object.bounds.iter_mut().for_each(|it| match *it {
                        | TypeParamBound::Lifetime(ref mut lifetime) => {
                            // Explicit `+ '_`  elision overrides the
                            // special one of trait objects, leading to
                            // classic elision rules to kick in:
                            self.visit_lifetime_mut(lifetime);
                        },
                        | TypeParamBound::Trait(TraitBound {
                            ref mut path,
                            ..
                        }) => {
                            self.visit_path_mut(path)
                        },
                    });
                    return; // do not sub-recurse
                },
                | _ => {},
            }
            // Sub-recurse
            ::syn::visit_mut::visit_type_mut(self, ty);
        }

        // `Fn…()` traits behave like `TypeBareFn`: their elided lifetimes
        // follow higher-order rules.
        fn visit_parenthesized_generic_arguments_mut (
            self: &'_ mut Self,
            _: &'_ mut ParenthesizedGenericArguments,
        )
        {
            /* Do nothing, _i.e._, skip it */
        }
    }
    LifetimeVisitor(f)
        .visit_type_mut(ty)
    ;
}
