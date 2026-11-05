use serde::Deserialize;

/// A GraphQL answer: the data, or (with status 200 all the same) errors.
#[derive(Deserialize)]
pub(crate) struct GraphQlResponse<T> {
    data: Option<T>,
    #[serde(default)]
    errors: Vec<GraphQlError>,
}

#[derive(Deserialize)]
struct GraphQlError {
    message: String,
}

impl<T> GraphQlResponse<T> {
    /// The data, or the error messages joined when there is none (or when
    /// GraphQL reported errors next to partial data, which callers do not use).
    pub fn into_data(self) -> Result<T, String> {
        let messages: Vec<String> = self.errors.into_iter().map(|error| error.message).collect();
        if let (Some(data), true) = (self.data, messages.is_empty()) {
            return Ok(data);
        }
        if messages.is_empty() {
            Err("GitHub sent no data".to_owned())
        } else {
            Err(messages.join("; "))
        }
    }
}
