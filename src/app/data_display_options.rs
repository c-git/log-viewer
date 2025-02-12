use std::collections::{BTreeMap, BTreeSet};

use egui::Color32;

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

    /// Fields that should be colored based on their value. Key is field name
    pub colored_fields: BTreeMap<String, FieldColoringRules>,

    /// When set adds a field with this name and populates it with the row numbers (Skips record if field name already exists)
    pub row_idx_field_name: Option<String>,

    /// Controls how errors during file loading are treated
    pub row_parse_error_handling: RowParseErrorHandling,

    /// Used for optionally converting message levels to strings
    pub level_conversion: Option<LevelConversion>,

    /// Used for optionally including the size of messages
    pub row_size_config: Option<RowSizeConfig>,
}

#[derive(Default, serde::Deserialize, serde::Serialize, Debug, PartialEq, Eq)]
#[serde(default)]
pub struct FieldColoringRules {
    /// Matches a field value to color
    pub value_color_map: BTreeMap<String, Color32>,
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
#[serde(default)]
pub struct LevelConversion {
    /// Skips record if field name already exists
    pub display_field_name: String,
    /// Skips conversion if source field cannot be found
    pub source_field_name: String,
    pub convert_map: BTreeMap<i64, String>,
}

#[derive(Default, serde::Deserialize, serde::Serialize, Debug, PartialEq, Eq)]
#[serde(default)]
pub struct RowSizeConfig {
    pub field_name: String,
    pub units: SizeUnits,
}

#[derive(Default, serde::Deserialize, serde::Serialize, Debug, PartialEq, Eq)]
pub enum SizeUnits {
    Bytes,
    KB,
    MB,
    GB,
    TB,
    #[default]
    /// Is output as a String because it includes the unit and is the largest unit where the output value is >= 1
    Auto,
}
impl SizeUnits {
    pub(crate) fn convert(&self, row_size_in_bytes: usize) -> serde_json::Value {
        let scalar = match self {
            SizeUnits::Bytes => 1.0,
            SizeUnits::KB => 1024.0,
            SizeUnits::MB => 1024.0 * 1024.0,
            SizeUnits::GB => 1024.0 * 1024.0 * 1024.0,
            SizeUnits::TB => 1024.0 * 1024.0 * 1024.0 * 1024.0,
            SizeUnits::Auto => {
                // TODO 5: Rewrite with shifts for performance
                // Special handling see doc string for explanation
                let units = ["Bytes", "KB", "MB", "GB", "TB"];
                let mut last_index = 0;
                let mut scalar = 1.0f64;
                let row_size_in_bytes = row_size_in_bytes as f64;
                for i in 1..units.len() {
                    let new_scalar = scalar * 1024.0;
                    if (row_size_in_bytes / new_scalar) >= 1.0 {
                        last_index = i;
                        scalar = new_scalar;
                    } else {
                        // Last was as correct unit
                        break;
                    }
                }
                let result = row_size_in_bytes / scalar;
                return format!("{result:0>9.4} {}", units[last_index]).into();
            }
        };
        let result = row_size_in_bytes as f64 / scalar;
        result.into()
    }
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
                "row_size",
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
            emphasize_if_matching_field_idx: Some(4),
            row_idx_field_name: Some("row#".to_string()),
            row_size_config: Some(RowSizeConfig {
                field_name: "row_size".to_string(),
                units: SizeUnits::Auto,
            }),
            row_parse_error_handling: Default::default(),
            level_conversion: Some(Default::default()),
            colored_fields: [(
                "level_str".to_string(),
                FieldColoringRules {
                    value_color_map: [
                        ("Trace".to_string(), Color32::from_rgb(150, 100, 200)),
                        ("Debug".to_string(), Color32::from_rgb(80, 140, 205)),
                        ("Info".to_string(), Color32::from_rgb(15, 175, 85)),
                        ("Warn".to_string(), Color32::from_rgb(210, 210, 20)),
                        ("Error".to_string(), Color32::from_rgb(220, 105, 105)),
                        ("Fatal".to_string(), Color32::from_rgb(255, 20, 20)),
                    ]
                    .into_iter()
                    .collect(),
                },
            )]
            .into_iter()
            .collect(),
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
