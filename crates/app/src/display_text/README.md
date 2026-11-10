# display_text

Turns plain values into the short text people read in the interface: GitHub timestamps as dates and ages, spans of seconds as durations, byte counts as sizes, commit SHAs cut to seven characters, and counts with their noun in the singular or plural. These are pure functions with no gpui and no state; the views decide where the text shows and how it looks.
