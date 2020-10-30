//! Uninteresting parsing impls

use super::*;

pub(in crate)
struct Attrs {
    pub lifetime: Str,
    pub continuation: Option<Ident>,
    pub dyn_safe: bool,
    pub recursive: bool,
}

impl Parse for Attrs {
    fn parse (input: ParseStream<'_>)
      -> Result<Self>
    {
        let mut ret = Self {
            lifetime: "ref".into(),
            continuation: None,
            dyn_safe: false,
            recursive: false,
        };
        if let Some(lt) = input.parse::<Option<Lifetime>>()? {
            ret.lifetime = lt.ident.to_string().into();
            if input.parse::<Option<Token![,]>>()?.is_none() {
                return Ok(ret);
            }
        }
        mod kw {
            ::syn::custom_keyword!(continuation_name);
            ::syn::custom_keyword!(dyn_safe);
            ::syn::custom_keyword!(recursive);
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
                | _case if input.peek(kw::dyn_safe) => {
                    input.parse::<kw::dyn_safe>().unwrap();
                    input.parse::<Token![=]>()?;
                    let bool_literal: LitBool = input.parse()?;
                    ret.dyn_safe = bool_literal.value;
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
