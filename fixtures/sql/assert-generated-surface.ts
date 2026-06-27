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
  type mutationDeleteAliasLimited_Input,
  mutationDeleteAliasLimited,
  type mutationInsertSet_Input,
  mutationInsertSet,
  type mutationInsertValues_Input,
  mutationInsertValues,
  type mutationReplaceSet_Input,
  mutationReplaceSet,
  type mutationReplaceValues_Input,
  mutationReplaceValues,
  type mutationSlotAssignment_Input,
  mutationSlotAssignment,
  type mutationUpdateAliasLimited_Input,
  mutationUpdateAliasLimited,
  type mutationUpsertValues_Input,
  mutationUpsertValues,
  type mutationValueTypeOverride_Input,
  mutationValueTypeOverride,
} from "./generated/valid/mutation_builders";
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
  type slotRuntimeOptionalFilter_Input,
  type slotRuntimeOptionalFilter_Output,
  type slotRuntimeSearch_Input,
  type slotRuntimeSearch_Output,
  slotRuntimeOptionalFilter,
  slotRuntimeSearch,
} from "./generated/valid/slot_runtime";
import {
  type slotFragmentContextualParam_Input,
  type slotFragmentContextualParam_Output,
  type slotFragmentSearch_Input,
  type slotFragmentSearch_Output,
  slotFragmentContextualParam,
  slotFragmentSearch,
} from "./generated/valid/slot_fragment_composition";
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

const mutationInsertValuesInput: mutationInsertValues_Input = {
  bigintId: "10",
  intValue: 7,
  textValue: "varchar-320-new",
  activeValue: 1,
  createdAt: "2026-06-21 10:11:12.123456",
};
const mutationInsertValuesQuery = mutationInsertValues(mutationInsertValuesInput);
const mutationInsertValuesParams: readonly [
  string,
  number,
  string,
  number,
  string,
] = mutationInsertValuesQuery.params;

const mutationInsertSetInput: mutationInsertSet_Input = {
  childId: "200",
  parentId: "1",
  childLabel: "child-new",
  childAmount: "25.50",
};
const mutationInsertSetQuery = mutationInsertSet(mutationInsertSetInput);
const mutationInsertSetParams: readonly [string, string, string, string] =
  mutationInsertSetQuery.params;

const mutationUpsertValuesInput: mutationUpsertValues_Input = {
  childId: "201",
  parentId: "1",
  childLabel: "child-upsert",
  childAmount: "30.75",
  updatedLabel: "child-updated",
  updatedAmount: "31.25",
};
const mutationUpsertValuesQuery = mutationUpsertValues(mutationUpsertValuesInput);
const mutationUpsertValuesParams: readonly [
  string,
  string,
  string,
  string,
  string,
  string,
] = mutationUpsertValuesQuery.params;

const mutationReplaceValuesInput: mutationReplaceValues_Input = {
  childId: "202",
  parentId: "2",
  childLabel: "child-replace",
  childAmount: "40.00",
};
const mutationReplaceValuesQuery = mutationReplaceValues(mutationReplaceValuesInput);
const mutationReplaceValuesParams: readonly [string, string, string, string] =
  mutationReplaceValuesQuery.params;

const mutationReplaceSetInput: mutationReplaceSet_Input = {
  childId: "203",
  parentId: "2",
  childLabel: "child-replace-set",
  childAmount: "41.00",
};
const mutationReplaceSetQuery = mutationReplaceSet(mutationReplaceSetInput);
const mutationReplaceSetParams: readonly [string, string, string, string] =
  mutationReplaceSetQuery.params;

const mutationUpdateAliasLimitedInput: mutationUpdateAliasLimited_Input = {
  textValue: "varchar-320-updated",
  bigintId: "1",
};
const mutationUpdateAliasLimitedQuery = mutationUpdateAliasLimited(
  mutationUpdateAliasLimitedInput,
);
const mutationUpdateAliasLimitedParams: readonly [string, string] =
  mutationUpdateAliasLimitedQuery.params;

const mutationDeleteAliasLimitedInput: mutationDeleteAliasLimited_Input = {
  parentId: "2",
};
const mutationDeleteAliasLimitedQuery = mutationDeleteAliasLimited(
  mutationDeleteAliasLimitedInput,
);
const mutationDeleteAliasLimitedParams: readonly [string] =
  mutationDeleteAliasLimitedQuery.params;

const mutationValueTypeOverrideInput: mutationValueTypeOverride_Input = {
  adjustment: 1.25,
  childId: "100",
};
const mutationValueTypeOverrideQuery = mutationValueTypeOverride(
  mutationValueTypeOverrideInput,
);
const mutationValueTypeOverrideParams: readonly [number, string] =
  mutationValueTypeOverrideQuery.params;

type MutationSlotNullableTextAssignment = Extract<
  NonNullable<mutationSlotAssignment_Input["assignment"]>,
  { $fragment: "mutationAssignNullableText" }
>;
type MutationSlotDecimalAssignment = Extract<
  NonNullable<mutationSlotAssignment_Input["assignment"]>,
  { $fragment: "mutationAssignDecimal" }
>;

const mutationSlotAssignmentNullableTextAssignment: MutationSlotNullableTextAssignment = {
  $fragment: "mutationAssignNullableText",
  textValue: null,
};
const mutationSlotAssignmentNullableTextInput: mutationSlotAssignment_Input = {
  textValue: "slot-base",
  bigintId: "1",
  assignment: mutationSlotAssignmentNullableTextAssignment,
};
const mutationSlotAssignmentNullableTextQuery = mutationSlotAssignment(
  mutationSlotAssignmentNullableTextInput,
);
const mutationSlotAssignmentNullableTextExpectedParams: readonly [
  string,
  string | null,
  string,
] = [
  mutationSlotAssignmentNullableTextInput.textValue,
  mutationSlotAssignmentNullableTextAssignment.textValue,
  mutationSlotAssignmentNullableTextInput.bigintId,
];
const mutationSlotAssignmentNullableTextParams: readonly unknown[] =
  mutationSlotAssignmentNullableTextQuery.params;

const mutationSlotAssignmentDecimalAssignment: MutationSlotDecimalAssignment = {
  $fragment: "mutationAssignDecimal",
  amount: "55.5000",
};
const mutationSlotAssignmentDecimalInput: mutationSlotAssignment_Input = {
  textValue: "slot-base",
  bigintId: "1",
  assignment: mutationSlotAssignmentDecimalAssignment,
};
const mutationSlotAssignmentDecimalQuery = mutationSlotAssignment(
  mutationSlotAssignmentDecimalInput,
);
const mutationSlotAssignmentDecimalExpectedParams: readonly [
  string,
  string,
  string,
] = [
  mutationSlotAssignmentDecimalInput.textValue,
  mutationSlotAssignmentDecimalAssignment.amount,
  mutationSlotAssignmentDecimalInput.bigintId,
];
const mutationSlotAssignmentDecimalParams: readonly unknown[] =
  mutationSlotAssignmentDecimalQuery.params;

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

const slotRuntimeSearchInput: slotRuntimeSearch_Input = {
  minId: "1",
  state: "state_a",
  filter: {
    $fragment: "slotRuntimeByVarchar",
    varcharFilter: "varchar-320-a",
  },
};
const slotRuntimeSearchQuery = slotRuntimeSearch(slotRuntimeSearchInput);
const slotRuntimeSearchParams: readonly unknown[] = slotRuntimeSearchQuery.params;
const slotRuntimeSearchOutput: slotRuntimeSearch_Output = [];

const slotRuntimeOptionalFilterQuery = slotRuntimeOptionalFilter();
const slotRuntimeOptionalFilterInput: slotRuntimeOptionalFilter_Input = {
  filter: {
    $fragment: "slotRuntimeByChildAmount",
    minAmount: "10.00",
  },
};
const slotRuntimeOptionalFilterSelectedQuery = slotRuntimeOptionalFilter(
  slotRuntimeOptionalFilterInput,
);
const slotRuntimeOptionalFilterParams: readonly unknown[] =
  slotRuntimeOptionalFilterSelectedQuery.params;
const slotRuntimeOptionalFilterOutput: slotRuntimeOptionalFilter_Output = [];

const slotFragmentSearchInput: slotFragmentSearch_Input = {
  minId: "1",
  filter: {
    $fragment: "slotFixtureByAmount",
    minAmount: "10.00",
  },
  repeatFilter: {
    $fragment: "slotFixtureActiveOnly",
  },
};
const slotFragmentSearchQuery = slotFragmentSearch(slotFragmentSearchInput);
const slotFragmentSearchParams: readonly unknown[] = slotFragmentSearchQuery.params;
const slotFragmentSearchOutput: slotFragmentSearch_Output = [];

const slotFragmentSearchTextBranch: slotFragmentSearch_Input = {
  minId: "1",
  filter: {
    $fragment: "slotFixtureByText",
    textFilter: "varchar-320-a",
  },
};
const slotFragmentSearchNullableBranch: slotFragmentSearch_Input = {
  minId: "1",
  filter: {
    $fragment: "slotFixtureNullableCreated",
    createdAfter: null,
  },
};
const slotFragmentSearchLocalBranch: slotFragmentSearch_Input = {
  minId: "1",
  filter: {
    $fragment: "slotFixtureByState",
    state: "state_a",
  },
};

const slotFragmentContextualParamInput: slotFragmentContextualParam_Input = {
  textComparator: {
    $fragment: "slotFixtureEqualsValue",
    value: "varchar-320-a",
  },
  numberComparator: {
    $fragment: "slotFixtureEqualsValue",
    value: 42,
  },
};
const slotFragmentContextualParamQuery = slotFragmentContextualParam(
  slotFragmentContextualParamInput,
);
const slotFragmentContextualParamParams: readonly unknown[] =
  slotFragmentContextualParamQuery.params;
const slotFragmentContextualParamOutput: slotFragmentContextualParam_Output = [];

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
void mutationInsertValuesParams;
void mutationInsertSetParams;
void mutationUpsertValuesParams;
void mutationReplaceValuesParams;
void mutationReplaceSetParams;
void mutationUpdateAliasLimitedParams;
void mutationDeleteAliasLimitedParams;
void mutationValueTypeOverrideParams;
void mutationSlotAssignmentNullableTextExpectedParams;
void mutationSlotAssignmentNullableTextParams;
void mutationSlotAssignmentDecimalExpectedParams;
void mutationSlotAssignmentDecimalParams;
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
void slotRuntimeSearchParams;
void slotRuntimeSearchOutput;
void slotRuntimeOptionalFilterQuery;
void slotRuntimeOptionalFilterParams;
void slotRuntimeOptionalFilterOutput;
void slotFragmentSearchParams;
void slotFragmentSearchOutput;
void slotFragmentSearchTextBranch;
void slotFragmentSearchNullableBranch;
void slotFragmentSearchLocalBranch;
void slotFragmentContextualParamParams;
void slotFragmentContextualParamOutput;
