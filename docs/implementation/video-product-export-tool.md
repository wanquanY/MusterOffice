# Agent 导出的运行归属与最终提交

2026-10-08。产品提交 `4d03fbcb98fc1380c1e94d3e25b6b5574f3b082f`。接续[本地导出执行与恢复](video-product-export-execution.md)。产品已实现原 Run/Tool 的导出准入、执行尝试绑定和原子正式发布端口。后续[原 Agent 调用与原生编码](video-agent-native-execution.md)已实际接通可信执行端口；原 CooperativeExecution、公开工具/技能版本及正常对话 UI 的接线仍未完成。整个视频方案仍在实施中。

## 宿主负责的能力

Musterwork 保存不可变导出意图，将每次导出尝试绑定到实际原 Run Attempt、Invocation、worker、lease 与 fence。原运行仍是唯一权限和租约来源；Schema 205 的绑定记录不创建新调度器或执行账户。原工程权限、当前运行、原工具状态和原 liveness 停止决定同时约束进度、候选字节访问及最终发布。

最终提交复用原 Artifact Tool owner 的事务内方法，把 Tool 成功、正式 Artifact/版本、原会话 Ledger、liveness terminal、导出回执与 Completed 一次提交。发生最后一步失败时整体回滚，不会先发布文件再补写工具结果。重试保留原候选 ContentId 与 source-attempt，原 Invocation 的结果 JSON 仍使用其自己的内容预算。

原 Tool 取消、运行失效或原租约过期后不能继续访问候选或推进导出；停止和清理仍可进行。手动导出继续使用 Product 来源；Agent 文件使用实际 Runtime Activity/Run/Invocation 和 executor Participant。输出沿用原 Artifact/FileQuery 体系，没有另建视频目录。

## 验证范围

新增测试使用真实 Runtime 准入、原 Tool journal、固定 SDK 工程及原 SQLite 内容存储，覆盖原子成功、精确重放、并发、最后一步失败回滚、中途撤销、Tool 提前结束、原租约失效、无权 owner、其他 Invocation 结果拒绝及旧候选恢复。这些持久层用例明确使用媒体字节夹具，不能代替真实 Agent 编码。

另回归实际原生 H.264/AAC 编码、原手动发布链、原 Artifact 提交和工程计算/编辑/撤销。正式文件再次经独立 FFmpeg 全片解码及音频比较，仍为 75 帧、128×96、3 秒、48 kHz 双声道、144000 authored samples；AAC 解码 padding 单独记录。

本轮 62 项不同测试通过：22 项原 SDK/Agent 工程与导出提交、30 项导出存储/迁移、1 项原 Artifact 提交、9 项真实媒体；生产库 strict Clippy、Device 编译检查及两份独立视频检查通过。详细用例、固定版本和日志 SHA-256 保存在产品 `.codex-work/video-export-agent-report.json`；产品实现说明为 `docs/architecture/native-video-export-tool.md`。本增量没有修改 Office 内核、SDK/worker/PPT pin、稳定 v0.1.0 标签或任何对外发行。后续须在原 CooperativeExecution 中接入这些端口，继续公开工具、技能、消息文件卡片与右侧播放器的正常产品验收。
