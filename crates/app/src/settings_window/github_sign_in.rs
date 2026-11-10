use gpui::{AnyElement, Context, Entity, IntoElement, Subscription, Task, div, prelude::*, px};
use qd_github::{GitHubClient, GitHubError, TokenSource, delete_stored_token, store_token};

use super::setting_row::{SETTING_FIELD_WIDTH, SettingRow};
use super::settings_window::SettingsWindow;
use super::sign_in_status::SignInStatus;
use crate::app_services::AppServices;
use crate::app_update::restart_app;
use crate::button::Button;
use crate::text_area::{TextArea, TextAreaEvent};
use crate::theme;

/// Where a classic token with the scopes the app needs is created: `repo`
/// (private repositories, reviews, merging) and `read:org` (team review
/// requests).
const NEW_TOKEN_URL: &str =
    "https://github.com/settings/tokens/new?scopes=repo,read:org&description=Quarterdeck";
/// GitHub's answer to a token it does not accept.
const UNAUTHORIZED: u16 = 401;

/// The GitHub page's sign-in: the field a token is pasted into, where
/// signing in stands, and the check or sign-out in flight.
pub struct TokenSignIn {
    /// Where a GitHub token is pasted to sign in.
    input: Entity<TextArea>,
    status: SignInStatus,
    /// The token check or sign-out in flight.
    task: Option<Task<()>>,
    /// Keeps the field's submit (cmd-enter) observed.
    _subscription: Subscription,
}

impl TokenSignIn {
    /// An empty token field; cmd-enter in it signs in, like the button.
    pub fn new(cx: &mut Context<SettingsWindow>) -> Self {
        let input = cx.new(|cx| TextArea::new("ghp_…", cx));
        let subscription = cx.subscribe(&input, |settings, _, event: &TextAreaEvent, cx| {
            if *event == TextAreaEvent::Submit {
                settings.sign_in(cx);
            }
        });
        Self {
            input,
            status: SignInStatus::default(),
            task: None,
            _subscription: subscription,
        }
    }
}

impl SettingsWindow {
    /// The account rows of the GitHub page: where the token comes from
    /// (signing out of a saved one), and a field to paste a token into,
    /// which is checked with GitHub, saved in the Keychain and used after
    /// a restart.
    pub(super) fn render_account_rows(&self, cx: &mut Context<Self>) -> AnyElement {
        let source = cx.global::<AppServices>().token_source;
        let account = source.map_or_else(
            || "Not signed in: Quarterdeck shows cached data only.".to_owned(),
            |source| format!("Signed in with {}.", source.describe()),
        );
        let is_busy = matches!(
            self.token_sign_in().status,
            SignInStatus::Checking | SignInStatus::SignedIn(_)
        );

        div()
            .flex()
            .flex_col()
            .child(SettingRow::new(
                "Account",
                account,
                div().children((source == Some(TokenSource::Keychain)).then(|| {
                    Button::new("sign-out", "Sign out")
                        .danger()
                        .disabled(is_busy)
                        .on_click(cx.listener(|settings, _, _, cx| settings.sign_out(cx)))
                })),
            ))
            .child(SettingRow::new(
                "Personal access token",
                "Paste a token with the repo and read:org scopes. It is saved in your Keychain and used when GITHUB_TOKEN is not set; Quarterdeck restarts to use it.",
                div()
                    .flex()
                    .flex_col()
                    .gap_1p5()
                    .w(px(SETTING_FIELD_WIDTH))
                    .child(self.token_sign_in().input.clone())
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                Button::new("save-token", "Sign in")
                                    .primary()
                                    .disabled(is_busy)
                                    .on_click(cx.listener(|settings, _, _, cx| {
                                        settings.sign_in(cx);
                                    })),
                            )
                            .child(
                                Button::new("new-token", "Create a token…")
                                    .ghost()
                                    .on_click(|_, _, cx| cx.open_url(NEW_TOKEN_URL)),
                            ),
                    )
                    .children(self.token_sign_in().status.describe().map(|line| {
                        let color = match self.token_sign_in().status {
                            SignInStatus::Failed(_) => theme::danger(),
                            _ => theme::muted(),
                        };
                        div().text_xs().text_color(color).child(line)
                    })),
            ))
            .into_any_element()
    }

    /// Checks the pasted token with GitHub (one request), saves it in the
    /// Keychain and restarts the app to use it.
    fn sign_in(&mut self, cx: &mut Context<Self>) {
        let token = self.token_sign_in().input.read(cx).text().trim().to_owned();
        if token.is_empty() {
            return;
        }
        self.token_sign_in_mut().status = SignInStatus::Checking;
        cx.notify();
        self.token_sign_in_mut().task = Some(cx.spawn(async move |settings, cx| {
            // Whose token it is, then save it: both block, so off the main thread.
            let signed_in = cx
                .background_executor()
                .spawn(async move {
                    let viewer = GitHubClient::new(token.clone()).viewer()?;
                    store_token(&token)?;
                    Ok::<_, GitHubError>(viewer.login)
                })
                .await;

            // Restart with the saved token, or say what went wrong.
            settings
                .update(cx, |settings, cx| {
                    settings.token_sign_in_mut().status = match signed_in {
                        Ok(login) => SignInStatus::SignedIn(login.into()),
                        Err(error) => SignInStatus::Failed(describe_sign_in_error(&error).into()),
                    };
                    cx.notify();
                    if matches!(settings.token_sign_in().status, SignInStatus::SignedIn(_)) {
                        restart_for_token(cx);
                    }
                })
                .ok();
        }));
    }

    /// Removes the saved token and restarts; the app then falls back to
    /// the `gh` CLI, if it is logged in.
    fn sign_out(&mut self, cx: &mut Context<Self>) {
        self.token_sign_in_mut().task = Some(cx.spawn(async move |settings, cx| {
            let deleted = cx
                .background_executor()
                .spawn(async move { delete_stored_token() })
                .await;
            settings
                .update(cx, |settings, cx| match deleted {
                    Ok(()) => restart_for_token(cx),
                    Err(error) => {
                        settings.token_sign_in_mut().status =
                            SignInStatus::Failed(error.to_string().into());
                        cx.notify();
                    }
                })
                .ok();
        }));
    }
}

/// Why a token could not be used, in words for the settings page.
fn describe_sign_in_error(error: &GitHubError) -> String {
    match error {
        GitHubError::Api {
            status: UNAUTHORIZED,
            ..
        } => "GitHub did not accept this token.".to_owned(),
        _ => error.to_string(),
    }
}

/// Quits and starts the app again, so it picks up the new token.
fn restart_for_token(cx: &mut gpui::App) {
    if let Err(error) = restart_app(cx) {
        eprintln!("cannot restart Quarterdeck: {error}");
    }
}
