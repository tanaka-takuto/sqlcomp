use sqlay_core as core;

use super::diagnostics::{query_error, with_slot_variant_context};
use super::slot_variants::AnalyzedQueryVariant;

pub(super) fn validate_variant_cardinality(
    variants: &[AnalyzedQueryVariant],
) -> core::DiagnosticResult<()> {
    let Some(base_variant) = variants.first() else {
        return Ok(());
    };
    let base_cardinality = effective_cardinality(&base_variant.query, &base_variant.analysis);

    for variant in variants.iter().skip(1) {
        let variant_cardinality = effective_cardinality(&variant.query, &variant.analysis);
        if variant_cardinality != base_cardinality {
            return Err(with_slot_variant_context(
                query_error(
                    &variant.query,
                    format!(
                        "Slot expansion variant for query `{}` resolved effective cardinality `{}`, but the base variant resolved effective cardinality `{}`; all variants must have matching effective cardinality, using an explicit query metadata `cardinality` override when present and dialect analysis otherwise",
                        variant.query.metadata().id(),
                        format_cardinality(variant_cardinality),
                        format_cardinality(base_cardinality),
                    ),
                ),
                variant.context.as_ref(),
            ));
        }
    }

    Ok(())
}

pub(super) fn validate_variant_row_shape(
    base_metadata: &core::DbQueryMetadata,
    variant: &AnalyzedQueryVariant,
    variant_metadata: &core::DbQueryMetadata,
) -> core::DiagnosticResult<()> {
    let base_columns = base_metadata.columns();
    let variant_columns = variant_metadata.columns();

    if variant_columns.len() != base_columns.len() {
        return Err(with_slot_variant_context(
            query_error(
                &variant.query,
                format!(
                    "Slot expansion variant for query `{}` returned {} result columns, but the base variant returned {}; all variants must have matching result row shape",
                    variant.query.metadata().id(),
                    variant_columns.len(),
                    base_columns.len(),
                ),
            ),
            variant.context.as_ref(),
        ));
    }

    for (index, (base_column, variant_column)) in
        base_columns.iter().zip(variant_columns).enumerate()
    {
        let column_number = index + 1;
        if variant_column.name() != base_column.name() {
            let difference = format!(
                "result column {column_number} name `{}` does not match base column name `{}`",
                variant_column.name(),
                base_column.name(),
            );
            return Err(row_shape_difference_error(variant, &difference));
        }
        if variant_column.ty() != base_column.ty() {
            let difference = format!(
                "result column {column_number} CoreType `{:?}` does not match base CoreType `{:?}`",
                variant_column.ty(),
                base_column.ty(),
            );
            return Err(row_shape_difference_error(variant, &difference));
        }
        if variant_column.is_nullable_for_output() != base_column.is_nullable_for_output() {
            let difference = format!(
                "result column {column_number} nullability `{}` does not match base nullability `{}`",
                format_nullability(variant_column.is_nullable_for_output()),
                format_nullability(base_column.is_nullable_for_output()),
            );
            return Err(row_shape_difference_error(variant, &difference));
        }
    }

    Ok(())
}

fn row_shape_difference_error(
    variant: &AnalyzedQueryVariant,
    difference: &str,
) -> core::DiagnosticReport {
    with_slot_variant_context(
        query_error(
            &variant.query,
            format!(
                "Slot expansion variant for query `{}` {difference}; all variants must have matching result row shape",
                variant.query.metadata().id(),
            ),
        ),
        variant.context.as_ref(),
    )
}

fn effective_cardinality(
    query: &core::RawQuery,
    analysis: &core::AnalyzedQuery,
) -> core::Cardinality {
    query
        .metadata()
        .cardinality()
        .unwrap_or_else(|| analysis.cardinality())
}

const fn format_nullability(nullable: bool) -> &'static str {
    if nullable { "nullable" } else { "not nullable" }
}

const fn format_cardinality(cardinality: core::Cardinality) -> &'static str {
    match cardinality {
        core::Cardinality::One => "one",
        core::Cardinality::Many => "many",
    }
}
