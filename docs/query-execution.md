# Query Execution with mysql2

SELECT query builders return SQL text, params, and generated TypeScript row/output
types. They do not execute queries, manage connections, load `.env` files, or
depend on a database driver. Application code chooses a driver and executes the
builder output.

The example below uses the generated `findBookDetail` builder from
[`examples/bookstore/sql/books.sql`](../examples/bookstore/sql/books.sql) with
`mysql2/promise`.

## Minimal SELECT

Read the same environment variable configured by `database.urlEnv`, pass the
generated SQL and params to `mysql2`, and keep row typing tied to the generated
builder types:

```ts
import mysql, {
  type Pool,
  type RowDataPacket,
} from "mysql2/promise";
import {
  findBookDetail,
  type findBookDetail_Input,
  type findBookDetail_Output,
  type findBookDetail_Row,
} from "../examples/bookstore/generated/sql/books";

type FindBookDetailMysqlRow = findBookDetail_Row & RowDataPacket;

function readDatabaseUrl(): string {
  const databaseUrl = process.env.DATABASE_URL;
  if (!databaseUrl) {
    throw new Error("DATABASE_URL is required");
  }
  return databaseUrl;
}

async function loadBookDetail(
  pool: Pool,
  input: findBookDetail_Input,
): Promise<findBookDetail_Output> {
  const statement = findBookDetail(input);
  const [rows] = await pool.execute<FindBookDetailMysqlRow[]>(
    statement.sql,
    [...statement.params],
  );
  return rows[0] ?? null;
}

async function main(): Promise<void> {
  const pool = mysql.createPool(readDatabaseUrl());

  try {
    const book = await loadBookDetail(pool, { isbn: "9780441478125" });
    console.log(book?.title ?? "not found");
  } finally {
    await pool.end();
  }
}
```

`mysql2` accepts mutable parameter arrays in its TypeScript surface. Generated
sqlay builders return readonly params, so spread them into a mutable array at the
driver boundary.

The `FindBookDetailMysqlRow` alias combines the generated row type with
`RowDataPacket`, which lets `mysql2` type the returned rows without replacing the
generated sqlay types with `any` or a hand-written duplicate row shape. For a
many-row builder, use the same pattern with the generated `<builder>_Row` and
return the rows as the generated `<builder>_Output` type.

Do not print database URLs, params that may contain secrets, or full connection
diagnostics in normal logs. The example above reports only the missing environment
variable name when configuration is absent.
