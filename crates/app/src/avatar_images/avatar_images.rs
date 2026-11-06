use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use gpui::{App, AppContext as _, Context, Entity, EventEmitter, Global, RenderImage, Task};
use image::{Frame, RgbaImage};
use qd_github::download_avatar;

use super::AvatarLoaded;

/// The size avatars are downloaded at: sharp at 20 points on a 2x display.
const AVATAR_PIXELS: u32 = 40;
/// Avatars downloaded at once, so they leave the background executor's
/// threads to the diffs and syntax colors.
const MAX_DOWNLOADS: usize = 4;

/// The app's [`AvatarImages`], one per process.
struct GlobalAvatarImages(Entity<AvatarImages>);

impl Global for GlobalAvatarImages {}

/// Where one avatar stands.
enum AvatarState {
    /// Waiting for a download slot.
    Queued,
    /// Downloading and decoding; the task is held so it runs to the end.
    Loading {
        _task: Task<()>,
    },
    Ready(Arc<RenderImage>),
    /// It could not be loaded; it is not asked for again this session.
    Failed,
}

/// GitHub avatars by URL, downloaded and decoded in the background when
/// asked for ([`MAX_DOWNLOADS`] at a time, in the order asked), kept for the
/// session. Views observe it, or subscribe to [`AvatarLoaded`] to redraw
/// only for the avatars they show, and draw a placeholder until then.
#[derive(Default)]
pub struct AvatarImages {
    avatars: HashMap<String, AvatarState>,
    /// The asked-for avatars waiting for a download slot, first asked first.
    queued: VecDeque<String>,
    /// How many downloads run.
    downloads: usize,
}

impl EventEmitter<AvatarLoaded> for AvatarImages {}

impl AvatarImages {
    /// Creates the global store.
    pub fn install(cx: &mut App) {
        let images = cx.new(|_| Self::default());
        cx.set_global(GlobalAvatarImages(images));
    }

    /// The app's avatar store.
    pub fn global(cx: &App) -> Entity<Self> {
        cx.global::<GlobalAvatarImages>().0.clone()
    }

    /// Starts loading `urls` from anywhere (a view's event handler, a
    /// task's result): each is downloaded once per session.
    pub fn load(cx: &mut App, urls: impl IntoIterator<Item = String>) {
        let urls: Vec<String> = urls.into_iter().collect();
        Self::global(cx).update(cx, |images, cx| {
            images.request(urls.iter().map(String::as_str), cx);
        });
    }

    /// The avatar at `url`, once loaded.
    pub fn image(&self, url: &str) -> Option<Arc<RenderImage>> {
        match self.avatars.get(url)? {
            AvatarState::Ready(image) => Some(image.clone()),
            AvatarState::Queued | AvatarState::Loading { .. } | AvatarState::Failed => None,
        }
    }

    /// Queues each of `urls` not asked for before, and starts as many
    /// downloads as there are free slots.
    pub fn request<'url>(
        &mut self,
        urls: impl IntoIterator<Item = &'url str>,
        cx: &mut Context<Self>,
    ) {
        for url in urls {
            if url.is_empty() || self.avatars.contains_key(url) {
                continue;
            }
            self.avatars.insert(url.to_owned(), AvatarState::Queued);
            self.queued.push_back(url.to_owned());
        }
        self.start_queued(cx);
    }

    /// Starts the queued downloads while fewer than [`MAX_DOWNLOADS`] run,
    /// off the main thread.
    fn start_queued(&mut self, cx: &mut Context<Self>) {
        while self.downloads < MAX_DOWNLOADS {
            let Some(url) = self.queued.pop_front() else {
                break;
            };
            self.downloads += 1;
            let owned_url = url.clone();
            let task = cx.spawn(async move |images, cx| {
                // Download and decode in the background; both block.
                let download_url = owned_url.clone();
                let loaded = cx
                    .background_executor()
                    .spawn(async move { load_avatar(&download_url) })
                    .await;

                // Keep the result, tell who shows it, and start the next download.
                images
                    .update(cx, |images, cx| {
                        let state = match loaded {
                            Some(image) => AvatarState::Ready(image),
                            None => AvatarState::Failed,
                        };
                        images.avatars.insert(owned_url.clone(), state);
                        images.downloads = images.downloads.saturating_sub(1);
                        cx.emit(AvatarLoaded { url: owned_url });
                        cx.notify();
                        images.start_queued(cx);
                    })
                    .ok();
            });
            self.avatars
                .insert(url, AvatarState::Loading { _task: task });
        }
    }
}

/// Downloads and decodes one avatar for gpui; `None` (reported on stderr)
/// when either fails.
fn load_avatar(url: &str) -> Option<Arc<RenderImage>> {
    let bytes = download_avatar(url, AVATAR_PIXELS)
        .inspect_err(|error| eprintln!("avatar {url}: {error}"))
        .ok()?;
    let decoded = image::load_from_memory(&bytes)
        .inspect_err(|error| eprintln!("avatar {url}: {error}"))
        .ok()?;
    Some(Arc::new(RenderImage::new(vec![Frame::new(to_bgra(
        decoded.to_rgba8(),
    ))])))
}

/// gpui draws a `RenderImage`'s bytes as BGRA: swaps red and blue.
fn to_bgra(mut image: RgbaImage) -> RgbaImage {
    for pixel in image.pixels_mut() {
        pixel.0.swap(0, 2);
    }
    image
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Red and blue trade places; green and alpha stay.
    #[test]
    fn swaps_red_and_blue() {
        let image = RgbaImage::from_raw(1, 1, vec![1, 2, 3, 4]).unwrap();
        assert_eq!(to_bgra(image).into_raw(), [3, 2, 1, 4]);
    }
}
