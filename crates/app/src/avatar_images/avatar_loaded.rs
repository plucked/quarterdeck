/// [`super::AvatarImages`] finished with the avatar at `url` (loaded or
/// failed), so a view that shows it redraws.
#[derive(Debug, Clone)]
pub struct AvatarLoaded {
    pub url: String,
}
