# Review Gate

Last updated: 2026-06-30

## Current Snapshot

状态：Planned

后续每个可运行实现切片完成后，必须进行工程 review/gate。PRD 阶段只记录规则，不执行代码 gate。

## Gate Rules

- Review 重点：行为回归、测试缺口、权限/安全问题、数据契约破坏、部署风险、未清理调试代码。
- 有 P0/P1 finding 时不得 ship。
- 需要 AI review 时，另一个 agent/model 只读审阅，不直接改文件。

