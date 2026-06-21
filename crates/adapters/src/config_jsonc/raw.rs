use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawProjectConfig {
    pub(super) source: Option<RawSourceConfig>,
    pub(super) output: Option<RawOutputConfig>,
    pub(super) database: Option<RawDatabaseConfig>,
    pub(super) target: Option<RawTargetConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawSourceConfig {
    pub(super) include: Option<Vec<String>>,
    pub(super) exclude: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawOutputConfig {
    pub(super) dir: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct RawDatabaseConfig {
    pub(super) dialect: Option<String>,
    pub(super) url_env: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawTargetConfig {
    pub(super) language: Option<String>,
}
