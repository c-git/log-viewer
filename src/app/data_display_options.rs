use std::collections::{BTreeMap, BTreeSet};

#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq, Eq)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct DataDisplayOptions {
    main_list_fields: Vec<String>,

    /// Lists fields to show last as they are not unique to a request
    common_fields: BTreeSet<String>,

    /// The field to use to highlight other related log entries
    ///
    /// WARNING: This must be a valid index into the list as this is assumed in method implementations
    emphasize_if_matching_field_idx: Option<usize>,

    /// When set adds a field with this name and populates it with the row numbers (Skips record if field name already exists)
    pub row_idx_field_name: Option<String>,

    /// Controls how errors during file loading are treated
    pub row_parse_error_handling: RowParseErrorHandling,

    /// Used for optionally converting message levels to strings
    pub level_conversion: Option<LevelConversion>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq, Eq)]
pub enum RowParseErrorHandling {
    AbortOnAnyErrors,
    ConvertFailedLines {
        raw_line_field_name: String,
        /// If set the error message from the failure is placed in this field
        parse_error_field_name: Option<String>,
    },
}

#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq, Eq)]
pub struct LevelConversion {
    /// Skips record if field name already exists
    pub display_field_name: String,
    /// Skips conversion if source field cannot be found
    pub source_field_name: String,
    pub convert_map: BTreeMap<i64, String>,
}

impl DataDisplayOptions {
    pub fn main_list_fields(&self) -> &[String] {
        &self.main_list_fields
    }
    pub fn emphasize_if_matching_field_idx(&self) -> &Option<usize> {
        &self.emphasize_if_matching_field_idx
    }
    pub fn common_fields(&self) -> &BTreeSet<String> {
        &self.common_fields
    }
}

impl Default for DataDisplayOptions {
    fn default() -> Self {
        Self {
            // TODO 3: Add ability to show, select and reorder selected fields
            main_list_fields: [
                "row#",
                "level_str",
                "time",
                "request_id",
                "otel.name",
                "msg",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            common_fields: [
                "elapsed_milliseconds",
                "file",
                "hostname",
                "http.flavor",
                "http.host",
                "http.method",
                "http.route",
                "http.scheme",
                "http.target",
                "http.user_agent",
                "level",
                "line",
                "name",
                "otel.kind",
                "pid",
                "req",
                "request_id",
                "res",
                "target",
                "time",
                "v",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            emphasize_if_matching_field_idx: Some(3),
            row_idx_field_name: Some("row#".to_string()),
            row_parse_error_handling: Default::default(),
            level_conversion: Some(Default::default()),
        }
    }
}

impl Default for RowParseErrorHandling {
    fn default() -> Self {
        Self::ConvertFailedLines {
            raw_line_field_name: "msg".into(),
            parse_error_field_name: Some("parse_err".into()),
        }
    }
}

impl Default for LevelConversion {
    fn default() -> Self {
        // See bunyan levels https://github.com/trentm/node-bunyan?tab=readme-ov-file#levels and note rust only goes up to Error
        let convert_map = vec![
            (60, "Fatal".to_string()),
            (50, "Error".to_string()),
            (40, "Warn".to_string()),
            (30, "Info".to_string()),
            (20, "Debug".to_string()),
            (10, "Trace".to_string()),
        ]
        .into_iter()
        .collect();
        Self {
            display_field_name: "level_str".into(),
            source_field_name: "level".into(),
            convert_map,
        }
    }
}
