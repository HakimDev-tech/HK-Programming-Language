//! Internal benchmark-only entry points. Enabled only with the bench-internals feature.
use super::{compiler, verifier::VerifiedProgram, vm, HkError};

/// A parsed, statically checked, compiled, and verified source program.
#[doc(hidden)]
pub struct PreparedProgram {
    verified: VerifiedProgram,
    source_name: String,
    source_text: String,
}

/// Measures front-end, compiler, and verifier cost without executing the program.
#[doc(hidden)]
pub fn compile_source(source_name: &str, source_text: &str) -> Result<PreparedProgram, HkError> {
    let verified = compiler::compile_source(source_name, source_text)?;
    Ok(PreparedProgram {
        verified,
        source_name: source_name.to_owned(),
        source_text: source_text.to_owned(),
    })
}

impl PreparedProgram {
    /// Executes the already verified bytecode without recompilation or cloning it.
    #[doc(hidden)]
    pub fn execute(&self) -> Result<Vec<String>, HkError> {
        vm::execute_ref(&self.verified, &self.source_name, &self.source_text)
    }
}
