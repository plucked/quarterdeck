use std::borrow::Cow;

use gpui::App;

/// The fonts built into the binary (`assets/fonts`, SIL Open Font License,
/// licences beside them): `Schibsted Grotesk` for the interface and
/// `JetBrains Mono` for code, one static file per weight and style the app
/// draws, so bold and medium text get their real cuts.
const BUNDLED_FONTS: [&[u8]; 9] = [
    include_bytes!("../../assets/fonts/SchibstedGrotesk-Regular.ttf"),
    include_bytes!("../../assets/fonts/SchibstedGrotesk-Medium.ttf"),
    include_bytes!("../../assets/fonts/SchibstedGrotesk-SemiBold.ttf"),
    include_bytes!("../../assets/fonts/SchibstedGrotesk-Bold.ttf"),
    include_bytes!("../../assets/fonts/SchibstedGrotesk-Italic.ttf"),
    include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf"),
    include_bytes!("../../assets/fonts/JetBrainsMono-Bold.ttf"),
    include_bytes!("../../assets/fonts/JetBrainsMono-Italic.ttf"),
    include_bytes!("../../assets/fonts/JetBrainsMono-BoldItalic.ttf"),
];

/// Makes the bundled fonts available to every window under their family
/// names (`theme::UI_FONT`, `theme::MONO_FONT`); call before opening one.
/// On failure the text falls back to the system's fonts.
pub fn load_bundled_fonts(cx: &App) {
    let fonts = BUNDLED_FONTS
        .iter()
        .map(|font| Cow::Borrowed(*font))
        .collect();
    if let Err(error) = cx.text_system().add_fonts(fonts) {
        eprintln!("cannot load the bundled fonts: {error:#}");
    }
}
