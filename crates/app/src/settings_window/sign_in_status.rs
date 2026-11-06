use gpui::SharedString;

/// Where signing in with a pasted token stands.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SignInStatus {
    /// Nothing sent yet.
    #[default]
    Idle,
    /// Asking GitHub whose token it is.
    Checking,
    /// GitHub took the token; it is saved and the app restarts.
    SignedIn(SharedString),
    /// The token was refused or could not be saved, and why.
    Failed(SharedString),
}

impl SignInStatus {
    /// The line under the token field; `None` when there is nothing to say.
    pub fn describe(&self) -> Option<SharedString> {
        match self {
            Self::Idle => None,
            Self::Checking => Some("Checking the token with GitHub…".into()),
            Self::SignedIn(login) => Some(format!("Signed in as {login}. Restarting…").into()),
            Self::Failed(reason) => Some(reason.clone()),
        }
    }
}
