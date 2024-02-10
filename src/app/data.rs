use anyhow::Context;

#[derive(serde::Deserialize, serde::Serialize, Default, Debug)]
pub struct Data {
    rows: Vec<LogRow>,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Debug)]
pub struct LogRow {
    time: Option<String>,
    request_id: Option<String>,
    otel_name: Option<String>,
    msg: Option<String>,
    // TODO 2: Capture other info
}

impl LogRow {
    const TEXT_FOR_EMPTY: &'static str = "[ --- ]";

    pub(crate) fn time(&self) -> &str {
        if let Some(val) = &self.time {
            val
        } else {
            Self::TEXT_FOR_EMPTY
        }
    }

    pub(crate) fn request_id(&self) -> &str {
        if let Some(val) = &self.request_id {
            val
        } else {
            Self::TEXT_FOR_EMPTY
        }
    }

    pub(crate) fn otel_name(&self) -> &str {
        if let Some(val) = &self.otel_name {
            val
        } else {
            Self::TEXT_FOR_EMPTY
        }
    }

    pub(crate) fn msg(&self) -> &str {
        if let Some(val) = &self.msg {
            val
        } else {
            Self::TEXT_FOR_EMPTY
        }
    }
}

impl Data {
    pub fn rows(&self) -> &[LogRow] {
        &self.rows
    }
}

impl TryFrom<&str> for Data {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut result = Data::default();
        for (i, line) in value.lines().enumerate() {
            result.rows.push(
                serde_json::from_str(line)
                    .with_context(|| format!("failed to parse line {}", i + 1))?,
            );
        }
        Ok(result)
    }
}
