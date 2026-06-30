use std::env;
use std::path::{Path, PathBuf};

pub struct ClaudeCliAvailability {
    command: String,
}

impl ClaudeCliAvailability {
    pub fn from_env() -> Self {
        let command = env::var("REACHNOTE_CLAUDE_CMD")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "claude".to_string());

        Self { command }
    }

    pub fn check(&self) -> Result<(), String> {
        if is_path_like(&self.command) {
            return executable_path(&PathBuf::from(&self.command))
                .map(|_| ())
                .ok_or_else(|| self.missing_message());
        }

        env::var_os("PATH")
            .and_then(|path| env::split_paths(&path).find_map(|dir| executable_path(&dir.join(&self.command))))
            .map(|_| ())
            .ok_or_else(|| self.missing_message())
    }

    fn missing_message(&self) -> String {
        format!(
            "未找到 Claude CLI：当前 PATH 中没有可执行的 `{}`。请安装 Claude CLI，或设置 REACHNOTE_CLAUDE_CMD 指向可执行文件。",
            self.command
        )
    }
}

fn is_path_like(command: &str) -> bool {
    command.contains('/') || command.contains('\\') || Path::new(command).is_absolute()
}

fn executable_path(path: &Path) -> Option<PathBuf> {
    let metadata = path.metadata().ok()?;
    if metadata.is_file() && is_executable(&metadata) {
        Some(path.to_path_buf())
    } else {
        None
    }
}

#[cfg(unix)]
fn is_executable(metadata: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;

    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(metadata: &std::fs::Metadata) -> bool {
    !metadata.permissions().readonly()
}
