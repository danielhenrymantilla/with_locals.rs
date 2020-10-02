/**
```rust
enum ControlFlow<Try, Return, Break, Continue> {
    EarlyTry(Try),
    EarlyReturn(Return),
    Break(Break),
    Continue(Continue),
}

enum None {}
```
**/
fn wrap_statements_inside_closure_body (
    mut stmts: Vec<Stmt>,
) -> Result<TokenStream2>
{Ok({
    #![allow(nonstandard_style)]
    let mut visitor = {
        #[derive(Debug)]
        struct Visitor {
            encountered_error: Option<Error>,
            // We need to keep track of the variants used in the returned enum
            // since those unused won't have type inference kicking in,
            // and will thus need explicit `Void` type annotations.
            question_mark: bool,
            explicit_return: bool,
            has_break: bool,
            has_continue: bool,
        }
        impl VisitMut for Visitor {
            fn visit_expr_mut (
                self: &'_ mut Self,
                expr: &'_ mut Expr,
            )
            {
                // sub-recurse
                visit_mut::visit_expr_mut(self, expr);

                mk_throw! {
                    #![dollar = $]
                    throw! in self.encountered_error
                }

                let krate = ::quote::quote! {
                    ::with_locals::__internals__
                };
                let ControlFlow = ::quote::quote! {
                    #krate::ControlFlow
                };
                let Void = ::quote::quote! {
                    #krate::None
                };
                let Try = ::quote::quote! {
                    #krate::Try
                };
                let core = ::quote::quote! {
                    #krate::core
                };

                macro_rules! todo { ($spanned:expr) => (
                    throw! { $spanned.span() =>
                        &format!("Unimplemented: {:#?}", $spanned),
                    }
                )}
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
                        expr_return.expr.replace(parse_quote! {
                            #ControlFlow::EarlyReturn(#returned_value)
                        });
                    },

                    | Expr::Continue(ref expr_continue) => {
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
                            return #ControlFlow::Continue
                        };
                    },

                    | Expr::Break(ref expr_break) => {
                        self.has_break = true;
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
                            if let Some(ref it) = expr_break.expr {
                                &**it
                            } else {
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

                    | Expr::Try(ref expr_try) => {
                        self.question_mark = true;
                        let matchee = &expr_try.expr;
                        *expr = parse_quote! {
                            match #Try::into_result(#matchee) {
                                | #core::result::Result::Ok(it) => it,
                                | #core::result::Result::Err(err) => {
                                    return #ControlFlow::PropagateError(
                                        #core::convert::Into::into(err)
                                    );
                                },
                            }
                        };
                    }

                    | _ => todo!(expr),
                }
            }
        }
        Visitor {
            encountered_error: None,
            question_mark: false,
            explicit_return: false,
            has_break: false,
            has_continue: false,
        }
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
    let body = ::quote::quote! {
        #(#stmts)*
    };
    match visitor {
        _ => todo!("`wrap_statements_inside_closure_body() -> {:?}`", visitor),
    }
})}
