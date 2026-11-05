//! Which changed files are tests or generated, judged from their paths.
//! These are heuristics: GitHub's API does not say (its `linguist-generated`
//! attributes are not exposed), so common conventions are matched.

/// Folder names that hold tests or their data.
const TEST_FOLDERS: [&str; 9] = [
    "test",
    "tests",
    "__tests__",
    "spec",
    "specs",
    "testing",
    "testdata",
    "fixtures",
    "__snapshots__",
];
/// File name endings (before the extension) of test files: `foo_test.go`,
/// `foo.test.ts`, `foo.spec.ts`, `FooTest.java`, `FooTests.cs`, `foo_spec.rb`.
const TEST_NAME_ENDINGS: [&str; 6] = ["_test", ".test", ".spec", "Test", "Tests", "_spec"];
/// File names that lock or pin dependency versions.
const LOCK_FILES: [&str; 14] = [
    "cargo.lock",
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "gemfile.lock",
    "poetry.lock",
    "composer.lock",
    "go.sum",
    "podfile.lock",
    "packages.lock.json",
    "flake.lock",
    "uv.lock",
    "bun.lockb",
    "npm-shrinkwrap.json",
];
/// File name endings of generated or minified files (lower case).
const GENERATED_NAME_ENDINGS: [&str; 12] = [
    ".min.js",
    ".min.css",
    ".js.map",
    ".css.map",
    ".pb.go",
    "_pb2.py",
    "_pb2_grpc.py",
    ".g.cs",
    ".g.dart",
    ".designer.cs",
    ".snap",
    ".meta",
];
/// Folder names whose contents are generated or vendored builds.
const GENERATED_FOLDERS: [&str; 4] = ["generated", "__generated__", "node_modules", "dist"];

/// Whether `path` looks like a test: in a test folder, or named like a test file.
pub fn is_test_path(path: &str) -> bool {
    let (folders, file_name) = split_path(path);
    let in_test_folder = folders
        .iter()
        .any(|folder| TEST_FOLDERS.contains(&folder.to_lowercase().as_str()));
    let stem = file_name
        .rsplit_once('.')
        .map_or(file_name, |(stem, _)| stem);
    let python_test =
        file_name.starts_with("test_") && extension_of(path).as_deref() == Some(".py");
    in_test_folder
        || python_test
        || TEST_NAME_ENDINGS
            .iter()
            .any(|ending| stem.ends_with(ending))
}

/// Whether `path` looks generated: a lock file, a minified or generated
/// file, a Unity `.meta` file, or anything in a generated folder.
pub fn is_generated_path(path: &str) -> bool {
    let (folders, file_name) = split_path(path);
    let lower_name = file_name.to_lowercase();
    let in_generated_folder = folders
        .iter()
        .any(|folder| GENERATED_FOLDERS.contains(&folder.to_lowercase().as_str()));
    in_generated_folder
        || LOCK_FILES.contains(&lower_name.as_str())
        || lower_name.contains(".generated.")
        || GENERATED_NAME_ENDINGS
            .iter()
            .any(|ending| lower_name.ends_with(ending))
}

/// The file's extension in lower case with its dot (`.rs`), or `None` for
/// names without one (`Makefile`, `.gitignore`).
pub fn extension_of(path: &str) -> Option<String> {
    let (_, file_name) = split_path(path);
    let (stem, extension) = file_name.rsplit_once('.')?;
    (!stem.is_empty()).then(|| format!(".{}", extension.to_lowercase()))
}

/// What the file-type filter groups the file by: its extension (as
/// [`extension_of`] gives it) or, for a name without one, the whole file
/// name as written (`Dockerfile`, `.gitignore`).
pub fn file_type_of(path: &str) -> String {
    extension_of(path).unwrap_or_else(|| split_path(path).1.to_owned())
}

/// The folder names and the file name of a path.
fn split_path(path: &str) -> (Vec<&str>, &str) {
    let mut segments: Vec<&str> = path.split('/').collect();
    let file_name = segments.pop().unwrap_or_default();
    (segments, file_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests are found by folder and by naming convention across languages.
    #[test]
    fn recognizes_tests() {
        for path in [
            "crates/app/tests/build.rs",
            "src/__tests__/app.tsx",
            "pkg/server_test.go",
            "web/button.test.ts",
            "web/button.spec.js",
            "src/main/java/FooTest.java",
            "Assets/Editor/PlayerTests.cs",
            "tools/test_parse.py",
        ] {
            assert!(is_test_path(path), "{path}");
        }
        for path in ["src/latest.rs", "docs/testing-guide.md", "src/contest.rs"] {
            assert!(!is_test_path(path), "{path}");
        }
    }

    /// Lock files, minified files, generated code and Unity metadata are generated.
    #[test]
    fn recognizes_generated_files() {
        for path in [
            "Cargo.lock",
            "web/package-lock.json",
            "static/app.min.js",
            "api/service.pb.go",
            "Forms/Main.Designer.cs",
            "Assets/Player.prefab.meta",
            "src/__generated__/schema.ts",
            "client/api.generated.ts",
        ] {
            assert!(is_generated_path(path), "{path}");
        }
        for path in ["src/main.rs", "Assets/Player.prefab", "lockfile.md"] {
            assert!(!is_generated_path(path), "{path}");
        }
    }

    /// Extensions are lower case with their dot; dotfiles and bare names have none.
    #[test]
    fn reads_extensions() {
        assert_eq!(extension_of("src/Main.RS").as_deref(), Some(".rs"));
        assert_eq!(
            extension_of("Assets/a.prefab.meta").as_deref(),
            Some(".meta")
        );
        assert_eq!(extension_of("Makefile"), None);
        assert_eq!(extension_of(".gitignore"), None);
    }

    /// Names without an extension are their own file type.
    #[test]
    fn reads_file_types() {
        assert_eq!(file_type_of("src/Main.RS"), ".rs");
        assert_eq!(file_type_of("docker/Dockerfile"), "Dockerfile");
        assert_eq!(file_type_of(".gitignore"), ".gitignore");
    }
}
