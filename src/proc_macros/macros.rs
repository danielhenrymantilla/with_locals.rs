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
