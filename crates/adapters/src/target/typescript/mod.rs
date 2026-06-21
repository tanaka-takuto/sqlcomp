//! TypeScript target generation adapter.

mod builders;
mod files;
mod literals;
mod slots;
mod symbols;
#[cfg(test)]
mod tests;
mod types;

pub use builders::{render_generated_file_contents, render_query, render_sql_property};
pub use files::TypeScriptTargetGenerator;
pub use literals::typescript_string_literal;
pub use symbols::QuerySymbols;
