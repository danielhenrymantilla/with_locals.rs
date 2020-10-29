fn handle_returning_locals (
    fun: &'_ mut impl helpers::FnLike,
    with_attrs: &'_ Attrs,
    outer_scope: Option<(&'_ Generics, ::func_wrap::ImplOrTrait<'_>)>,
) -> Result<()>
{Ok({
    let &Attrs { ref lifetime, ref continuation, recursive } = with_attrs;
    let continuation_name =
        if let Some(ref continuation_name) = continuation {
            format_ident!("{}", continuation_name)
        } else {
            format_ident!("__continuation__")
        }
    ;
    let fun = fun.fields();
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
    let __ { attrs, sig, block, .. } = fun;
    let &mut Signature {
        ref mut ident,
        ref mut inputs,
        ref mut output,
        ref mut generics, .. } = sig;
    // Add the <R, F : FnOnce(OutputReferringToLocals) -> R> generic params.
    generics.params.push(parse_quote!(
        __Continuation_Return__ /* R */
    ));
    let ret =
        match
            ::core::mem::replace(output, parse_quote! {
              -> __Continuation_Return__
            })
        {
            | ReturnType::Type(_, ty) => *ty,
            | ReturnType::Default => unreachable!(),
        }
    ;
    proc_macro_use! {
        use $krate::{FnOnce};
    }
    generics.params.push(parse_quote! {
        __Continuation__ /* F */
        :
        // for<#(#lifetimes),*>
        #FnOnce(#ret) -> __Continuation_Return__
    });
    inputs.push(parse_quote!(
        #continuation_name : __Continuation__
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
                                match #inner_expr { it => match #Try::into_result(it) {
                                    | #Ok_(it) => it,
                                    | #Err_(err) => {
                                        return __continuation__(
                                            #Try::from_err(
                                                #Into::into(err)
                                            )
                                        );
                                    },
                                }}
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
        attrs.push(parse_quote! {
            #[allow(
                nonstandard_style,
                unreachable_code,
                unused_braces,
                unused_parens,
            )]
        });
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
            // handle_let_bindings::f(block, with_attrs)?;
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
                    &mut |__ret__: #ret| -> (/* Ensure this is not generic */) {
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
        let mut block_prefix = quote! {
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
        };
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
