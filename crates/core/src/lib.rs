// from_str 系列方法有意返回 Option（解析外部/DB 文本，非法值即 None），
// 不实现 std::str::FromStr，因此放宽该 lint。
#![allow(clippy::should_implement_trait)]

use serde::Serialize;

pub mod analysis;
pub mod notion;
pub mod platform;
pub mod task;
pub mod template;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ShellStatus {
    pub product: &'static str,
    pub phase: &'static str,
    pub default_screen: &'static str,
}

pub fn shell_status() -> ShellStatus {
    ShellStatus {
        product: "ReachNote",
        phase: "local-analysis",
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
        assert_eq!(status.phase, "local-analysis");
        assert_eq!(status.default_screen, "queue");
    }
}
