#![forbid(unsafe_code)]

#![cfg_attr(feature = "nightly",
    feature(external_doc),
    doc(include = "../README.md"),
)]

use ::core::ops::Not as _;

use ::proc_macro::{
    TokenStream,
};
use ::proc_macro2::{
    Span,
    TokenStream as TokenStream2,
};
use ::quote::{
    format_ident,
    // quote,
    // quote_spanned,
    ToTokens,
};
use ::syn::{*,
    parse::{
        Nothing,
        Parse,
        // Parser,
        ParseStream,
    },
    // punctuated::Punctuated,
    spanned::Spanned,
    Result,
    visit_mut::{self, VisitMut},
};

enum Input {
    TraitItemMethod(TraitItemMethod),
    ImplItemMethod(ImplItemMethod),
    ItemFn(ItemFn),
}

impl Parse for Input {
    fn parse (input: ParseStream<'_>)
      -> Result<Self>
    {
        // FIXME: this could be optimized, but `syn` does not export its
        // internal `parse_visibility` helper function.
        // For the sake of simplicity, use this naive approach for now.
        use ::syn::parse::discouraged::Speculative;
        let ref fork = input.fork();
        if let Ok(it) = fork.parse::<TraitItemMethod>() {
            input.advance_to(fork);
            return Ok(Self::TraitItemMethod(it));
        }
        let ref fork = input.fork();
        if let Ok(it) = fork.parse::<ImplItemMethod>() {
            input.advance_to(fork);
            return Ok(Self::ImplItemMethod(it));
        }
        let ref fork = input.fork();
        match fork.parse::<ItemFn>() {
            | Ok(it) => {
                input.advance_to(fork);
                Ok(Self::ItemFn(it))
            },
            | Err(err) => {
                // Here we could directly err with `err`, but in case the
                // user is annotating a stmt or an expr, which is allowed
                // as long as the enscoping function is annotated (preprocessor
                // pattern), a more useful error message than "expected `fn`"
                // could be generated.
                // Yes, I do care about nice error messages!
                const MSG: &str =
                    "Missing `#[with]` annotation on the enscoping function"
                ;
                let span = Span::call_site();
                // That being said, an item can be seen as an `Item::Stmt`,
                // so make sure to bail out if that's the case.
                let ref fork = input.fork();
                match fork.parse::<Stmt>() {
                    | Err(_)
                    | Ok(Stmt::Item(_)) => {},
                    | Ok(_) => return Err(Error::new(span, MSG)),
                }
                // Ditto for `Expr`: a `union ...` can be parsed as one...
                let ref fork = input.fork();
                match fork.parse::<Expr>() {
                    | Err(_) => {},
                    | Ok(Expr::Path(ExprPath {
                        qself: None,
                        path,
                        ..
                    }))
                        if path.is_ident("union")
                    => {},

                    | Ok(_) => {
                        return Err(Error::new(span, MSG));
                    },
                }
                Err(err)
            }
        }
    }
}

type Str = ::std::borrow::Cow<'static, str>;

struct Attrs {
    lifetime: Str,
    continuation: Option<Ident>,
}

mod kw {
    ::syn::custom_keyword!(continuation_name);
}

impl Parse for Attrs {
    fn parse (input: ParseStream<'_>)
      -> Result<Self>
    {
        let mut ret = Self {
            lifetime: "ref".into(),
            continuation: None,
        };
        if let Some(lt) = input.parse::<Option<Lifetime>>()? {
            ret.lifetime = lt.ident.to_string().into();
            if input.parse::<Option<Token![,]>>()?.is_none() {
                return Ok(ret);
            }
        }
        if input.peek(kw::continuation_name) {
            input.parse::<kw::continuation_name>().unwrap();
            input.parse::<Token![=]>()?;
            ret.continuation.replace(input.parse()?);
            input.parse::<Option<Token![,]>>()?;
        }
        Ok(ret)
    }
}

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

    handle_returning_locals(&mut *fun, attrs);
    if let Err(err) = handle_let_bindings(&mut *fun, attrs) {
        return err.to_compile_error().into();
    }

    let ret = fun.to_token_stream();

    #[cfg(feature = "verbose-expansions")] {
    if  ::std::env::var("WITH_LOCALS_DEBUG_FILTER")
            .ok()
            .map_or(true, |ref filter| {
                fun .fields()
                    .sig
                    .ident
                    .to_string()
                    .contains(filter)
            })
    {
        if let Some(ref formatted) = helpers::rustfmt(&ret.to_string()) {
            if  ::bat::PrettyPrinter::new()
                    .input_from_bytes(formatted.as_ref())
                    .language("rust")
                    .true_color(false)
                    .print()
                    .is_err()
            {
                println!("{}", formatted);
            }
        } else {
            println!("{}", ret);
        }
    }}

    ret.into()
}

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
fn handle_let_bindings (
    fun: &'_ mut Input,
    &Attrs { ref lifetime, .. }: &'_ Attrs,
) -> Result<()>
{Ok({
    struct ReplaceLetBindingsWithWithCalls<'__> {
        encountered_error: &'__ mut Option<::syn::Error>,
        lifetime: &'__ str,
    }
    impl VisitMut for ReplaceLetBindingsWithWithCalls<'_> {
        fn visit_block_mut (
            self: &'_ mut Self,
            block: &'_ mut Block,
        )
        {
            macro_rules! throw {
                ( $span:expr => $err_msg:expr $(,)? ) => ({
                    self.encountered_error.replace(
                        Error::new($span, $err_msg)
                    );
                    panic!();
                });

                ( $err_msg:expr $(,)? ) => (
                    throw! { Span::call_site() => $err_smg }
                );
            }

            let mut with_idx = None;
            for (i, stmt) in (0 ..).zip(&mut block.stmts) {
                // `( #[with] )? let <binding> (: <ty>)? = <expr>;`
                if let Stmt::Local(ref mut let_binding) = *stmt {
                    let mut has_with = false;
                    let_binding.attrs.retain(|attr| {
                        if attr.path.is_ident("with") {
                            has_with = true;
                            if let Err(err) =
                                ::syn::parse2::<Nothing>(attr.tokens.clone())
                            {
                                panic!(*self.encountered_error = Some(err));
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
                let init =
                    if let Some(it) = let_assign.init.take() { it } else {
                        throw!(let_assign.semi_token.span() =>
                            "Missing expression"
                        );
                    }
                ;
                let mut call = *init.1;
                let (attrs, args, func) = match call {
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

                    | ref extraneous => throw!(extraneous.span() =>
                        "`function(...)` or `<expr>.method(...)` expected"
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

                block.stmts.push(Stmt::Expr(call));
            }
            block.stmts.iter_mut().for_each(|stmt| self.visit_stmt_mut(stmt));
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

        fn visit_item_mut (
            self: &'_ mut Self,
            _: &'_ mut Item,
        )
        {
            // Do not recurse into items defined inside the function body.
        }
    }

    let mut encountered_error = None;
    let mut visitor = ReplaceLetBindingsWithWithCalls {
        encountered_error: &mut encountered_error,
        lifetime: &*lifetime,
    };
    use ::std::panic;
    if let Err(panic) = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        match *fun {
            | Input::ItemFn(ref mut it) => {
                visitor.visit_item_fn_mut(it);
            },
            | Input::TraitItemMethod(ref mut it) => {
                visitor.visit_trait_item_method_mut(it);
            },
            | Input::ImplItemMethod(ref mut it) => {
                visitor.visit_impl_item_method_mut(it);
            },
        }
    }))
    {
        if let Some(err) = encountered_error {
            return Err(err);
        } else {
            panic::resume_unwind(panic);
        }
    }
})}

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
                                {
                                    // use #krate::ResultOptionHack;
                                    match #inner_expr/*.into_result_hack()*/ {
                                        | Ok(it) => it,
                                        | Err(err) => return __continuation__(Err(err).into()),
                                    }
                                }
                            };
                        }

                        | _ => {
                            // sub-recurse
                            visit_mut::visit_expr_mut(self, expr);
                        },
                    }
                }
            }
            ReturnMapper.visit_block_mut(block);
            attrs.push(parse_quote! {
                #[allow(unreachable_code, unused_braces)]
            })
        }
        *block = parse_quote!({
            // Some user-provided code patterns, once transformed, may scare
            // Rust into thinking we are calling an `FnOnce()` multiple times.
            // Since that _shouldn't_ be the case, we defer to a runtime check,
            // hoping that, in practice, it will end up being optimized away.
            let mut #continuation_name = {
                let mut #continuation_name =
                    ::core::option::Option::Some(#continuation_name)
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

use helpers::{Fields as __, LifetimeVisitor};
mod helpers {
    use super::*;

    impl ToTokens for Input {
        fn to_tokens (self: &'_ Self, out: &'_ mut TokenStream2)
        {
            match *self {
                | Input::ItemFn(ref inner) => inner.to_tokens(out),
                | Input::TraitItemMethod(ref inner) => inner.to_tokens(out),
                | Input::ImplItemMethod(ref inner) => inner.to_tokens(out),
            }
        }
    }

    pub (in super)
    struct LifetimeVisitor<'__> {
        pub (in super)
        lifetime: &'__ str,

        pub (in super)
        lifetimes: &'__ mut Vec<(/*Lifetime*/)>,
    }

    impl VisitMut for LifetimeVisitor<'_> {
        fn visit_lifetime_mut (
            self: &'_ mut Self,
            lifetime: &'_ mut Lifetime,
        )
        {
            if lifetime.ident == self.lifetime {
                // lifetime.ident = format_ident!(
                //     "__self_{}__", self.lifetimes.len(),
                //     span = lifetime.ident.span(),
                // );
                lifetime.ident =
                    format_ident!("_", span = lifetime.ident.span())
                ;
                self.lifetimes.push({ /* lifetime.clone() */ });
            }
        }
    }

    pub(in super)
    struct Fields<'fun> {
        pub(in super) attrs: &'fun mut Vec<Attribute>,
        // pub(in super) vis: Option<&'fun mut Visibility>,
        pub(in super) sig: &'fun mut Signature,
        pub(in super) block: Option<&'fun mut Block>,
    }

    impl Input {
        pub(in super)
        fn fields (self: &'_ mut Self) -> Fields<'_>
        {
            match *self {
                | Self::ItemFn(ItemFn {
                    ref mut attrs,
                    // ref mut vis,
                    ref mut sig,
                    ref mut block,
                    ..
                })
                => Fields {
                    attrs,
                    // vis: Some(vis),
                    sig,
                    block: Some(block),
                },

                | Self::ImplItemMethod(ImplItemMethod {
                    ref mut attrs,
                    // ref mut vis,
                    ref mut sig,
                    ref mut block,
                    ..
                })
                => Fields {
                    attrs,
                    // vis: Some(vis),
                    sig,
                    block: Some(block),
                },

                | Self::TraitItemMethod(TraitItemMethod {
                    ref mut attrs,
                    ref mut sig,
                    default: ref mut block,
                    ..
                })
                => Fields {
                    attrs,
                    // vis: None,
                    sig,
                    block: block.as_mut(),
                },
            }
        }
    }

    #[cfg(feature = "verbose-expansions")]
    pub(in crate)
    fn rustfmt (input: &'_ str)
      -> Option<String>
    {Some({
        let mut child =
            ::std::process::Command::new("rustfmt")
                .stdin(::std::process::Stdio::piped())
                .stdout(::std::process::Stdio::piped())
                .spawn()
                .ok()?
        ;
        match child.stdin.take().unwrap() { ref mut stdin => {
            ::std::io::Write::write_all(stdin, input.as_bytes()).ok()?;
        }}
        let mut stdout = String::new();
        ::std::io::Read::read_to_string(
            &mut child.stdout.take().unwrap(),
            &mut stdout,
        ).ok()?;
        child.wait().ok()?;
        stdout
    })}
}
