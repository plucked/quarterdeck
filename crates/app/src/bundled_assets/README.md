# bundled_assets

Hands gpui the fonts and icons that are compiled into the binary from the crate's `assets` folder: registers the interface and code fonts at launch, serves the SVG icons as the app's asset source, and picks each file type's icon for the active theme. It owns no drawing: views place the icons and theme chooses the fonts by name. The files themselves live in `assets/`, with their licences beside them.
