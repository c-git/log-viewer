use std::collections::BTreeSet;

#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq, Eq)]
pub struct DataDisplayOptions {
    main_list_fields: Vec<String>,

    /// Lists fields to show last as they are not unique to a request
    common_fields: BTreeSet<String>,

    /// The field to use to highlight other related log entries
    ///
    /// WARNING: This must be a valid index into the list as this is assumed in method implementations
    emphasize_if_matching_field_idx: Option<usize>,
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
            main_list_fields: ["time", "request_id", "otel.name", "msg"]
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
                "request_id",
                "target",
                "time",
                "v",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            emphasize_if_matching_field_idx: Some(1),
        }
    }
}
