use std::fmt::Display;

#[derive(Debug, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq, Clone)]
#[serde(default)]
pub struct FilterConfig {
    pub search_key: String,
    pub filter_on: FilterOn,
    pub is_case_sensitive: bool,
    pub comparator: Comparator,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq, Clone)]
pub enum FilterOn {
    #[default]
    Any,
    Field(FieldSpecifier),
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq, Clone)]
pub struct FieldSpecifier {
    pub name: String,
}

impl FilterOn {
    /// Returns `true` if the filter on is [`Any`].
    ///
    /// [`Any`]: FilterOn::Any
    #[must_use]
    pub fn is_any(&self) -> bool {
        matches!(self, Self::Any)
    }

    /// Returns `true` if the filter on is [`Field`].
    ///
    /// [`Field`]: FilterOn::Field
    #[must_use]
    pub fn is_field(&self) -> bool {
        matches!(self, Self::Field { .. })
    }
}

#[cfg_attr(test, derive(strum::EnumIter))]
#[derive(Debug, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum Comparator {
    LessThan,
    LessThanEqual,
    Equal,
    GreaterThan,
    GreaterThanEqual,
    NotEqual,
    #[default]
    Contains,
    NotContains,
}

impl Comparator {
    pub fn apply(&self, search_key: &str, value: &str) -> bool {
        match self {
            Comparator::LessThan => value < search_key,
            Comparator::LessThanEqual => value <= search_key,
            Comparator::Equal => value == search_key,
            Comparator::GreaterThan => value > search_key,
            Comparator::GreaterThanEqual => value >= search_key,
            Comparator::NotEqual => value != search_key,
            Comparator::Contains => value.contains(search_key),
            Comparator::NotContains => !value.contains(search_key),
        }
    }
}

impl Display for Comparator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Comparator::LessThan => "Less Than",
                Comparator::LessThanEqual => "Less than equal",
                Comparator::Equal => "Equal",
                Comparator::GreaterThan => "Greater than",
                Comparator::GreaterThanEqual => "Greater than equal",
                Comparator::NotEqual => "Not equal",
                Comparator::Contains => "Contains",
                Comparator::NotContains => "Not contains",
            }
        )
    }
}

impl Display for FilterOn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FilterOn::Any => write!(f, "Any"),
            FilterOn::Field(name) => write!(f, "[Field Named: {name}]"),
        }
    }
}

impl Display for FieldSpecifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.name.fmt(f)
    }
}
