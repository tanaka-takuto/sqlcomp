use sqlcomp_core as core;

use super::super::QuerySymbols;

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
