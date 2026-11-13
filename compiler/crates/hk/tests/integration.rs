use hk::run;
#[test] fn end_to_end_program(){
    let source=r#"
        const school = "HK Academy";
        let grades = [15, 12, 17];
        fn sum(values) {
            let total = 0;
            for (let i = 0; i < values.length; i = i + 1) {
                total = total + values[i];
            }
            return total;
        }
        let total = sum(grades);
        print(school);
        print(total);
        if (total >= 30 && true) { print("passed"); } else { print("failed"); }
    "#;
    assert_eq!(run("integration.hk",source).unwrap(),vec!["HK Academy","44","passed"]);
}
