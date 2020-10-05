//! Uninteresting parsing impls

use super::*;

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

mod kw {
    ::syn::custom_keyword!(continuation_name);
    ::syn::custom_keyword!(recursive);
}

impl Parse for Attrs {
    fn parse (input: ParseStream<'_>)
      -> Result<Self>
    {
        let mut ret = Self {
            lifetime: "ref".into(),
            continuation: None,
            recursive: false,
        };
        if let Some(lt) = input.parse::<Option<Lifetime>>()? {
            ret.lifetime = lt.ident.to_string().into();
            if input.parse::<Option<Token![,]>>()?.is_none() {
                return Ok(ret);
            }
        }
        while input.is_empty().not() {
            match () {
                | _case if input.peek(kw::recursive) => {
                    input.parse::<kw::recursive>().unwrap();
                    input.parse::<Token![=]>()?;
                    let bool_literal: LitBool = input.parse()?;
                    ret.recursive = bool_literal.value;
                    input.parse::<Option<Token![,]>>()?;
                },
                | _case if input.peek(kw::continuation_name) => {
                    input.parse::<kw::continuation_name>().unwrap();
                    input.parse::<Token![=]>()?;
                    ret.continuation.replace(input.parse()?);
                    input.parse::<Option<Token![,]>>()?;
                },
                | _default => break,
            }
        }
        Ok(ret)
    }
}
