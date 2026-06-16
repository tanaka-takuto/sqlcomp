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
  type paramDirectColumnInference_Input,
  type paramDirectColumnInference_Output,
  paramDirectColumnInference,
  type paramInListMarkers_Input,
  type paramInListMarkers_Output,
  paramInListMarkers,
  type paramNullableInput_Input,
  type paramNullableInput_Output,
  paramNullableInput,
  type paramRepeatedId_Input,
  type paramRepeatedId_Output,
  paramRepeatedId,
  type paramValueTypeOverride_Input,
  type paramValueTypeOverride_Output,
  paramValueTypeOverride,
} from "./generated/valid/param_bindings";
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

const directParamInput: paramDirectColumnInference_Input = {
  parentBigintNnCol: "1",
};
const directParamQuery = paramDirectColumnInference(directParamInput);
const directParamParams: readonly [string] = directParamQuery.params;
const directParamOutput: paramDirectColumnInference_Output = [];

const valueTypeParamInput: paramValueTypeOverride_Input = {
  lowerVarchar: "varchar-320-a",
};
const valueTypeParamQuery = paramValueTypeOverride(valueTypeParamInput);
const valueTypeParamParams: readonly [string] = valueTypeParamQuery.params;
const valueTypeParamOutput: paramValueTypeOverride_Output = [];

const nullableParamInput: paramNullableInput_Input = {
  optionalDatetime: null,
};
const nullableParamQuery = paramNullableInput(nullableParamInput);
const nullableParamParams: readonly [string | null] = nullableParamQuery.params;
const nullableParamOutput: paramNullableInput_Output = [];

const repeatedParamInput: paramRepeatedId_Input = {
  searchText: "varchar-320-a",
};
const repeatedParamQuery = paramRepeatedId(repeatedParamInput);
const repeatedParamParams: readonly [string, string] = repeatedParamQuery.params;
const repeatedParamOutput: paramRepeatedId_Output = [];

const inListParamInput: paramInListMarkers_Input = {
  firstState: "state_a",
  secondState: "state_b",
};
const inListParamQuery = paramInListMarkers(inListParamInput);
const inListParamParams: readonly [string, string] = inListParamQuery.params;
const inListParamOutput: paramInListMarkers_Output = [];

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
void directParamParams;
void directParamOutput;
void valueTypeParamParams;
void valueTypeParamOutput;
void nullableParamParams;
void nullableParamOutput;
void repeatedParamParams;
void repeatedParamOutput;
void inListParamParams;
void inListParamOutput;
