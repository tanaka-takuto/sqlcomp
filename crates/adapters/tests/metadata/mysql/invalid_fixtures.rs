use sqlx::{Connection, MySqlConnection};

use super::fixture_support::{
    DATABASE_URL_ENV, FRAGMENT_PARAM_INFERENCE_FAILURE, INIT_FIXTURES,
    MUTATION_UNSUPPORTED_INFERENCE_CONTEXT, MYSQL_FIXTURE_LOCK,
    PARAM_CONFLICTING_REPEATED_NULLABILITY, PARAM_CONFLICTING_REPEATED_TYPE,
    PARAM_UNSUPPORTED_INFERENCE_CONTEXT, REPEATED_SLOT_FRAGMENT_PARAM_TYPE_CONFLICT,
    SLOT_VARIANT_ROW_SHAPE_MISMATCH, assert_mysql_invalid_fixture_error_contains,
    execute_fixture_statements,
};

#[test]
#[ignore = "requires a running MySQL service and DATABASE_URL"]
fn mysql_param_invalid_fixtures_report_expected_diagnostics()
-> Result<(), Box<dyn std::error::Error>> {
    let _fixture_lock = MYSQL_FIXTURE_LOCK
        .lock()
        .expect("fixture lock should not be poisoned");
    let database_url = std::env::var(DATABASE_URL_ENV)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let mut connection = runtime.block_on(MySqlConnection::connect(&database_url))?;

    for fixture in INIT_FIXTURES {
        runtime.block_on(execute_fixture_statements(&mut connection, fixture))?;
    }

    let cases = [
        (
            "param_unsupported_inference_context.sql",
            PARAM_UNSUPPORTED_INFERENCE_CONTEXT,
            "Param `lowerVarchar` requires `valueType` because no supported qualified column context was found",
        ),
        (
            "param_conflicting_repeated_type.sql",
            PARAM_CONFLICTING_REPEATED_TYPE,
            "conflicting Param `sameValue` types: first occurrence resolved to Int64 but later occurrence resolved to String",
        ),
        (
            "param_conflicting_repeated_nullability.sql",
            PARAM_CONFLICTING_REPEATED_NULLABILITY,
            "conflicting Param `sameText` nullability: first occurrence is nullable false but later occurrence is nullable true",
        ),
    ];

    for (file_name, source, expected) in cases {
        assert_mysql_invalid_fixture_error_contains(&database_url, file_name, source, expected)?;
    }

    Ok(())
}

#[test]
#[ignore = "requires a running MySQL service and DATABASE_URL"]
fn mysql_slot_fragment_invalid_fixtures_report_expected_diagnostics()
-> Result<(), Box<dyn std::error::Error>> {
    let _fixture_lock = MYSQL_FIXTURE_LOCK
        .lock()
        .expect("fixture lock should not be poisoned");
    let database_url = std::env::var(DATABASE_URL_ENV)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let mut connection = runtime.block_on(MySqlConnection::connect(&database_url))?;

    for fixture in INIT_FIXTURES {
        runtime.block_on(execute_fixture_statements(&mut connection, fixture))?;
    }

    let cases = [
        (
            "fragment_param_inference_failure.sql",
            FRAGMENT_PARAM_INFERENCE_FAILURE,
            "Param `lowerText` requires `valueType` because no supported qualified column context was found",
        ),
        (
            "repeated_slot_fragment_param_type_conflict.sql",
            REPEATED_SLOT_FRAGMENT_PARAM_TYPE_CONFLICT,
            "conflicting Fragment Param `value` type in query `repeatedSlotFragmentParamTypeConflict`, Slot `comparator`, Fragment `equalsValue`",
        ),
        (
            "slot_variant_row_shape_mismatch.sql",
            SLOT_VARIANT_ROW_SHAPE_MISMATCH,
            "Slot expansion variant for query `slotVariantRowShapeMismatch` returned 2 result columns, but the base variant returned 1",
        ),
    ];

    for (file_name, source, expected) in cases {
        assert_mysql_invalid_fixture_error_contains(&database_url, file_name, source, expected)?;
    }

    assert_mysql_invalid_fixture_error_contains(
        &database_url,
        "fragment_param_inference_failure.sql",
        FRAGMENT_PARAM_INFERENCE_FAILURE,
        "while validating Slot expansion variant for query `fragmentParamInferenceFailure` with selections: filter=lowerTextFilter",
    )?;

    Ok(())
}

#[test]
#[ignore = "requires a running MySQL service and DATABASE_URL"]
fn mysql_mutation_invalid_fixtures_report_expected_diagnostics()
-> Result<(), Box<dyn std::error::Error>> {
    let _fixture_lock = MYSQL_FIXTURE_LOCK
        .lock()
        .expect("fixture lock should not be poisoned");
    let database_url = std::env::var(DATABASE_URL_ENV)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let mut connection = runtime.block_on(MySqlConnection::connect(&database_url))?;

    for fixture in INIT_FIXTURES {
        runtime.block_on(execute_fixture_statements(&mut connection, fixture))?;
    }

    assert_mysql_invalid_fixture_error_contains(
        &database_url,
        "mutation_unsupported_inference_context.sql",
        MUTATION_UNSUPPORTED_INFERENCE_CONTEXT,
        "Param `adjustment` requires `valueType` because no supported mutation column context was found",
    )?;

    Ok(())
}
