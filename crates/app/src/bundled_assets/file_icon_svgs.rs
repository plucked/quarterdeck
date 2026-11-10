//! The SVG of each `FileIcon`, compiled into the binary and served by
//! `BundledIcons` under `icons/files/`, and which one the active theme draws.

use qd_file_tree::FileIcon;

use crate::theme;

/// One bundled file icon SVG: the asset path it is served at, and its bytes.
struct FileIconSvg {
    path: &'static str,
    svg: &'static [u8],
}

/// A `FileIconSvg` for `assets/icons/files/<name>.svg`.
macro_rules! file_icon_svg {
    ($name:literal) => {
        FileIconSvg {
            path: concat!("icons/files/", $name, ".svg"),
            svg: include_bytes!(concat!("../../assets/icons/files/", $name, ".svg")),
        }
    };
}

/// The asset path of the light-theme variant: the one given, else the icon itself.
macro_rules! light_variant_path {
    ($name:literal) => {
        concat!("icons/files/", $name, ".svg")
    };
    ($name:literal, $light:literal) => {
        concat!("icons/files/", $light, ".svg")
    };
}

/// Declares `FILE_ICON_SVGS` and the path functions from one line per
/// icon: its SVG's name and, for icons drawn in near-white, the darker
/// variant light themes draw instead.
macro_rules! file_icon_svgs {
    ($($variant:ident => $name:literal $(| light $light:literal)?,)*) => {
        /// Every bundled file icon SVG, light variants included.
        const FILE_ICON_SVGS: &[FileIconSvg] = &[
            $(file_icon_svg!($name), $(file_icon_svg!($light),)?)*
        ];

        /// The asset path of `icon` on a dark background.
        fn dark_theme_path(icon: FileIcon) -> &'static str {
            match icon {
                $(FileIcon::$variant => concat!("icons/files/", $name, ".svg"),)*
            }
        }

        /// The asset path of `icon` on a light background.
        fn light_theme_path(icon: FileIcon) -> &'static str {
            match icon {
                $(FileIcon::$variant => light_variant_path!($name $(, $light)?),)*
            }
        }
    };
}

file_icon_svgs! {
    File => "file",
    Rust => "rust",
    CSharp => "csharp",
    Python => "python",
    TypeScript => "typescript",
    TypeScriptDef => "typescript-def",
    JavaScript => "javascript",
    React => "react",
    ReactTs => "react_ts",
    TestTs => "test-ts",
    TestJs => "test-js",
    TestJsx => "test-jsx",
    Go => "go",
    GoMod => "go-mod",
    Java => "java",
    Kotlin => "kotlin",
    Swift => "swift",
    C => "c",
    Cpp => "cpp",
    H => "h",
    Hpp => "hpp",
    ObjectiveC => "objective-c",
    ObjectiveCpp => "objective-cpp",
    Ruby => "ruby",
    Php => "php",
    Lua => "lua",
    Dart => "dart",
    Scala => "scala",
    Haskell => "haskell",
    Elixir => "elixir",
    Zig => "zig",
    Nix => "nix",
    Console => "console",
    PowerShell => "powershell",
    Html => "html",
    Css => "css",
    Sass => "sass",
    Less => "less",
    Vue => "vue",
    Svelte => "svelte",
    Svg => "svg",
    Json => "json",
    Yaml => "yaml",
    Toml => "toml" | light "toml_light",
    Xml => "xml",
    Markdown => "markdown",
    Mdx => "mdx",
    Readme => "readme",
    Changelog => "changelog",
    License => "license",
    Document => "document",
    Pdf => "pdf",
    Table => "table",
    Database => "database",
    GraphQl => "graphql",
    Proto => "proto",
    Razor => "razor",
    Shader => "shader",
    Terraform => "terraform",
    Bazel => "bazel",
    Docker => "docker",
    Git => "git",
    Gradle => "gradle",
    Maven => "maven",
    Http => "http",
    Url => "url",
    Image => "image",
    Photoshop => "adobe-photoshop" | light "adobe-photoshop_light",
    Font => "font",
    Audio => "audio",
    Video => "video",
    Zip => "zip",
    Lock => "lock",
    Key => "key",
    Settings => "settings",
    Exe => "exe",
    Dll => "dll",
    Lib => "lib",
    Model3d => "3d",
    Hex => "hex",
    VisualStudio => "visualstudio",
    NuGet => "nuget",
    Unity => "unity",
    EditorConfig => "editorconfig" | light "editorconfig_light",
    CodeOwners => "codeowners",
    NodeJs => "nodejs",
    Npm => "npm",
    Yarn => "yarn",
    TsConfig => "tsconfig",
    Eslint => "eslint",
    Prettier => "prettier",
    Vite => "vite",
    Vitest => "vitest",
    Jest => "jest",
    Next => "next" | light "next_light",
    Playwright => "playwright",
    PostCss => "postcss",
    Claude => "claude",
    Skill => "skill",
    Agent => "agent",
    Env => "tune",
    Makefile => "makefile",
    CMake => "cmake",
    VsCode => "vscode",
    GitHubActions => "github-actions-workflow",
    Jar => "jar",
    Android => "android",
    Diff => "diff",
    Template => "template",
    Log => "log",
    Jupyter => "jupyter",
    UnityMeta => "unity-meta",
    UnityAsmdef => "unity-asmdef",
    UnityPrefab => "unity-prefab",
    UnityMaterial => "unity-material",
    UnityAsset => "unity-asset",
    UnityAnimation => "unity-animation",
    ShaderGraph => "shader-graph",
    UiTemplate => "ui-template",
    UiStyle => "ui-style",
    Schema => "schema",
}

/// The asset path of `icon`'s SVG in the active theme.
pub fn file_icon_path(icon: FileIcon) -> &'static str {
    if theme::is_dark_theme() {
        dark_theme_path(icon)
    } else {
        light_theme_path(icon)
    }
}

/// The file icon SVG served at asset `path`, if `path` names one.
pub fn file_icon_svg(path: &str) -> Option<&'static [u8]> {
    FILE_ICON_SVGS
        .iter()
        .find(|icon| icon.path == path)
        .map(|icon| icon.svg)
}

/// The asset paths of every file icon SVG.
pub fn file_icon_paths() -> impl Iterator<Item = &'static str> {
    FILE_ICON_SVGS.iter().map(|icon| icon.path)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    /// No two bundled SVGs share a path, so each serves its own picture.
    #[test]
    fn paths_are_unique() {
        let paths: HashSet<&str> = file_icon_paths().collect();
        assert_eq!(paths.len(), FILE_ICON_SVGS.len());
    }

    /// Both themes' paths of an icon are served, light variants included.
    #[test]
    fn every_path_is_served() {
        for icon in [
            FileIcon::File,
            FileIcon::Rust,
            FileIcon::Toml,
            FileIcon::UnityMeta,
        ] {
            assert!(file_icon_svg(dark_theme_path(icon)).is_some());
            assert!(file_icon_svg(light_theme_path(icon)).is_some());
        }
        assert_eq!(
            light_theme_path(FileIcon::Toml),
            "icons/files/toml_light.svg"
        );
        assert_eq!(dark_theme_path(FileIcon::Toml), "icons/files/toml.svg");
    }
}
