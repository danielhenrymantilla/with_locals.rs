use super::*;

pub(in super) use wrap_statements_inside_closure_body as f;

pub(in crate)
struct Ret {
    pub(in crate)
    closure_body: TokenStream2,

    pub(in crate)
    wrap_ret: TokenStream2,

    pub(in crate)
    wrap_break: TokenStream2,

    pub(in crate)
    wrap_continue: TokenStream2,
}

/// The frontend crate exposes the following helpers:
///
/// ```rust
/// enum ControlFlow<Eval, Return, Break, Continue> {
///     Eval(Eval),
///     EarlyReturn(Return),
///     Break(Break),
///     Continue(Continue),
/// }
///
/// enum Unreachable {}
/// ```
///
/// See `src/lib.rs` for the actual code.
pub(in super)
fn wrap_statements_inside_closure_body (
    mut stmts: ::std::collections::VecDeque<Stmt>,
) -> Result<Ret>
{Ok({
    #![allow(nonstandard_style)]

    proc_macro_use! {
        use $krate::{
            ControlFlow,
        };
    }

    /// Required because you can't always `break ()` where you can `break`
    /// (_e.g._, `for` loops) ... -___-'
    enum BreakKind {
        WithValue(Span, Span),
        Alone,
    }

    let mut visitor = {
        #[derive(Default)]
        struct Visitor {
            encountered_error: Option<Error>,
            // We need to keep track of the variants used in the returned enum
            // since those unused won't have type inference kicking in,
            // and will thus need explicit `Void` type annotations.
            explicit_return: bool,
            break_kind: Option<BreakKind>,
            has_continue: bool,
            within_loop: bool,
        }
        impl VisitMut for Visitor {
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
                mk_throw! {
                    #![dollar = $]
                    throw! in self.encountered_error
                }

                proc_macro_use!{
                    use $krate::{
                        ControlFlow,
                        Into,
                        Ok_, Err_,
                        Try,
                    };
                }

                match *expr {
                    | Expr::Return(ref mut expr_return) => {
                        self.explicit_return = true;
                        let storage;
                        let returned_value =
                            if let Some(ref it) = expr_return.expr {
                                &**it
                            } else {
                                storage = parse_quote! {
                                    ()
                                };
                                &storage
                            }
                        ;
                        expr_return.expr = Some(parse_quote! {
                            #ControlFlow::EarlyReturn(#returned_value)
                        });
                    },

                    | Expr::Continue(ref expr_continue)
                        if self.within_loop.not()
                    => {
                        self.has_continue = true;
                        if let Some(ref label) = expr_continue.label {
                            throw! { label.span() =>
                                "\
                                    `#[with]` does not support \
                                    labelled `continue`s\
                                "
                            }
                        }
                        *expr = parse_quote! {
                            return #ControlFlow::Continue(())
                        };
                    },

                    | Expr::Break(ref expr_break)
                        if self.within_loop.not()
                    => {
                        if let Some(ref label) = expr_break.label {
                            throw! { label.span() =>
                                "\
                                    `#[with]` does not support \
                                    labelled `break`s\
                                "
                            }
                        }
                        let storage;
                        let broken_value =
                            if let Some(ref break_value) = expr_break.expr {
                                self.break_kind.replace(BreakKind::WithValue(
                                    expr_break.break_token.span(),
                                    break_value.span(),
                                ));
                                &**break_value
                            } else {
                                self.break_kind = Some(BreakKind::Alone);
                                storage = parse_quote! {
                                    ()
                                };
                                &storage
                            }
                        ;
                        *expr = parse_quote! {
                            return #ControlFlow::Break(#broken_value)
                        };
                    },

                    | Expr::Try(ref mut expr_try) => {
                        self.explicit_return = true;
                        let matchee = &mut expr_try.expr;
                        // /!\ sub-recurse *before* the transformation, since
                        // we do generate an *inner* `return` expression:
                        self.visit_expr_mut(matchee);
                        *expr = parse_quote! {
                            match #matchee { it => match #Try::into_result(it) {
                                | #Ok_(it) => it,
                                | #Err_(err) => {
                                    return #ControlFlow::EarlyReturn(
                                        #Try::from_err(
                                            #Into::into(err)
                                        )
                                    );
                                },
                            }}
                        };
                        // do not subrecurse now
                        return;
                    },

                    | Expr::ForLoop(_)
                    | Expr::Loop(_)
                    | Expr::While(_)
                    => {
                        self.within_loop = (
                            ::core::mem::replace(&mut self.within_loop, true),
                            // sub-recurse
                            visit_mut::visit_expr_mut(self, expr),
                        ).0;
                    }

                    | Expr::Async(_)
                    | Expr::Closure(_)
                    => {
                        // skip sub-recursing
                        return;
                    },

                    | _ => {},
                }
                // sub-recurse
                visit_mut::visit_expr_mut(self, expr);
            }
        }
        Visitor::default()
    };
    use ::std::panic;
    if let Err(panic) = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        stmts.iter_mut().for_each(|stmt| {
            visitor.visit_stmt_mut(stmt)
        });
    }))
    {
        if let Some(err) = visitor.encountered_error {
            return Err(err);
        } else {
            panic::resume_unwind(panic);
        }
    }
    let stmts = stmts.into_iter();

    fn ty_and_handler (
        can_be_inferred: bool,
        wrapper: impl FnOnce() -> TokenStream2,
    ) -> (
            TokenStream2,
            TokenStream2,
        )
    {
        proc_macro_use! {
            use $krate::{Unreachable};
        }
        if can_be_inferred {
            (
                quote! {
                    _
                },
                wrapper(),
            )
        } else {
            (
                quote! {
                    #Unreachable
                },
                quote! {
                    {
                        let unreachable = it;
                        match unreachable {}
                    }
                },
            )
        }
    }

    let (Return, wrap_ret) = ty_and_handler(
        visitor.explicit_return,
        || quote! {
            return it
        },
    );
    let (Break, wrap_break) = match visitor.break_kind {
        | None => (
            {
                proc_macro_use! {
                    use $krate::{Unreachable};
                }
                Unreachable
            },
            quote! {
                {
                    let unreachable = it;
                    match unreachable {}
                }
            },
        ),
        | Some(break_kind) => (
            quote! { _ },
            if let BreakKind::WithValue(start, end) = break_kind {
                let break_ = quote_spanned!(start=> break );
                let it = quote_spanned!(end=> it );
                quote! {
                    #break_ #it
                }
            } else {
                quote! {
                    {
                        let () = it;
                        break;
                    }
                }
            },
        ),
    };
    let (Continue, wrap_continue) = ty_and_handler(
        visitor.has_continue,
        || quote! {
            { let () = it; continue }
        },
    );
    Ret {
        closure_body: quote! {
            #ControlFlow::<_, #Return, #Break, #Continue>::Eval({
                #(#stmts)*
            })
        },
        wrap_ret,
        wrap_break,
        wrap_continue,
    }
})}
