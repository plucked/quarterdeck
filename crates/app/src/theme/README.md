# Theme

The app's look in one place: eight color themes (four light, four dark) and the two fonts. Each theme starts as a designed palette of about twenty colors; from it the rest of the colors the views need (raised surfaces, hover, secondary text) are worked out, with secondary text faded only as far as its contrast target allows. One theme is active at a time, set from the user's preference; views ask for colors by role (`text()`, `added()`, `syntax(kind)`) and never hold hex values themselves. Saving the preference is `AppSettings`' job, and picking one is the settings window's.
