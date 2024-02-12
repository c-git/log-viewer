#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq, Eq)]
pub struct DataDisplayOptions {
    main_list_fields: Vec<String>,
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
}

impl Default for DataDisplayOptions {
    fn default() -> Self {
        Self {
            // TODO 3: Add ability to show, select and reorder selected fields
            main_list_fields: vec![
                "time".into(),
                "request_id".into(),
                "otel.name".into(),
                "msg".into(),
            ],
            emphasize_if_matching_field_idx: Some(1),
        }
    }
}
