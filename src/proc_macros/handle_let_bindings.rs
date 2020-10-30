use super::*;

pub(in super) use handle_let_bindings as f;

/// ```rust,ignore
/// #[with] let var = func(/* args */);
/// ...
/// ```
///
/// must become:
///
/// ```rust,ignore
/// func(/* args */, |var| {
///     ...
/// })
/// ```
pub(in super)
fn handle_let_bindings (
    block: &'_ mut Block,
    &Attrs { ref lifetime, dyn_safe, recursive, .. }: &'_ Attrs,
) -> Result<()>
{Ok({
    let mut encountered_error = None;
    let mut visitor = ReplaceLetBindingsWithCbCalls {
        encountered_error: &mut encountered_error,
        lifetime: &*lifetime,
        dyn_safe_calls: dyn_safe && recursive,
    };
    use ::std::panic;
    if let Err(panic) = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        visitor.visit_block_mut(block)
    }))
    {
        if let Some(err) = encountered_error {
            return Err(err);
        } else {
            panic::resume_unwind(panic);
        }
    }
})}

struct ReplaceLetBindingsWithCbCalls<'__> {
    encountered_error: &'__ mut Option<::syn::Error>,
    lifetime: &'__ str,
    dyn_safe_calls: bool,
}

impl VisitMut for ReplaceLetBindingsWithCbCalls<'_> {
    fn visit_item_mut (
        self: &'_ mut Self,
        _: &'_ mut Item,
    )
    {
        // Do not recurse into items defined inside the function body.
    }

    fn visit_block_mut (
        self: &'_ mut Self,
        block: &'_ mut Block,
    )
    {
        mk_throw! {
            #![dollar = $]
            throw! in self.encountered_error
        }

        let orig_dyn_safe_calls = self.dyn_safe_calls;
        let with_idx = (0 ..).zip(&mut block.stmts).find_map(|(i, stmt)| {
            // `( #[with] )? let <binding> (: <ty>)? = <expr>;`
            if let Stmt::Local(ref mut let_binding) = *stmt {
                let mut has_with = false;

                fn dyn_safe_parser (input: ParseStream<'_>)
                  -> Result<Option<bool>>
                {
                    match (|| Ok({
                        let content;
                        parenthesized!(content in input);
                        content
                    }))()
                    {
                        | Err(_) => {
                            Ok(None)
                        },
                        | Ok(content) => {
                            mod kw { ::syn::custom_keyword!(dyn_safe); }
                            Ok(if  content
                                    .parse::<Option<kw::dyn_safe>>()?
                                    .is_some()
                            {
                                // allow the `#[with(dyn_safe)]` shorthand
                                if  content.parse::<Option<Token![=]>>()?
                                        .is_some()
                                {
                                    let lit_bool: LitBool = content.parse()?;
                                    let _: Option<Token![,]> = content.parse()?;
                                    Some(lit_bool.value)
                                } else {
                                    Some(true)
                                }
                            } else {
                                None
                            })
                        },
                    }
                }

                let_binding.attrs.retain(|attr| {
                    if attr.path.is_ident("with") {
                        has_with = true;
                        match dyn_safe_parser.parse2(attr.tokens.clone()) {
                            Ok(Some(dyn_safe)) => self.dyn_safe_calls = dyn_safe,
                            Ok(None) => {},
                            Err(err) => {
                                panic!(*self.encountered_error = Some(err));
                            },
                        }
                        false // remove attr
                    } else {
                        true
                    }
                });
                // Also look for a special lifetime
                has_with |= {
                    let ref mut lifetimes = vec![];
                    LifetimeVisitor { lifetimes, lifetime: self.lifetime }
                        .visit_pat_mut(&mut let_binding.pat)
                    ;
                    lifetimes.is_empty().not()
                };
                if has_with {
                    return Some(i);
                }
            }
            None
        });
        if let Some(i) = with_idx {
            let mut stmts_after_with_let: ::std::collections::VecDeque<_> =
                block
                    .stmts
                    // .split_off(i + 1)
                    .drain((i + 1) ..).collect()
            ;
            let mut let_assign =
                if let Some(Stmt::Local(it)) = block.stmts.pop() {
                    it
                } else {
                    unreachable!();
                }
            ;
            let mut binding = let_assign.pat;
            let init =
                if let Some(it) = let_assign.init.take() { it } else {
                    throw!(let_assign.semi_token.span() =>
                        "Missing expression"
                    );
                }
            ;
            let mut call = *init.1;
            let (attrs, args, func) = loop {
                break match call {
                    | Expr::MethodCall(ExprMethodCall {
                        ref mut attrs,
                        ref mut method,
                        ref mut args,
                        ref mut turbofish,
                        ..
                    })
                    => {
                        if let Some(ref mut turbofish) = turbofish {
                            // ContinuationRet
                            turbofish.args.push(GenericMethodArgument::Type(
                                parse_quote![ _ ]
                            ));
                            // Continuation
                            turbofish.args.push(GenericMethodArgument::Type(
                                parse_quote![ _ ]
                            ));
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
                                "Expected a function name"
                            ),
                        };
                        let at_last /* pun intended */ =
                            path.path
                                .segments
                                .iter_mut()
                                .next_back()
                                .unwrap()
                        ;

                        // check to see if there is turbofish around
                        match at_last.arguments {
                            | PathArguments::AngleBracketed(ref mut turbofish)
                            => {
                                // ContinuationRet
                                turbofish.args.push(GenericArgument::Type(
                                    parse_quote![ _ ]
                                ));
                                // Continuation
                                turbofish.args.push(GenericArgument::Type(
                                    parse_quote![ _ ]
                                ));
                            },

                            | _
                            => {},
                        }

                        (attrs, args, &mut at_last.ident)
                    },

                    | Expr::Match(ExprMatch {
                        ref mut expr,
                        match_token: token::Match {
                            span,
                        },
                        ..
                    })
                    | Expr::Try(ExprTry {
                        ref mut expr,
                        question_token: token::Question {
                            spans: [span],
                        },
                        ..
                    }) => {
                        let anon = format_ident!(
                            "__with_locals_anon__",
                            span = span,
                        );
                        let expr = mem::replace(expr, parse_quote! {
                            #anon
                        });
                        stmts_after_with_let.push_front(parse_quote! {
                            let #binding = #call;
                        });
                        binding = parse_quote!( #anon );
                        call = *expr;
                        continue;
                    },

                    | ref extraneous => throw!(extraneous.span() =>
                        "\
                            expected \
                            `function(...)`, \
                            `function(...)?...?`, \
                            or `<expr>.method(...)`, \
                            or `<expr>.method(...)?...?`\
                        "
                    ),
                }
            };

            // attrs: bail if present
            if let Some(extraneous) = attrs.first() {
                throw!(extraneous.span() =>
                    "`#[with]` does not support attributes"
                );
            }

            // func: prepend `with_` to the function name
            *func = format_ident!("with_{}", func);

            let wrap_statements_inside_closure_body::Ret {
                closure_body,
                wrap_ret,
                wrap_break,
                wrap_continue } =
                    match wrap_statements_inside_closure_body::f(
                        stmts_after_with_let
                    )
                    {
                        | Ok(it) => it,
                        | Err(err) => panic! {
                            *self.encountered_error = Some(err)
                        },
                    }
            ;

            proc_macro_use! {
                use $krate::{ControlFlow, Some_};
            }

            // args: append the continuation
            args.push(if self.dyn_safe_calls.not () {
                parse_quote!(
                    |#binding| #closure_body
                )
            } else {
                parse_quote!(
                    &mut {
                        let mut closure = #Some_(|#binding| #closure_body);
                        move |ret| {
                            __with_locals_ret_slot__.replace(
                                closure.take().unwrap()(ret)
                            );
                            ::with_locals::dyn_safe::ContinuationReturn
                        }
                    }
                )
            });
            if self.dyn_safe_calls {
                proc_macro_use! {
                    use $krate::{None_};
                }
                call = parse_quote!({
                    let mut __with_locals_ret_slot__ = #None_;
                    {
                        let __with_locals_ret_slot__ = &mut __with_locals_ret_slot__;
                        let _ = #call;
                    }
                    __with_locals_ret_slot__.unwrap()
                });
            }
            block.stmts.push(Stmt::Expr(parse_quote! {
                match #call {
                    | #ControlFlow::Eval(it) => it,
                    | #ControlFlow::EarlyReturn(it) => #wrap_ret,
                    | #ControlFlow::Break(it) => #wrap_break,
                    | #ControlFlow::Continue(it) => #wrap_continue,
                }
            }));
        }
        self.dyn_safe_calls = orig_dyn_safe_calls;
        // sub-recurse.
        block
            .stmts
            .iter_mut()
            .for_each(|stmt| self.visit_stmt_mut(stmt))
        ;
    }

    /// The following function is not necessary, but it leads to nicer
    /// error messages if the `#[with]` attribute is misplaced.
    ///
    /// Indeed, imagine someone annotating an assignment instead of a
    /// new `let` binding. In that case, the previous visitor will not
    /// catch it, thus leaving the attribute as is, which not only makes no
    /// sense, it can also trigger things such as:
    ///
    /// ```text
    /// error[E0658]: attributes on expressions are experimental
    /// ```
    ///
    /// This visitor will try to catch that, and provide a nicer error
    /// message.
    fn visit_attribute_mut (
        self: &'_ mut Self,
        attr: &'_ mut Attribute,
    )
    {
        if attr.path.is_ident("with") {
            panic!(*self.encountered_error = Some(Error::new(
                attr.span(),
                "`#[with]` must be applied to a `let` binding.",
            )));
        }
        // visit_mut::visit_attribute_mut(self, attr); /* No need */
    }
}
