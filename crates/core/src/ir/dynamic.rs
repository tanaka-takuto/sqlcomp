use super::ParamBinding;

/// Runtime-composable SQL body for a query or mutation with Slot and Repeat occurrences.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledDynamicQuery {
    base_bodies: Vec<CompiledSqlBody>,
    base_segments: Vec<CompiledSqlSegment>,
    slot_occurrences: Vec<CompiledSlotOccurrence>,
    slots: Vec<CompiledSlotDefinition>,
    repeats: Vec<CompiledRepeatDefinition>,
}

impl CompiledDynamicQuery {
    /// Build a dynamic body with Slot occurrences only.
    ///
    /// `base_segments` contains the SQL text around Slot occurrences, so callers
    /// should provide exactly one more base segment than Slot occurrence.
    #[must_use]
    pub fn new(
        base_segments: Vec<CompiledSqlSegment>,
        slot_occurrences: Vec<CompiledSlotOccurrence>,
        slots: Vec<CompiledSlotDefinition>,
    ) -> Self {
        let base_bodies = base_segments
            .iter()
            .cloned()
            .map(CompiledSqlBody::from_segment)
            .collect();
        Self {
            base_bodies,
            base_segments,
            slot_occurrences,
            slots,
            repeats: Vec::new(),
        }
    }

    /// Build a dynamic body with Slot and Repeat occurrences.
    ///
    /// `base_bodies` contains the SQL bodies around Slot occurrences, so callers
    /// should provide exactly one more base body than Slot occurrence.
    #[must_use]
    pub fn new_with_bodies(
        base_bodies: Vec<CompiledSqlBody>,
        slot_occurrences: Vec<CompiledSlotOccurrence>,
        slots: Vec<CompiledSlotDefinition>,
        repeats: Vec<CompiledRepeatDefinition>,
    ) -> Self {
        let base_segments = base_bodies
            .iter()
            .map(CompiledSqlBody::legacy_segment)
            .collect();

        Self {
            base_bodies,
            base_segments,
            slot_occurrences,
            slots,
            repeats,
        }
    }

    /// Base SQL bodies around Slot occurrences.
    #[must_use]
    pub fn base_bodies(&self) -> &[CompiledSqlBody] {
        &self.base_bodies
    }

    /// Legacy base SQL segments around Slot occurrences.
    #[must_use]
    pub fn base_segments(&self) -> &[CompiledSqlSegment] {
        &self.base_segments
    }

    /// Slot occurrences in query SQL order.
    #[must_use]
    pub fn slot_occurrences(&self) -> &[CompiledSlotOccurrence] {
        &self.slot_occurrences
    }

    /// Unique Slot definitions in query first-seen order.
    #[must_use]
    pub fn slots(&self) -> &[CompiledSlotDefinition] {
        &self.slots
    }

    /// Unique top-level Repeat definitions in first-seen order.
    #[must_use]
    pub fn repeats(&self) -> &[CompiledRepeatDefinition] {
        &self.repeats
    }
}

/// One SQL body and the Repeat occurrences it contains.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledSqlBody {
    base_segments: Vec<CompiledSqlSegment>,
    repeat_occurrences: Vec<CompiledRepeatOccurrence>,
}

impl CompiledSqlBody {
    /// Build a compiled SQL body.
    ///
    /// `base_segments` contains SQL text around Repeat occurrences, so callers should
    /// provide exactly one more base segment than Repeat occurrence.
    #[must_use]
    pub const fn new(
        base_segments: Vec<CompiledSqlSegment>,
        repeat_occurrences: Vec<CompiledRepeatOccurrence>,
    ) -> Self {
        Self {
            base_segments,
            repeat_occurrences,
        }
    }

    /// Build a static SQL body with no Repeat occurrences.
    #[must_use]
    pub fn from_segment(segment: CompiledSqlSegment) -> Self {
        Self {
            base_segments: vec![segment],
            repeat_occurrences: Vec::new(),
        }
    }

    /// Base SQL segments around Repeat occurrences.
    #[must_use]
    pub fn base_segments(&self) -> &[CompiledSqlSegment] {
        &self.base_segments
    }

    /// Repeat occurrences in SQL emission order.
    #[must_use]
    pub fn repeat_occurrences(&self) -> &[CompiledRepeatOccurrence] {
        &self.repeat_occurrences
    }

    fn legacy_segment(&self) -> CompiledSqlSegment {
        let mut sql = String::new();
        let mut params = Vec::new();

        for (index, segment) in self.base_segments.iter().enumerate() {
            sql.push_str(segment.sql());
            params.extend(segment.params().iter().cloned());

            if let Some(repeat) = self.repeat_occurrences.get(index) {
                let item_segment = repeat.item_segment();
                sql.push_str(item_segment.sql());
                params.extend(item_segment.params().iter().cloned());
            }
        }

        CompiledSqlSegment::new(sql, params)
    }
}

/// Unique Repeat input definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledRepeatDefinition {
    id: String,
    fields: Vec<ParamBinding>,
}

impl CompiledRepeatDefinition {
    /// Build a compiled Repeat definition.
    #[must_use]
    pub const fn new(id: String, fields: Vec<ParamBinding>) -> Self {
        Self { id, fields }
    }

    /// Repeat ID exactly as written in source metadata.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Repeat item fields in generated input order.
    #[must_use]
    pub fn fields(&self) -> &[ParamBinding] {
        &self.fields
    }
}

/// One Repeat occurrence in SQL emission order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledRepeatOccurrence {
    repeat_id: String,
    separator: String,
    item_segment: CompiledSqlSegment,
}

impl CompiledRepeatOccurrence {
    /// Build a compiled Repeat occurrence.
    #[must_use]
    pub const fn new(
        repeat_id: String,
        separator: String,
        item_segment: CompiledSqlSegment,
    ) -> Self {
        Self {
            repeat_id,
            separator,
            item_segment,
        }
    }

    /// Repeat ID for this occurrence.
    #[must_use]
    pub fn repeat_id(&self) -> &str {
        &self.repeat_id
    }

    /// Raw SQL separator inserted between runtime items.
    #[must_use]
    pub fn separator(&self) -> &str {
        &self.separator
    }

    /// SQL item template and Param bindings in placeholder order.
    #[must_use]
    pub const fn item_segment(&self) -> &CompiledSqlSegment {
        &self.item_segment
    }
}

/// One SQL segment and the Param bindings it contains.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledSqlSegment {
    sql: String,
    params: Vec<ParamBinding>,
}

impl CompiledSqlSegment {
    /// Build a compiled SQL segment.
    #[must_use]
    pub const fn new(sql: String, params: Vec<ParamBinding>) -> Self {
        Self { sql, params }
    }

    /// SQL text for this segment.
    #[must_use]
    pub fn sql(&self) -> &str {
        &self.sql
    }

    /// Param bindings in this segment in SQL placeholder order.
    #[must_use]
    pub fn params(&self) -> &[ParamBinding] {
        &self.params
    }
}

/// One occurrence of a query-local Slot in SQL order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledSlotOccurrence {
    slot_id: String,
}

impl CompiledSlotOccurrence {
    /// Build a compiled Slot occurrence.
    #[must_use]
    pub const fn new(slot_id: String) -> Self {
        Self { slot_id }
    }

    /// Query-local Slot ID for this occurrence.
    #[must_use]
    pub fn slot_id(&self) -> &str {
        &self.slot_id
    }
}

/// Unique Slot definition and its ordered target branches.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledSlotDefinition {
    id: String,
    branches: Vec<CompiledSlotBranch>,
}

impl CompiledSlotDefinition {
    /// Build a compiled Slot definition.
    #[must_use]
    pub const fn new(id: String, branches: Vec<CompiledSlotBranch>) -> Self {
        Self { id, branches }
    }

    /// Query-local Slot ID.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Target branches in source `targets` order.
    #[must_use]
    pub fn branches(&self) -> &[CompiledSlotBranch] {
        &self.branches
    }
}

/// One selected Fragment branch for a Slot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledSlotBranch {
    target_id: String,
    body: CompiledSqlBody,
    segments: Vec<CompiledSqlSegment>,
    repeats: Vec<CompiledRepeatDefinition>,
}

impl CompiledSlotBranch {
    /// Build a compiled Slot branch with static SQL segments.
    #[must_use]
    pub fn new(target_id: String, segments: Vec<CompiledSqlSegment>) -> Self {
        let sql = segments.iter().map(CompiledSqlSegment::sql).collect();
        let params = segments
            .iter()
            .flat_map(|segment| segment.params().iter().cloned())
            .collect();
        let body = CompiledSqlBody::from_segment(CompiledSqlSegment::new(sql, params));
        Self {
            target_id,
            body,
            segments,
            repeats: Vec::new(),
        }
    }

    /// Build a compiled Slot branch with a dynamic Repeat-aware body.
    #[must_use]
    pub fn new_with_body(
        target_id: String,
        body: CompiledSqlBody,
        repeats: Vec<CompiledRepeatDefinition>,
    ) -> Self {
        let segments = vec![body.legacy_segment()];
        Self {
            target_id,
            body,
            segments,
            repeats,
        }
    }

    /// Fragment ID selected by this branch.
    #[must_use]
    pub fn target_id(&self) -> &str {
        &self.target_id
    }

    /// Fragment SQL segments for this branch.
    #[must_use]
    pub fn segments(&self) -> &[CompiledSqlSegment] {
        &self.segments
    }

    /// Fragment SQL body for this branch.
    #[must_use]
    pub const fn body(&self) -> &CompiledSqlBody {
        &self.body
    }

    /// Unique Repeat definitions in this selected branch.
    #[must_use]
    pub fn repeats(&self) -> &[CompiledRepeatDefinition] {
        &self.repeats
    }
}
