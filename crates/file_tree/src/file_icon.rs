use crate::file_icon_rules::{
    COPY_EXTENSIONS, EXTENSION_ICONS, FILE_NAME_ICONS, FILE_NAME_PREFIX_ICONS, WORKFLOW_FOLDER,
};

/// The icon a file shows in the file tree, picked from its path. The app
/// draws each with a bundled SVG; most come from the Material Icon Theme;
/// the C#, Unity, UI and schema ones were drawn to match it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileIcon {
    /// Any file no rule knows.
    File,
    Rust,
    CSharp,
    Python,
    TypeScript,
    /// TypeScript declarations (`.d.ts`).
    TypeScriptDef,
    JavaScript,
    React,
    ReactTs,
    /// TypeScript tests (`.test.ts`, `.spec.ts`).
    TestTs,
    /// JavaScript tests (`.test.js`, `.spec.mjs`).
    TestJs,
    /// JSX and TSX tests (`.test.tsx`).
    TestJsx,
    Go,
    /// `go.mod`, `go.sum`, `go.work`.
    GoMod,
    Java,
    Kotlin,
    Swift,
    C,
    Cpp,
    /// C headers.
    H,
    /// C++ headers.
    Hpp,
    ObjectiveC,
    ObjectiveCpp,
    Ruby,
    Php,
    Lua,
    Dart,
    Scala,
    Haskell,
    Elixir,
    Zig,
    Nix,
    /// Shell scripts and batch files.
    Console,
    PowerShell,
    Html,
    Css,
    Sass,
    Less,
    Vue,
    Svelte,
    Svg,
    Json,
    Yaml,
    Toml,
    Xml,
    Markdown,
    Mdx,
    Readme,
    Changelog,
    License,
    /// Plain text.
    Document,
    Pdf,
    /// Spreadsheets and CSV.
    Table,
    /// SQL and database files.
    Database,
    GraphQl,
    /// Protocol buffers.
    Proto,
    Razor,
    /// Shader sources (HLSL, GLSL, Unity `.shader`).
    Shader,
    Terraform,
    Bazel,
    Docker,
    Git,
    Gradle,
    Maven,
    /// HTTP request files.
    Http,
    Url,
    Image,
    Photoshop,
    Font,
    Audio,
    Video,
    /// Archives.
    Zip,
    /// Lock files without a more specific icon.
    Lock,
    /// Keys and certificates.
    Key,
    /// Configuration (`.ini`, `.config`, `.props`).
    Settings,
    Exe,
    /// Shared libraries (`.dll`, `.so`, `.dylib`).
    Dll,
    /// Static libraries (`.a`, `.lib`).
    Lib,
    /// 3D models.
    Model3d,
    /// Binary data.
    Hex,
    /// Visual Studio solutions and `MSBuild` projects.
    VisualStudio,
    NuGet,
    /// Unity scenes and packages.
    Unity,
    EditorConfig,
    CodeOwners,
    /// `package.json` and Node version files.
    NodeJs,
    Npm,
    Yarn,
    TsConfig,
    Eslint,
    Prettier,
    Vite,
    Vitest,
    Jest,
    Next,
    Playwright,
    PostCss,
    /// `CLAUDE.md`.
    Claude,
    /// `SKILL.md`.
    Skill,
    /// `AGENTS.md`.
    Agent,
    /// `.env` files.
    Env,
    Makefile,
    CMake,
    VsCode,
    /// GitHub Actions workflows and actions.
    GitHubActions,
    Jar,
    /// Android packages (`.aar`, `.apk`).
    Android,
    /// Patches and diffs.
    Diff,
    /// Templates (Scriban `.sbn`, `.tpl`, Handlebars).
    Template,
    Log,
    Jupyter,
    /// Unity's `.meta` import settings beside every asset.
    UnityMeta,
    /// Unity assembly definitions and references.
    UnityAsmdef,
    UnityPrefab,
    /// Unity materials and physics materials.
    UnityMaterial,
    /// Unity's other serialized assets (`.asset`, `.preset`, `.lighting`).
    UnityAsset,
    /// Unity animation clips and animator controllers.
    UnityAnimation,
    /// Unity Shader Graph and VFX Graph files.
    ShaderGraph,
    /// UI templates (`.ui`, `.uimodule`, `.uxml`).
    UiTemplate,
    /// UI style sheets (`.style`, `.uss`, `.tss`).
    UiStyle,
    /// `.schema` interface definitions.
    Schema,
}

impl FileIcon {
    /// The icon of the file at `path`. Rules are tried from the most to the
    /// least specific: the folder (GitHub workflows), the whole file name
    /// (`Cargo.toml`, `.gitignore`), the start of the name (`.env.local`,
    /// `vite.config.ts`), then the longest known extension (`.test.ts`
    /// before `.ts`), so a dotfile like `.dockerignore` matches by its
    /// extension when no name rule names it. A copy (`settings.json.example`)
    /// shows the icon of what it copies. Names compare in lower case.
    pub fn of(path: &str) -> Self {
        let (folder, file_name) = path.rsplit_once('/').unwrap_or(("", path));
        let name = file_name.to_lowercase();
        // A copy's original decides when it has an icon; `foo.template` alone keeps its own.
        let original = name
            .rsplit_once('.')
            .filter(|(stem, extension)| !stem.is_empty() && COPY_EXTENSIONS.contains(extension))
            .map(|(stem, _)| Self::of_file_name(folder, stem))
            .filter(|icon| *icon != Self::File);
        original.unwrap_or_else(|| Self::of_file_name(folder, &name))
    }

    /// The icon of the lower-case file `name` in `folder`, by the rules in
    /// the order [`FileIcon::of`] gives.
    fn of_file_name(folder: &str, name: &str) -> Self {
        // A workflow file is YAML in `.github/workflows`, named anything.
        let is_yaml = matches!(name.rsplit_once('.'), Some((_, "yml" | "yaml")));
        if is_yaml && folder.ends_with(WORKFLOW_FOLDER) {
            return Self::GitHubActions;
        }
        // A whole-name rule wins over the extension: `cargo.toml` is Rust.
        if let Some(icon) = icon_in(FILE_NAME_ICONS, name) {
            return icon;
        }
        // Name families: `.env.production`, `dockerfile.dev`, `tsconfig.app.json`.
        if let Some((_, icon)) = FILE_NAME_PREFIX_ICONS
            .iter()
            .find(|(prefix, _)| name.starts_with(prefix))
        {
            return *icon;
        }
        // Each dot starts a candidate extension, the longest first.
        name.match_indices('.')
            .find_map(|(index, _)| icon_in(EXTENSION_ICONS, &name[index + 1..]))
            .unwrap_or(Self::File)
    }
}

/// The icon `table` gives `key`, if it lists it.
fn icon_in(table: &[(&str, FileIcon)], key: &str) -> Option<FileIcon> {
    table
        .iter()
        .find(|(candidate, _)| *candidate == key)
        .map(|(_, icon)| *icon)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Common source files get their language's icon, whatever the case.
    #[test]
    fn picks_icons_by_extension() {
        for (path, icon) in [
            ("crates/app/src/main.rs", FileIcon::Rust),
            ("Assets/Scripts/Player.CS", FileIcon::CSharp),
            ("web/app/page.tsx", FileIcon::ReactTs),
            ("tools/build.py", FileIcon::Python),
            ("infra/main.tf", FileIcon::Terraform),
            ("docs/guide.md", FileIcon::Markdown),
            ("Makefile.unknown-thing", FileIcon::File),
            ("bin/tool", FileIcon::File),
        ] {
            assert_eq!(FileIcon::of(path), icon, "{path}");
        }
    }

    /// Longer extensions win: tests, declarations, `.d.ts` over `.ts`.
    #[test]
    fn prefers_the_longest_extension() {
        assert_eq!(FileIcon::of("web/route.test.ts"), FileIcon::TestTs);
        assert_eq!(FileIcon::of("web/app.spec.tsx"), FileIcon::TestJsx);
        assert_eq!(FileIcon::of("types/env.d.ts"), FileIcon::TypeScriptDef);
        assert_eq!(FileIcon::of("web/route.ts"), FileIcon::TypeScript);
    }

    /// Dotfiles and files without an extension are found by name.
    #[test]
    fn picks_icons_for_dotfiles_and_known_names() {
        for (path, icon) in [
            (".gitignore", FileIcon::Git),
            ("server/.dockerignore", FileIcon::Docker),
            ("server/Dockerfile.dockerignore", FileIcon::Docker),
            ("server/Dockerfile", FileIcon::Docker),
            ("server/Dockerfile.dev", FileIcon::Docker),
            (".editorconfig", FileIcon::EditorConfig),
            (".gitattributes", FileIcon::Git),
            ("web/.env.example", FileIcon::Env),
            (".env", FileIcon::Env),
            ("Cargo.toml", FileIcon::Rust),
            ("crates/app/Cargo.lock", FileIcon::Rust),
            ("BUILD.bazel", FileIcon::Bazel),
            (".bazelrc", FileIcon::Bazel),
            ("CODEOWNERS", FileIcon::CodeOwners),
            ("LICENSE", FileIcon::License),
            ("README.md", FileIcon::Readme),
            ("CLAUDE.md", FileIcon::Claude),
            ("web/package.json", FileIcon::NodeJs),
            ("web/tsconfig.app.json", FileIcon::TsConfig),
            ("web/vite.config.ts", FileIcon::Vite),
            ("web/eslint.config.mjs", FileIcon::Eslint),
            (".prettierrc", FileIcon::Prettier),
            ("fastlane/Gymfile", FileIcon::Ruby),
            ("VERSION", FileIcon::Document),
        ] {
            assert_eq!(FileIcon::of(path), icon, "{path}");
        }
    }

    /// Unity's own file types each get their icon.
    #[test]
    fn picks_icons_for_unity_files() {
        for (path, icon) in [
            ("Assets/Player.cs.meta", FileIcon::UnityMeta),
            ("Assets/Game.asmdef", FileIcon::UnityAsmdef),
            ("Assets/Foo.prefab", FileIcon::UnityPrefab),
            ("Assets/Skin.mat", FileIcon::UnityMaterial),
            ("ProjectSettings/TagManager.asset", FileIcon::UnityAsset),
            ("Assets/Walk.anim", FileIcon::UnityAnimation),
            ("Assets/Water.shadergraph", FileIcon::ShaderGraph),
            ("Assets/Water.hlsl", FileIcon::Shader),
            ("Assets/Main.unity", FileIcon::Unity),
            ("Assets/UI/AppRoot.ui", FileIcon::UiTemplate),
            ("Assets/UI/AppRoot.style", FileIcon::UiStyle),
            ("contracts/eco/permissions.schema", FileIcon::Schema),
        ] {
            assert_eq!(FileIcon::of(path), icon, "{path}");
        }
    }

    /// A copy shows its original's icon; a lone `.template` keeps the template icon.
    #[test]
    fn copies_show_the_original_icon() {
        assert_eq!(FileIcon::of("settings.json.example"), FileIcon::Json);
        assert_eq!(FileIcon::of("web/toolbox.html.backup"), FileIcon::Html);
        assert_eq!(FileIcon::of("infra/vars.template"), FileIcon::Template);
        assert_eq!(FileIcon::of("notes.old"), FileIcon::File);
    }

    /// YAML under `.github/workflows` is a workflow, elsewhere plain YAML.
    #[test]
    fn recognizes_github_workflows() {
        assert_eq!(
            FileIcon::of(".github/workflows/ci.yml"),
            FileIcon::GitHubActions
        );
        assert_eq!(FileIcon::of("deploy/ci.yml"), FileIcon::Yaml);
    }
}
