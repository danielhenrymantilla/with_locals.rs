include!("../prelude.rs");

#[with]
use self;

#[with]
const _: () = {};

const _: () = {
    #[with]
    static __: () = {};
};

const _: () = {
    #[with]
    type __ = ();
};

const _: () = {
    #[with]
    struct __ {}
};

const _: () = {
    #[with]
    enum __ {}
};

const _: () = {
    #[with]
    union __ {}
};
