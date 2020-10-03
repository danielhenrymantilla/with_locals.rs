macro_rules! mk_throw {(
    #![dollar = $__:tt]
    $throw:ident ! in $encountered_error:expr
) => (
    macro_rules! $throw {
        ( $span:expr => $err_msg:expr $__(,)? ) => ({
            $encountered_error.replace(
                Error::new($span, $err_msg)
            );
            panic!();
        });

        ( $err_msg:expr $__(,)? ) => (
            throw! { Span::call_site() => $err_smg }
        );
    }
)}

macro_rules! proc_macro_use {(
    use $dol:tt $krate:ident::{$($item:ident),* $(,)? };
) => (
    let $krate = quote! {
        ::with_locals::__
    };
    drop(&$krate);
    $(
        #[allow(nonstandard_style)]
        let $item = quote! {
            ::with_locals::__::$item
        };
        drop(&$krate);
    )*
)}
