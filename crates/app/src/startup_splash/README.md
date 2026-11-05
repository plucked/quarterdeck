# Startup splash

The intro at launch. It opens a borderless, see-through window that plays the startup movie (`assets/startup-intro.mp4`, tracked by Git LFS) frame by frame on time, while the main window is already open but hidden, so its queue loads and its GitHub refresh runs underneath. When the movie ends, or on a click, the splash fades out over a second as the main window fades in, then the splash closes.

The pictures come from `qd-video`, which decodes the movie with macOS; this folder only times and draws them. When the movie is missing or cannot be read (a clone without Git LFS has only a pointer file), the splash ends at once and the main window fades in.
