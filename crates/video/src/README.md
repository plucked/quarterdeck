# qd-video source

Turns a movie file into pictures the app can draw: it opens the file's first video track with the system's AVFoundation, has it decoded into 32-bit BGRA, and hands out one frame at a time with its presentation time. It does not play, time or draw anything (the app's startup splash does), and it knows nothing of gpui.
