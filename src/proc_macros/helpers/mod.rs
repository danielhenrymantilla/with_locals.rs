use super::*;

#[macro_use]
mod macros;

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

#[cfg(feature = "expand-macros")]
pub(in super)
fn pretty_print_tokenstream (
    code: &'_ TokenStream2,
    fname: &'_ Ident,
)
{
    fn try_format (input: &'_ str)
      -> Option<String>
    {Some({
        let mut child =
            ::std::process::Command::new("rustfmt")
                .stdin(::std::process::Stdio::piped())
                .stdout(::std::process::Stdio::piped())
                .stderr(::std::process::Stdio::piped())
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
        if child.wait().ok()?.success().not() { return None; }
        stdout
    })}

    if  ::std::env::var("WITH_LOCALS_DEBUG_FILTER")
            .ok()
            .map_or(true, |ref filter| {
                fname
                    .to_string()
                    .contains(filter)
            })
    {
        if let Some(ref formatted) = try_format(&code.to_string()) {
            // It's formatted, now let's try to also colorize it:
            if  ::bat::PrettyPrinter::new()
                    .input_from_bytes(formatted.as_ref())
                    .language("rust")
                    .true_color(false)
                    .print()
                    .is_err()
            {
                // Fallback to non-colorized-but-formatted output.
                println!("{}", formatted);
            }
        } else {
            // Fallback to raw output.
            println!("{}", code);
        }
    }
}
