import {
  type generationEscapedSql_Output,
  type generationExplicitManyOverridesLimitOne_Output,
  type generationExplicitOneOverridesMany_Output,
  type generationInferredSingleRow_Output,
  generationEscapedSql,
  generationExplicitManyOverridesLimitOne,
  generationExplicitOneOverridesMany,
  generationInferredSingleRow,
} from "./generated/valid/generation_surface";
import {
  type nestedPathMapping_Output,
  nestedPathMapping,
} from "./generated/valid/nested/path_mapping";
import {
  type typeMetadataAggregateExpressions_Output,
  type typeMetadataDirectColumns_Output,
  type typeMetadataExpressions_Output,
  type typeMetadataJoinColumns_Output,
  type typeMetadataLeftJoinColumns_Output,
  type typeMetadataOddColumnNames_Output,
  type typeMetadataSingleRow_Output,
  typeMetadataAggregateExpressions,
  typeMetadataDirectColumns,
  typeMetadataExpressions,
  typeMetadataJoinColumns,
  typeMetadataLeftJoinColumns,
  typeMetadataOddColumnNames,
  typeMetadataSingleRow,
} from "./generated/valid/type_metadata_matrix";

const directColumnsQuery = typeMetadataDirectColumns();
const directColumnsOutput: typeMetadataDirectColumns_Output = [];

const joinColumnsQuery = typeMetadataJoinColumns();
const joinColumnsOutput: typeMetadataJoinColumns_Output = [];

const leftJoinColumnsQuery = typeMetadataLeftJoinColumns();
const leftJoinColumnsOutput: typeMetadataLeftJoinColumns_Output = [];

const expressionsQuery = typeMetadataExpressions();
const expressionsOutput: typeMetadataExpressions_Output = [];

const aggregateExpressionsQuery = typeMetadataAggregateExpressions();
const aggregateExpressionsOutput: typeMetadataAggregateExpressions_Output = [];

const oddColumnNamesQuery = typeMetadataOddColumnNames();
const oddColumnNamesOutput: typeMetadataOddColumnNames_Output = [];

const singleRowQuery = typeMetadataSingleRow();
const singleRowOutput: typeMetadataSingleRow_Output = null;

const escapedSqlQuery = generationEscapedSql();
const escapedSqlOutput: generationEscapedSql_Output = [];

const inferredSingleRowQuery = generationInferredSingleRow();
const inferredSingleRowOutput: generationInferredSingleRow_Output = null;

const explicitOneOverridesManyQuery = generationExplicitOneOverridesMany();
const explicitOneOverridesManyOutput: generationExplicitOneOverridesMany_Output = null;

const explicitManyOverridesLimitOneQuery = generationExplicitManyOverridesLimitOne();
const explicitManyOverridesLimitOneOutput: generationExplicitManyOverridesLimitOne_Output = [];

const nestedPathMappingQuery = nestedPathMapping();
const nestedPathMappingOutput: nestedPathMapping_Output = [];

void directColumnsQuery;
void directColumnsOutput;
void joinColumnsQuery;
void joinColumnsOutput;
void leftJoinColumnsQuery;
void leftJoinColumnsOutput;
void expressionsQuery;
void expressionsOutput;
void aggregateExpressionsQuery;
void aggregateExpressionsOutput;
void oddColumnNamesQuery;
void oddColumnNamesOutput;
void singleRowQuery;
void singleRowOutput;
void escapedSqlQuery;
void escapedSqlOutput;
void inferredSingleRowQuery;
void inferredSingleRowOutput;
void explicitOneOverridesManyQuery;
void explicitOneOverridesManyOutput;
void explicitManyOverridesLimitOneQuery;
void explicitManyOverridesLimitOneOutput;
void nestedPathMappingQuery;
void nestedPathMappingOutput;
