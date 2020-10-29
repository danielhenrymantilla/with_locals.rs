include!("../prelude.rs");

struct Implementor;

impl Implementor {
    #[with(recursive = true)]
    fn foo (&self)
      -> &'ref ()
    {
        &()
    }

    #[with(recursive = true)]
    fn bar (self: &'_ Self)
      -> &'ref ()
    {
        &()
    }
}

trait Trait {
    #[with(recursive = true)]
    fn foo (&self)
      -> &'ref ()
    {
        &()
    }

    #[with(recursive = true)]
    fn bar (self: &'_ Self)
      -> &'ref ()
    {
        &()
    }
}
