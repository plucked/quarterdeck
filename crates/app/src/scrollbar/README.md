# scrollbar

Draws a vertical scrollbar beside a scroll container and lets the mouse drive it. It reads and moves the container's gpui `ScrollHandle`; it does not own the content or the scroll position, and the container still scrolls with the wheel on its own. A view places the scrollbar over the container's right edge, inside a wrapper with `relative()`.
