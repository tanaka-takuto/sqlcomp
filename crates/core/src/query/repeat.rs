use crate::SourceLocation;

use super::ParamUsage;

/// One inline Repeat range in source order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepeatUsage {
    id: String,
    separator: String,
    start_index: usize,
    end_index: usize,
    item_param_usages: Vec<ParamUsage>,
    source_location: SourceLocation,
}

impl RepeatUsage {
    /// Build a Repeat usage occurrence.
    #[must_use]
    pub const fn new(
        id: String,
        separator: String,
        start_index: usize,
        end_index: usize,
        source_location: SourceLocation,
    ) -> Self {
        Self {
            id,
            separator,
            start_index,
            end_index,
            item_param_usages: Vec::new(),
            source_location,
        }
    }

    /// Attach inline Param occurrences inside the repeated item template.
    #[must_use]
    pub fn with_item_param_usages(mut self, usages: Vec<ParamUsage>) -> Self {
        self.item_param_usages = usages;
        self
    }

    /// Repeat ID exactly as written in source metadata.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Raw SQL separator text inserted between expanded items.
    #[must_use]
    pub fn separator(&self) -> &str {
        &self.separator
    }

    /// Byte index in analysis SQL where the repeated item template starts.
    #[must_use]
    pub const fn start_index(&self) -> usize {
        self.start_index
    }

    /// Byte index in analysis SQL where the repeated item template ends.
    #[must_use]
    pub const fn end_index(&self) -> usize {
        self.end_index
    }

    /// Inline Param occurrences inside this Repeat item template.
    #[must_use]
    pub fn item_param_usages(&self) -> &[ParamUsage] {
        &self.item_param_usages
    }

    /// Source location for the Repeat range.
    #[must_use]
    pub const fn source_location(&self) -> &SourceLocation {
        &self.source_location
    }

    /// Replace source location context for the Repeat range.
    #[must_use]
    pub fn with_source_location(mut self, location: SourceLocation) -> Self {
        self.source_location = location;
        self
    }
}
