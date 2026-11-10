use serde::Deserialize;

/// The `type` GitHub gives an error on data the token may not read.
const FORBIDDEN_ERROR_TYPE: &str = "FORBIDDEN";

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
    /// GitHub's kind of error (`FORBIDDEN`, `NOT_FOUND`, …), when it says.
    #[serde(rename = "type", default)]
    kind: Option<String>,
}

/// Data that may lack the parts the token may not read.
pub(crate) enum GraphQlData<T> {
    /// Everything asked for.
    Complete(T),
    /// GitHub left out (as `null`) parts the token may not read.
    PartlyForbidden(T),
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

    /// The data, also when GitHub withheld parts of it because the token
    /// may not read them (every error `FORBIDDEN`); any other error, or no
    /// data, fails as in [`GraphQlResponse::into_data`].
    pub fn into_data_or_forbidden(self) -> Result<GraphQlData<T>, String> {
        let is_only_forbidden = !self.errors.is_empty()
            && self
                .errors
                .iter()
                .all(|error| error.kind.as_deref() == Some(FORBIDDEN_ERROR_TYPE));
        match (is_only_forbidden, self.data) {
            (true, Some(data)) => Ok(GraphQlData::PartlyForbidden(data)),
            (_, data) => Self {
                data,
                errors: self.errors,
            }
            .into_data()
            .map(GraphQlData::Complete),
        }
    }
}
