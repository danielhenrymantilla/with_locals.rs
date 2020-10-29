#![forbid(unsafe_code)]

include!("../prelude.rs");

struct Implementor;

#[with]
impl Implementor {
    #[with(recursive = true)]
    fn recurse (&self, recurse: bool)
      -> &'ref ()
    {
        if recurse {
            let _: &'ref _ = self.recurse(false);
        }
        &()
    }
}

#[with]
trait Trait {
    #[with(recursive = true)]
    fn recurse (&self, recurse: bool)
      -> &'ref ()
    {
        if recurse {
            let _: &'ref _ = self.recurse(false);
        }
        &()
    }
}
