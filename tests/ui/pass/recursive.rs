#![forbid(unsafe_code)]

include!("../prelude.rs");

struct Implementor;

#[with]
impl Implementor {
    #[with('local, recursive = true)]
    fn recurse (&self, recurse: bool)
      -> &'local ()
    {
        if recurse {
            let _: &'local _ = self.recurse(false);
        }
        &()
    }
}

#[with]
trait Trait {
    #[with('local, recursive = true)]
    fn recurse (&self, recurse: bool)
      -> &'local ()
    {
        if recurse {
            let _: &'local _ = self.recurse(false);
        }
        &()
    }
}
