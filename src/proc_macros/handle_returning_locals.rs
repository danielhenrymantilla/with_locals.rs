fn handle_returning_locals (
    fun: &'_ mut Input,
    &Attrs { ref lifetime, ref continuation }: &'_ Attrs,
)
{
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
            return;
        }
    ;
    let mut lifetimes = vec![]; {
        LifetimeVisitor { lifetimes: &mut lifetimes, lifetime: &*lifetime }
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
    let __ {
        ref mut attrs,
        sig: &mut Signature {
            ref mut ident,
            ref mut inputs,
            ref mut output,
            ref mut generics, .. },
        ref mut block,
        .. } = {fun}
    ;
    // Add the <R, F : FnOnce(OutputReferringToLocals) -> R> generic params.
    generics.params.push(parse_quote! {
        __Continuation_Return__ /* R */
    });
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
    generics.params.push(parse_quote! {
        __Continuation__ /* F */
        :
        // for<#(#lifetimes),*>
        ::core::ops::FnOnce(#ret) -> __Continuation_Return__
    });
    inputs.push(parse_quote! {
        #continuation_name : __Continuation__
    });
    *ident = format_ident!("with_{}", ident);
    if let Some(&mut ref mut block) = *block {
        // Only apply `return <expr> -> return cont(<expr>)` magic
        // if no continuation name has been provided.
        if continuation.is_none() {
            // Replace any terminating `expr` with `return <expr>`:
            #[derive(Default)]
            struct AddExplicitReturns {
                done: bool,
            }
            impl VisitMut for AddExplicitReturns {
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
                fn visit_expr_mut (
                    self: &'_ mut Self,
                    expr: &'_ mut Expr,
                )
                {
                    proc_macro_use! {
                        use $krate::{
                            Into,
                            Result,
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
                                match #Try::into_result(
                                    #inner_expr
                                )
                                {
                                    | #Result::Ok(it) => it,
                                    | #Result::Err(err) => {
                                        return __continuation__(
                                            #Try::from_err(
                                                #Into::into(err)
                                            )
                                        );
                                    },
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
        attrs.push(parse_quote! {
            #[allow(unreachable_code, unused_braces)]
        });
        proc_macro_use! {
            use $krate::{Option};
        }
        *block = parse_quote!({
            // Some user-provided code patterns, once transformed, may scare
            // Rust into thinking we are calling an `FnOnce()` multiple times.
            // Since that _shouldn't_ be the case, we defer to a runtime check,
            // hoping that, in practice, it will end up being optimized away.
            let mut #continuation_name = {
                let mut #continuation_name =
                    #Option::Some(#continuation_name)
                ;
                move |__ret__: #ret| {
                    #continuation_name.take().unwrap()(__ret__)
                }
            };
            macro_rules! #continuation_name { ($expr:expr) => (
                match $expr { __ret__ => {
                    return #continuation_name(__ret__);
                }}
            )}
            #block
        });
    }
}
