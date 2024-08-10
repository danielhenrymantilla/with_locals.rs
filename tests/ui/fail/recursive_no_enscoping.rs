include!("../prelude.rs");

struct Implementor;

impl Implementor {
    #[with('local, recursive = true)]
    fn foo (&self)
      -> &'local ()
    {
        &()
    }

    #[with('local, recursive = true)]
    fn bar (self: &'_ Self)
      -> &'local ()
    {
        &()
    }
}

trait Trait {
    #[with('local, recursive = true)]
    fn foo (&self)
      -> &'local ()
    {
        &()
    }

    #[with('local, recursive = true)]
    fn bar (self: &'_ Self)
      -> &'local ()
    {
        &()
    }
}
