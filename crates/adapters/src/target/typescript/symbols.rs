use sqlay_core as core;

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

/// TypeScript symbols generated from one compiled mutation ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MutationSymbols {
    function: String,
    input_type: String,
}

impl MutationSymbols {
    /// Build TypeScript symbol names for a compiled mutation.
    #[must_use]
    pub fn for_mutation(mutation: &core::CompiledMutation) -> Self {
        Self::from_mutation_id(mutation.id().as_str())
    }

    /// Build TypeScript symbol names from a validated mutation ID.
    #[must_use]
    pub fn from_mutation_id(mutation_id: &str) -> Self {
        Self {
            function: mutation_id.to_owned(),
            input_type: format!("{mutation_id}_Input"),
        }
    }

    /// Generated mutation builder function name.
    #[must_use]
    pub fn function_name(&self) -> &str {
        &self.function
    }

    /// Generated input type alias name.
    #[must_use]
    pub fn input_type_name(&self) -> &str {
        &self.input_type
    }
}
