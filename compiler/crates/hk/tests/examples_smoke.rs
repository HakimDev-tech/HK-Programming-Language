use hk::{run, run_path};

#[test]
fn all_documented_examples_execute_with_expected_output() {
    let examples: [(&str, &str, &[&str]); 5] = [
        (
            "examples/hello.hk",
            include_str!("../../../../examples/hello.hk"),
            &["Hello, HK!", "0.1"],
        ),
        (
            "examples/arrays.hk",
            include_str!("../../../../examples/arrays.hk"),
            &["Ada", "Grace", "Linus"],
        ),
        (
            "examples/control-flow.hk",
            include_str!("../../../../examples/control-flow.hk"),
            &["The sum is 15."],
        ),
        (
            "examples/functions.hk",
            include_str!("../../../../examples/functions.hk"),
            &["120"],
        ),
        (
            "examples/gradual_typing.hk",
            include_str!("../../../../examples/gradual_typing.hk"),
            &["84"],
        ),
    ];

    for (name, source, expected) in examples {
        let actual = run(name, source)
            .unwrap_or_else(|error| panic!("{name} should execute successfully: {error}"));
        let expected = expected
            .iter()
            .map(|line| (*line).to_owned())
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "unexpected output from {name}");
    }
}

#[test]
fn module_example_executes_through_filesystem_loader() {
    let entry =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../examples/modules/main.hk");
    let actual = run_path(entry).expect("the module example should execute successfully");
    assert_eq!(actual, vec!["counter initialized", "3", "3", "9", "18"]);
}
