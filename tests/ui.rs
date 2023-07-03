#[test]
fn trybuild ()
{
    let ref tests_ui =
        ::std::path::Path::new("tests")
            .join("ui")
    ;
    let nightly =
        ::std::env::var("RUSTC_BOOTSTRAP")
            .ok()
            .map_or(false, |s| s == "1")
    ;
    ::trybuild::TestCases::new()
        .pass(
            tests_ui
                .join("pass/*.rs")
        )
    ;
    if nightly {
        ::trybuild::TestCases::new()
            .pass(
                tests_ui
                    .join("pass")
                    .join("nightly/*.rs")
            )
        ;
    }
    if ::std::env::var("CI_SKIP_UI_TESTS").ok().map_or(false, |s| s == "1") {
        return;
    }
    if nightly {
        ::trybuild::TestCases::new()
            .compile_fail(
                tests_ui
                    .join("fail")
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
    }
}
