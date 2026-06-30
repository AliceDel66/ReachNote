use serde::Serialize;

pub mod task;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ShellStatus {
    pub product: &'static str,
    pub phase: &'static str,
    pub default_screen: &'static str,
}

pub fn shell_status() -> ShellStatus {
    ShellStatus {
        product: "ReachNote",
        phase: "static-shell",
        default_screen: "queue",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_status_defaults_to_queue() {
        let status = shell_status();

        assert_eq!(status.product, "ReachNote");
        assert_eq!(status.phase, "static-shell");
        assert_eq!(status.default_screen, "queue");
    }
}
