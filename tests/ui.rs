#[test]
fn trybuild ()
{
    if ::std::env::var("CI_SKIP_UI_TESTS").ok().map_or(false, |s| s == "1") {
        return;
    }
    let ref tests_ui =
        ::std::path::Path::new("tests")
            .join("ui")
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
    } else {
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
    }
}
