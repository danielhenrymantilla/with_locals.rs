use super::*;

#[macro_use]
mod macros;

pub(in crate)
struct LifetimeVisitor<'__> {
    pub(in crate)
    lifetime: &'__ str,

    pub(in crate)
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

pub(in crate)
struct Fields<'fun> {
    pub attrs: &'fun mut Vec<Attribute>,
    // pub vis: Option<&'fun mut Visibility>,
    pub sig: &'fun mut Signature,
    pub block: Option<&'fun mut Block>,
}

pub(in crate)
trait FnLike {
    fn fields<'fun> (self: &'fun mut Self)
      -> Fields<'fun>
    ;
}

impl FnLike for ItemFn {
    fn fields<'fun> (self: &'fun mut ItemFn)
      -> Fields<'fun>
    {
        let ItemFn {
            ref mut attrs,
            // ref mut vis,
            ref mut sig,
            ref mut block,
            ..
        } = *self;
        Fields {
            attrs,
            // vis: Some(vis),
            sig,
            block: Some(block),
        }
    }
}

impl FnLike for ImplItemMethod {
    fn fields<'fun> (self: &'fun mut ImplItemMethod)
      -> Fields<'fun>
    {
        let ImplItemMethod {
            ref mut attrs,
            // ref mut vis,
            ref mut sig,
            ref mut block,
            ..
        } = *self;
        Fields {
            attrs,
            // vis: Some(vis),
            sig,
            block: Some(block),
        }
    }
}

impl FnLike for TraitItemMethod {
    fn fields<'fun> (self: &'fun mut TraitItemMethod)
      -> Fields<'fun>
    {
        let TraitItemMethod {
            ref mut attrs,
            ref mut sig,
            default: ref mut block,
            ..
        } = *self;
        Fields {
            attrs,
            // vis: None,
            sig,
            block: block.as_mut(),
        }
    }
}

#[cfg(feature = "expand-macros")]
pub(in crate)
fn pretty_print_tokenstream (
    code: &'_ TokenStream2,
    fname: &'_ str,
)
{
    fn try_format (input: &'_ str)
      -> Option<String>
    {Some({
        let mut child =
            ::std::process::Command::new("rustfmt")
                .args(&["--edition", "2018"])
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
            .map_or(true, |ref filter| fname.contains(filter))
    {
        if let Some(ref formatted) = try_format(&code.to_string()) {
            println!("{}", formatted);
        } else {
            // Fallback to raw output.
            println!("{}", code);
        }
    }
}
