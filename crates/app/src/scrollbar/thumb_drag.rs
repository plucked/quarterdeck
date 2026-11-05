/// The drag state of one scrollbar, kept by gpui across frames.
#[derive(Default)]
pub struct ThumbDrag {
    /// While the thumb is dragged: how far below the thumb's top it was
    /// grabbed, so the thumb does not jump under the pointer.
    pub grab_offset: Option<f32>,
}
