use zed_extension_api::{
    self as zed, Command, LanguageServerId, Os, Result, Worktree, current_platform, serde_json,
};

struct GlintExtension;

const PLUGIN_PACKAGE: &str = "@glint/tsserver-plugin";

// In priority order. typescript-language-server is the most common; vtsls is
// the VS Code-flavored alternative and accepts the same `--stdio` flag.
const SERVER_NAMES: &[&str] = &["typescript-language-server", "vtsls"];

impl GlintExtension {
    /// Existence probe that works inside Zed's WASM sandbox. Raw `std::fs`
    /// against the host filesystem isn't part of the extension API contract,
    /// but `read_text_file` is — and npm `.bin` shims are text on every
    /// platform we care about (shell scripts on Unix, `.cmd` on Windows).
    fn worktree_file_exists(worktree: &Worktree, path: &str) -> bool {
        worktree.read_text_file(path).is_ok()
    }

    fn is_windows() -> bool {
        matches!(current_platform().0, Os::Windows)
    }

    /// Join a worktree-relative path onto the absolute worktree root with the
    /// host's native separator. The wasm target's `PathBuf` always uses `/`,
    /// which produces ugly mixed-separator paths on Windows (`C:\foo/bar`) —
    /// Win32 tolerates them but logs and error messages look broken.
    fn join_root(root: &str, rel: &str) -> String {
        if Self::is_windows() {
            format!("{root}\\{}", rel.replace('/', "\\"))
        } else {
            format!("{root}/{rel}")
        }
    }

    /// On Windows, npm installs CLIs as `.cmd` shims (and occasionally `.exe`),
    /// so the bare name doesn't resolve from `node_modules/.bin`. `Worktree::which`
    /// already honors PATHEXT for the PATH fallback, so this is only needed for
    /// the project-local lookup.
    fn binary_candidates(name: &str) -> Vec<String> {
        if Self::is_windows() {
            vec![format!("{name}.cmd"), format!("{name}.exe"), name.into()]
        } else {
            vec![name.into()]
        }
    }

    /// Look for a TypeScript language server, preferring the project's local
    /// install (so it picks up the project's TypeScript version), then PATH.
    fn find_ts_server(&self, worktree: &Worktree) -> Option<(String, Vec<String>)> {
        let root = worktree.root_path();
        let args = vec!["--stdio".into()];

        for name in SERVER_NAMES {
            for candidate in Self::binary_candidates(name) {
                let rel = format!("node_modules/.bin/{candidate}");
                if Self::worktree_file_exists(worktree, &rel) {
                    return Some((Self::join_root(&root, &rel), args));
                }
            }
            if let Some(path) = worktree.which(name) {
                return Some((path, args));
            }
        }
        None
    }
}

impl zed::Extension for GlintExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        match self.find_ts_server(worktree) {
            Some((command, args)) => Ok(Command {
                command,
                args,
                env: worktree.shell_env(),
            }),
            None => Err(
                "No TypeScript language server found in this project or on PATH.\n\
                 Install one (use whichever package manager matches your project):\n  \
                   npm install -D typescript-language-server typescript     (project-local, recommended)\n  \
                   pnpm add  -D typescript-language-server typescript\n  \
                   yarn add  -D typescript-language-server typescript\n  \
                   npm install -g typescript-language-server typescript     (global)\n\
                 On macOS, if Zed was launched from Spotlight/Finder its PATH may not include \
                 your shell's PATH — launch Zed from a terminal, or run `launchctl setenv PATH \"$PATH\"`."
                    .into(),
            ),
        }
    }

    /// Tell the underlying tsserver to load `@glint/tsserver-plugin` from the
    /// project's node_modules. This is the editor-side equivalent of putting
    /// `{ "name": "@glint/tsserver-plugin" }` in tsconfig.json's
    /// compilerOptions.plugins array — same effect, but local to this editor.
    fn language_server_initialization_options(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<serde_json::Value>> {
        // We don't gate the LSP on plugin presence — if @glint/tsserver-plugin
        // can't be loaded, plain TS features still work, which is strictly
        // better than refusing to start.
        //
        // `location` is the parent directory the plugin should be `require`d
        // from. Node's resolver walks up from there looking for
        // `node_modules/<name>`, so pointing it at the worktree's
        // `node_modules` dir works for the common case (and the previous
        // version of this extension shipped this exact value successfully).
        // For Yarn PnP / global-only installs we still send this — tsserver
        // logs a "plugin not found" warning and continues, which is the
        // graceful-degradation path we want.
        let node_modules = Self::join_root(&worktree.root_path(), "node_modules");

        // We send the plugin spec under several keys to cover both
        // typescript-language-server and vtsls option shapes. Unknown keys
        // are ignored by each server, so it's safe to send all of them.
        Ok(Some(serde_json::json!({
            // typescript-language-server: top-level `plugins`.
            "plugins": [
                {
                    "name": PLUGIN_PACKAGE,
                    "location": node_modules,
                }
            ],
            // typescript-language-server: also accepts these for tsserver-level config.
            "tsserver": {
                "globalPlugins": [PLUGIN_PACKAGE],
                "pluginPaths": [node_modules],
            },
            // vtsls: nested.
            "vtsls": {
                "tsserver": {
                    "globalPlugins": [
                        {
                            "name": PLUGIN_PACKAGE,
                            "location": node_modules,
                            "enableForWorkspaceTypeScriptVersions": true,
                        }
                    ],
                }
            },
            // Help vtsls/tsserver actually use the project's TypeScript.
            "preferences": {
                "includePackageJsonAutoImports": "on"
            }
        })))
    }
}

zed::register_extension!(GlintExtension);
