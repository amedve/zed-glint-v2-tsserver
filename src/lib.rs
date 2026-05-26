use std::path::PathBuf;
use zed_extension_api::{
    self as zed, serde_json, Command, LanguageServerId, Result, Worktree,
};

struct GlintExtension;

impl GlintExtension {
    /// Look for a TypeScript language server, preferring the project's local
    /// install (so it picks up the project's TypeScript version), then PATH.
    fn find_ts_server(&self, worktree: &Worktree) -> Option<(String, Vec<String>)> {
        let root = PathBuf::from(worktree.root_path());
        let local_bin = root.join("node_modules").join(".bin");

        // Names to try, in priority order. typescript-language-server is the
        // most common; vtsls is the VS Code-flavored alternative.
        for name in &["typescript-language-server", "vtsls"] {
            // Project-local first.
            let local = local_bin.join(name);
            if std::fs::metadata(&local).is_ok() {
                return Some((
                    local.to_string_lossy().into_owned(),
                    vec!["--stdio".into()],
                ));
            }
            // Then PATH.
            if let Some(path) = worktree.which(name) {
                return Some((path, vec!["--stdio".into()]));
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
                "No TypeScript language server found. Install one with:\n  \
                 npm install -g typescript-language-server typescript\n\
                 or as a project devDependency."
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
        let node_modules = PathBuf::from(worktree.root_path()).join("node_modules");
        let location = node_modules.to_string_lossy().into_owned();

        // We send the plugin spec under several keys to cover both
        // typescript-language-server and vtsls option shapes.
        Ok(Some(serde_json::json!({
            // typescript-language-server: top-level `plugins`.
            "plugins": [
                {
                    "name": "@glint/tsserver-plugin",
                    "location": location,
                }
            ],
            // typescript-language-server: also accepts these.
            "tsserver": {
                "globalPlugins": ["@glint/tsserver-plugin"],
                "pluginPaths": [location],
            },
            // vtsls: nested.
            "vtsls": {
                "tsserver": {
                    "globalPlugins": [
                        {
                            "name": "@glint/tsserver-plugin",
                            "location": location,
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
