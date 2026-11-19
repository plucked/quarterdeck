use qd_github::PullRequest;

use crate::app_services::AppServices;
use crate::app_settings::AppSettings;
use crate::avatar_images::AvatarImages;
use crate::shared_cache::SharedCache;
use crate::ui_state::UiStateRecorder;

/// A pull request whose version and identity are controlled by the test.
pub(in crate::pull_request_detail) fn pull_request(id: u64, version: &str) -> PullRequest {
    PullRequest {
        id,
        number: id,
        repo: qd_github::RepoRef {
            owner: "owner".into(),
            name: "repo".into(),
        },
        title: "title".into(),
        body: None,
        author: qd_github::User {
            login: "user".into(),
            avatar_url: String::new(),
        },
        html_url: String::new(),
        draft: false,
        created_at: "start".into(),
        updated_at: version.into(),
        labels: vec![],
        stack: None,
        branches: None,
        viewer_review: None,
    }
}

/// Sets up the isolated cache and UI settings used by a cached review.
pub(in crate::pull_request_detail) fn cached_services(
    cx: &mut gpui::TestAppContext,
    directory: &std::path::Path,
) -> AppServices {
    let database = directory.join("cache");
    let recorder = UiStateRecorder::open(&database).unwrap();
    let cache = SharedCache::new(qd_cache::GitHubCache::open(&database).unwrap());
    let services = AppServices {
        github: Err("offline".into()),
        token_source: None,
        cache,
        database,
    };
    cx.update(|app| {
        app.set_global(AppSettings::load(&recorder));
        app.set_global(recorder);
        AvatarImages::install(app);
    });
    services
}
