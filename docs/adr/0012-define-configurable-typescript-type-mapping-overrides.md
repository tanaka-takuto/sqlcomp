# ADR 0012: Define Configurable TypeScript Type Mapping Overrides

## Status

Accepted

## Context

`sqlay` maps database metadata into a language-neutral type model and then emits
TypeScript SQL builder types. The current TypeScript mapping is intentionally
conservative: precision-sensitive values such as `BIGINT` and `DECIMAL` map to
`string`, unknown values map to `unknown`, and MySQL `ENUM` and `SET` values map to
`string`.

Those defaults are safe, but production TypeScript projects often need generated
types that match project conventions: enum typo prevention, domain-specific named
types, or `number` for decimal and bigint values when the application accepts the
precision risk. This design must not turn sqlay into a runtime parser, driver
adapter, or database execution layer.

## Decision

Add a TypeScript target configuration section for type annotation overrides:

```jsonc
{
  "target": {
    "language": "typescript",
    "typescript": {
      "typeMapping": {
        "core": {
          "decimal": "number",
          "int64": "number",
        },
        "columns": {
          "orders.total_amount": {
            "type": "MoneyAmount",
            "import": {
              "from": "@/domain/money",
              "name": "MoneyAmount",
            },
          },
          "billing.orders.status": "BillingOrderStatus",
        },
        "builders": {
          "listOrders": {
            "fields": {
              "totalAmount": {
                "type": "MoneyAmount",
                "import": {
                  "from": "@/domain/money",
                  "name": "MoneyAmount",
                },
              },
            },
            "params": {
              "minimumAmount": {
                "type": "MoneyAmount",
                "import": {
                  "from": "@/domain/money",
                  "name": "MoneyAmount",
                },
              },
            },
            "repeats": {
              "lineItems": {
                "fields": {
                  "unitPrice": {
                    "type": "MoneyAmount",
                    "import": {
                      "from": "@/domain/money",
                      "name": "MoneyAmount",
                    },
                  },
                },
              },
            },
          },
        },
      },
    },
  },
}
```

These overrides change generated TypeScript type annotations only. They do not
parse result values, validate inputs at runtime, configure a database driver, or
change generated SQL. If a project maps `decimal` or `int64` to `number`, the
project is responsible for accepting precision risk and configuring its execution
path consistently.

Override values may use a shorthand string when no import is needed, or an object
with `type` and optional `import`. A `type` must be either a supported TypeScript
primitive such as `number` or a portable TypeScript identifier, not an arbitrary
TypeScript type expression. Complex branded or generic types should be defined by
the user as named type aliases and referenced by name.

When an import is provided, sqlay emits a type-only import:

```ts
import type { MoneyAmount } from "@/domain/money";
```

The import `from` value must be a non-relative module specifier such as
`@/domain/money` or `@acme/domain-types`. Relative paths such as `./money` and
`../money` are rejected because generated files preserve SQL source directory
structure, so one relative path cannot be correct for every generated file. Initial
support does not include import aliases; `import.name` must match `type`. Duplicate
imports from the same module are de-duplicated per generated file, and same local
type names imported from different modules are configuration errors.

Type overrides apply to the whole generated TypeScript type surface:

- SELECT result row fields.
- direct Param input fields.
- Repeat item fields.
- fixed params tuple element types.

Builder-local override paths are intentionally scope-qualified:

- `builders.<id>.fields.<field>` targets SELECT result row fields.
- `builders.<id>.params.<param>` targets direct Param input fields and fixed params
  tuple entries for that Param in static builders.
- `builders.<id>.repeats.<repeat>.fields.<field>` targets item fields under one
  direct Repeat input. The Repeat ID is part of the path because item field names
  are scoped to Repeat inputs and may repeat across different Repeat ranges.

Nullability is preserved. A nullable database column or `nullable: true` Param
becomes `CustomType | null` after a base type override. Dynamic builders that use
Slot or Repeat keep `params: readonly SqlParam[]` with the existing private
`type SqlParam = unknown`; only their input types and result row types are narrowed.

Inline Param `valueType` remains a Core type hint, not a TypeScript type
annotation. For example, `valueType: decimal` classifies the Param as sqlay decimal
metadata; the TypeScript representation may still be changed by `core.decimal`,
`columns`, or `builders` overrides.

Override priority is narrowest-first:

1. Builder-local overrides:
   `builders.<id>.fields.<field>`, `builders.<id>.params.<param>`, and
   `builders.<id>.repeats.<repeat>.fields.<field>`.
2. `columns.<column-reference>`.
3. schema-backed MySQL `ENUM` default literal unions.
4. `core.<core-type>`.
5. sqlay's built-in conservative TypeScript mapping.

Column references are flat strings:

- `table.column` means a table in the connection's current database.
- `database.table.column` means a table in an explicitly named MySQL database.

SQL table sources should support both current-database tables and explicit
`database.table` references as schema-backed sources. Schema-backed type mapping
therefore needs metadata keyed by `(database, table, column)`, not only current
database table names. Derived tables, functions, JSON table sources, expression
result inference, and identifiers containing dots remain outside the initial
schema-backed source model.

MySQL `ENUM` columns that resolve to schema-backed real columns generate inline
TypeScript literal unions by default:

```ts
status: "draft" | "paid" | "shipped";
```

This applies only when sqlay can tie a generated field or Param to a real schema
column. Expression results, function results, CASE expressions, and other
non-schema-backed result values do not infer enum literal unions initially. Enum
values are read from `information_schema.columns.COLUMN_TYPE`, such as
`enum('draft','paid')`, and carried through Core IR as a language-neutral enum value
type. The implementation may extend `CoreType` or introduce a richer type reference,
but enum values must not be TypeScript-only metadata.

MySQL `SET` stays mapped to plain `string` in the initial design. `mysql2/promise`
returns SET values as strings, including comma-separated combinations such as
`"read,write"` and the empty string. A future design may add SET string unions, but
initial support does not emit arrays or generated SET aliases.

Configured overrides must be applied during `check` or `compile`. Unused overrides,
unknown builders, unknown fields, unknown Params, unknown Repeats, unknown Repeat
item fields, and unknown schema columns are configuration errors rather than
warnings.

## Consequences

This feature should be implemented in small dependent slices:

1. Extend configuration parsing and validation for `target.typescript.typeMapping`.
2. Extend MySQL schema metadata to support current-database and `database.table`
   sources keyed by database, table, and column.
3. Add Core type representation for schema-backed enum value sets.
4. Resolve type override priority and detect unused or conflicting overrides.
5. Update TypeScript generation for custom type names, inline enum literal unions,
   type-only imports, import de-duplication, and import conflict diagnostics.
6. Add fixtures and examples for enum literals, decimal and bigint number overrides,
   custom project types, schema-qualified column references, and invalid config
   diagnostics.
7. Update user-facing documentation for annotation-only behavior and runtime
   responsibility.

## Alternatives Considered

Infer enum literal unions only by opt-in. This was rejected because schema-backed
MySQL enum columns have a precise value set and a literal union improves generated
API quality without runtime conversion.

Infer enum literal unions for every MySQL expression result reported as `ENUM`.
This was rejected for initial support because it is unclear whether metadata for
expression result value sets is reliable enough. Initial support is limited to
schema-backed real columns.

Use `mode: "unsafeNumber"` for decimal and bigint number overrides. This was
rejected as redundant because the default remains conservative `string`; explicitly
mapping the type to `number` is the opt-in. The docs and ADR must still call out
precision risk and runtime responsibility.

Allow arbitrary TypeScript type expressions in config. This was rejected to keep
configuration validation and generated output predictable. Users can define complex
types in their project and reference the named type.

Generate aliases for enum unions or import conflicts. This was rejected because
sqlay avoids automatic naming transformations. Enum unions are emitted inline, and
import name conflicts are errors.
