//! TypeScript target generation adapter.

use sqlcomp_app::TargetGenerator;
use sqlcomp_core as core;

/// TypeScript symbols generated from one compiled query ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuerySymbols {
    function: String,
    input_type: String,
    row_type: String,
    output_type: String,
}

impl QuerySymbols {
    /// Build TypeScript symbol names for a compiled query.
    #[must_use]
    pub fn for_query(query: &core::CompiledQuery) -> Self {
        Self::from_query_id(query.id().as_str())
    }

    /// Build TypeScript symbol names from a validated query ID.
    #[must_use]
    pub fn from_query_id(query_id: &str) -> Self {
        Self {
            function: query_id.to_owned(),
            input_type: format!("{query_id}_Input"),
            row_type: format!("{query_id}_Row"),
            output_type: format!("{query_id}_Output"),
        }
    }

    /// Generated query builder function name.
    #[must_use]
    pub fn function_name(&self) -> &str {
        &self.function
    }

    /// Generated input type alias name.
    #[must_use]
    pub fn input_type_name(&self) -> &str {
        &self.input_type
    }

    /// Generated result row type alias name.
    #[must_use]
    pub fn row_type_name(&self) -> &str {
        &self.row_type
    }

    /// Generated output type alias name.
    #[must_use]
    pub fn output_type_name(&self) -> &str {
        &self.output_type
    }
}

/// Dummy TypeScript target generator.
#[derive(Clone, Copy, Debug, Default)]
pub struct TypeScriptTargetGenerator;

impl TargetGenerator for TypeScriptTargetGenerator {
    fn generate(
        &self,
        _queries: &[core::CompiledQuery],
    ) -> core::DiagnosticResult<core::GeneratedFiles> {
        Ok(core::GeneratedFiles::new(Vec::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::QuerySymbols;
    use sqlcomp_core as core;

    #[test]
    fn query_symbols_use_id_exactly_with_fixed_suffixes() {
        for query_id in ["listUsers", "list_users", "_findUser2", "HTTPStatus200"] {
            let symbols = QuerySymbols::from_query_id(query_id);

            assert_eq!(symbols.function_name(), query_id);
            assert_eq!(symbols.input_type_name(), format!("{query_id}_Input"));
            assert_eq!(symbols.row_type_name(), format!("{query_id}_Row"));
            assert_eq!(symbols.output_type_name(), format!("{query_id}_Output"));
        }
    }

    #[test]
    fn query_symbols_are_derived_from_compiled_query_id_without_transformation() {
        let query = core::CompiledQuery::new(
            core::QueryId::new("list_users".to_owned()),
            "SELECT id FROM users;".to_owned(),
            core::Cardinality::Many,
            Vec::new(),
            Vec::new(),
        );

        let symbols = QuerySymbols::for_query(&query);

        assert_eq!(symbols.function_name(), "list_users");
        assert_eq!(symbols.input_type_name(), "list_users_Input");
        assert_eq!(symbols.row_type_name(), "list_users_Row");
        assert_eq!(symbols.output_type_name(), "list_users_Output");
    }
}
