#[test]
fn trybuild ()
{
    let ref tests_ui =
        ::std::path::Path::new("tests")
            .join("ui")
    ;
    ::trybuild::TestCases::new()
        .compile_fail(
            tests_ui
                .join("fail/*.rs")
        )
    ;
    ::trybuild::TestCases::new()
        .pass(
            tests_ui
                .join("pass/*.rs")
        )
    ;
    let nightly = {
        fn _it () -> bool
        {
            ::std::env::var("RUSTC_BOOTSTRAP")
                .ok()
                .map_or(false, |s| s == "1")
        }
        {
            #[::rustversion::nightly]
            fn _it () -> bool { true }
            _it()
        }
    };
    if nightly {
        ::trybuild::TestCases::new()
            .compile_fail(
                tests_ui
                    .join("fail")
                    .join("nightly/*.rs")
            )
        ;
        ::trybuild::TestCases::new()
            .pass(
                tests_ui
                    .join("pass")
                    .join("nightly/*.rs")
            )
        ;
    };
}
