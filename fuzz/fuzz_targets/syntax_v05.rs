#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &[u8]| {
    if let Ok(source) = std::str::from_utf8(input) {
        match hk::syntax_v05::lex_v05(source) {
            Ok(tokens) => {
                let _ = hk::syntax_v05::parse_nominal_declarations(&tokens);
                let _ = hk::syntax_v05::parse_trait_and_impl_declarations(&tokens);
                let _ = hk::syntax_v05::parse_function_declarations(&tokens);
                let mut cursor = 0;
                let _ = hk::syntax_v05::parse_struct_literal(&tokens, &mut cursor);
                cursor = 0;
                let _ = hk::syntax_v05::parse_enum_constructor(&tokens, &mut cursor);
                let _ = hk::syntax_v05::parse_type_expr(&tokens, &mut cursor);
                cursor = 0;
                let _ = hk::syntax_v05::parse_generic_parameters(&tokens, &mut cursor);
            }
            Err(_) => {}
        }
    }
});
