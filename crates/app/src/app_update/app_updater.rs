use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use gpui::{App, AppContext as _, Context, Entity, Global, Subscription, Task};
use qd_markdown::MarkdownDocument;
use qd_update::{
    AppVersion, ReleaseManifest, RunningBundle, UpdateFeed, relaunch_after_exit,
    update_download_directory,
};

use super::update_decision::{UpdateDecision, decide_update};
use super::update_status::{ReadyUpdate, UpdateStatus};

/// The first check waits until launching (and the first refresh) is over.
const FIRST_CHECK_DELAY: Duration = Duration::from_secs(15);
/// Checks after that: the manifest is one small download, so a few per day.
const CHECK_INTERVAL: Duration = Duration::from_hours(4);

/// The app's [`AppUpdater`], one per process.
struct GlobalAppUpdater(Entity<AppUpdater>);

impl Global for GlobalAppUpdater {}

/// Keeps the app up to date: checks the newest GitHub release at launch
/// and every few hours, downloads and verifies a newer one in the
/// background, and installs it on quit or on a restart the user asks for.
/// Views observe it to show [`UpdateStatus`].
pub struct AppUpdater {
    status: UpdateStatus,
    feed: UpdateFeed,
    /// The running bundle, once found to be updatable.
    bundle: Option<RunningBundle>,
    /// The locate, check, download or install in flight.
    work_task: Option<Task<()>>,
    /// The timer of the next check.
    schedule_task: Option<Task<()>>,
    /// Keeps the quit hook (install a ready update) registered.
    _quit_subscription: Subscription,
}

impl AppUpdater {
    /// Creates the updater and starts finding the running bundle; checks
    /// follow when it can be updated.
    pub fn start(cx: &mut App) {
        let updater = cx.new(|cx| {
            let mut updater = Self {
                status: UpdateStatus::Starting,
                feed: UpdateFeed::default(),
                bundle: None,
                work_task: None,
                schedule_task: None,
                _quit_subscription: cx.on_app_quit(Self::on_app_quit),
            };
            updater.locate_bundle(cx);
            updater
        });
        cx.set_global(GlobalAppUpdater(updater));
    }

    /// The app's updater.
    pub fn global(cx: &App) -> Entity<Self> {
        cx.global::<GlobalAppUpdater>().0.clone()
    }

    pub fn status(&self) -> &UpdateStatus {
        &self.status
    }

    /// The update ready to install, if any.
    pub fn ready_update(&self) -> Option<&ReadyUpdate> {
        match &self.status {
            UpdateStatus::Ready(ready) => Some(ready),
            _ => None,
        }
    }

    /// Checks for a newer release now, unless a check, download or install runs.
    pub fn check_now(&mut self, cx: &mut Context<Self>) {
        if self.status.can_check() {
            self.check(cx);
        }
    }

    /// Installs the ready update, then quits and starts the new version.
    pub fn restart_to_update(&mut self, cx: &mut Context<Self>) {
        let (Some(ready), Some(bundle)) = (self.ready_update().cloned(), self.bundle.clone())
        else {
            return;
        };
        self.status = UpdateStatus::Installing;
        cx.notify();
        self.work_task = Some(cx.spawn(async move |updater, cx| {
            // Two renames and removing the old bundle, off the main thread.
            let installed = cx
                .background_executor()
                .spawn(async move { bundle.install(&ready.staged) })
                .await;

            // Installed: start the new version once this process is gone.
            let relaunched = installed
                .map_err(|error| error.to_string())
                .and_then(|()| relaunch_after_exit().map_err(|error| error.to_string()));
            match relaunched {
                Ok(()) => cx.update(|cx| cx.quit()).ok(),
                Err(reason) => updater
                    .update(cx, |updater, cx| {
                        updater.status = UpdateStatus::Failed(reason.into());
                        cx.notify();
                    })
                    .ok(),
            };
        }));
    }

    /// Finds the running bundle off the main thread (it runs `codesign`);
    /// an updatable one gets its leftovers removed and the checks scheduled.
    fn locate_bundle(&mut self, cx: &mut Context<Self>) {
        self.work_task = Some(cx.spawn(async move |updater, cx| {
            let located = cx
                .background_executor()
                .spawn(async move {
                    let bundle = RunningBundle::locate()?;
                    bundle.remove_leftovers();
                    Ok(bundle)
                })
                .await;
            updater
                .update(cx, |updater, cx| {
                    match located {
                        Ok(bundle) => {
                            updater.bundle = Some(bundle);
                            updater.status = UpdateStatus::UpToDate;
                            updater.schedule_check(FIRST_CHECK_DELAY, cx);
                        }
                        Err(blocker) => updater.status = UpdateStatus::Unavailable(blocker),
                    }
                    cx.notify();
                })
                .ok();
        }));
    }

    /// Checks after `delay`; every check arms the next one.
    fn schedule_check(&mut self, delay: Duration, cx: &mut Context<Self>) {
        self.schedule_task = Some(cx.spawn(async move |updater, cx| {
            cx.background_executor().timer(delay).await;
            updater.update(cx, Self::check_now).ok();
        }));
    }

    /// Reads the newest release's manifest and acts on it: downloads a newer
    /// one, or reports that the app is up to date. A ready update stays on
    /// screen while the check runs.
    fn check(&mut self, cx: &mut Context<Self>) {
        let Some(bundle) = self.bundle.clone() else {
            return;
        };
        let staged = self.ready_update().map(|ready| ready.staged.version);
        if staged.is_none() {
            self.status = UpdateStatus::Checking;
            cx.notify();
        }
        self.schedule_check(CHECK_INTERVAL, cx);

        let feed = self.feed.clone();
        self.work_task = Some(cx.spawn(async move |updater, cx| {
            // Read the manifest (one small download, no API budget).
            let latest = {
                let feed = feed.clone();
                cx.background_executor()
                    .spawn(async move { feed.latest_release() })
                    .await
            };
            let release = match latest {
                Ok(release) => release,
                Err(error) => {
                    updater
                        .update(cx, |updater, cx| updater.fail(&error.to_string(), cx))
                        .ok();
                    return;
                }
            };

            // Decide, then download only a release newer than what is running or staged.
            let decision = decide_update(release.version, AppVersion::current(), staged);
            let should_download = updater
                .update(cx, |updater, cx| {
                    match decision {
                        UpdateDecision::Download => {
                            updater.status = UpdateStatus::Downloading(release.version);
                        }
                        UpdateDecision::KeepStaged => {}
                        UpdateDecision::UpToDate => updater.status = UpdateStatus::UpToDate,
                    }
                    cx.notify();
                    decision == UpdateDecision::Download
                })
                .unwrap_or(false);
            if !should_download {
                return;
            }

            // Download, unpack beside the bundle and verify, all off the main thread.
            let staged = cx
                .background_executor()
                .spawn(async move { download_and_stage(&feed, &bundle, release) })
                .await;
            updater
                .update(cx, |updater, cx| match staged {
                    Ok(ready) => {
                        updater.status = UpdateStatus::Ready(ready);
                        cx.notify();
                    }
                    Err(reason) => updater.fail(&reason, cx),
                })
                .ok();
        }));
    }

    /// Reports a failed check; a ready update stays ready.
    fn fail(&mut self, reason: &str, cx: &mut Context<Self>) {
        eprintln!("update check failed: {reason}");
        if self.ready_update().is_none() {
            self.status = UpdateStatus::Failed(reason.to_owned().into());
            cx.notify();
        }
    }

    /// Installs a ready update while the app quits: two renames on the
    /// main thread, as gpui waits only briefly for quit hooks.
    fn on_app_quit(&mut self, _cx: &mut Context<Self>) -> impl Future<Output = ()> + use<> {
        if let (Some(ready), Some(bundle)) = (self.ready_update(), &self.bundle)
            && let Err(error) = bundle.install(&ready.staged)
        {
            eprintln!("cannot install the update: {error}");
        }
        async {}
    }
}

/// Downloads `release`, unpacks it beside `bundle` and verifies it; the
/// zip is removed once unpacked.
fn download_and_stage(
    feed: &UpdateFeed,
    bundle: &RunningBundle,
    release: ReleaseManifest,
) -> Result<ReadyUpdate, String> {
    let directory = update_download_directory().ok_or("no cache folder")?;
    let zip = feed
        .download_release(&release, &directory)
        .map_err(|error| error.to_string())?;
    let staged = bundle.stage_release(&zip, &release);
    std::fs::remove_file(&zip).ok();
    let staged = staged.map_err(|error| error.to_string())?;
    let notes = Arc::new(MarkdownDocument::parse(&release.notes));
    Ok(ReadyUpdate {
        release,
        staged,
        notes,
    })
}
