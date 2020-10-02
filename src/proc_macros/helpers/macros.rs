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

// macro_rules! my_parse_quote {(
//     $($tt:tt)*
// ) => ({
//     #[cfg(feature = "verbose-expansions")]
//     let storage;
//     let mut s = &quote!( $($tt)* ).to_string();
//     s = s;
//     #[cfg(feature = "verbose-expansions")] {
//         if let Some(formatted) = crate::helpers::rustfmt(s) {
//             storage = formatted;
//             s = &storage
//         };
//     }
//     eprintln!("`parse_quote!` input:\n{}", s);
//     ::syn::parse_quote!( $($tt)* )
// })}

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
