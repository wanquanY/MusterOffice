# 原生工具提交时间与丢响应恢复

2026-09-28，接续[原生 Agent 工具接入](product-native-agent.md)。产品实现和验证继续只使用 `codex/musteroffice-integration` worktree；没有修改 Musterwork 主工作区。本阶段修改产品适配和原事务，不修改固定 SDK/worker 或计算内核。

## 时间由谁记录

原生准备曾将原请求时间同时用于计算身份和 Tool 完成、Artifact 发布记录。请求排队或计算耗时较长时，这会让完成时间早于开始时间，且不能据此计算耗时。

| 数据 | 时间来源 | 重试行为 |
| --- | --- | --- |
| 原生 revision、draft、candidate、checkpoint 中的计算元数据 | 原 Invocation 的稳定逻辑时间 | 保留完全相同的字节、引用与 fingerprint |
| Tool terminal、presentation commit command、Ledger event 的发生时间 | 原 SQLite writer 事务中的数据库时钟 | 首次成功写入；已成功调用返回原记录 |
| Artifact 首次创建、版本发布及更新时间 | 同一事务的发布观察时间 | 重复调用不创建版本，不刷新时间 |

Office 候选新增显式 `artifact_mutation_at` 发布投影。准备阶段继续使用稳定投影；只有原提交者在验证已准备命令后提供数据库时间。输入内容、Artifact/版本身份、作者、质量声明与依赖闭包保持一致。原 `committed_at` 命令字段因既有 checkpoint/fingerprint 兼容继续保留；对 Office 它表示稳定命令时间，不可当作实际完成时间。旧 HTML 合同保留原有语义。

这没有增加持久作业、第二套数据库或内核存储。逻辑元数据不能拿来估算耗时；性能测量仍需要明确输入、设备、缓存和计时范围。事务内采样是提交观察点，不宣称是磁盘 fsync 完成时刻。

## 原事务的最终检查

计算结束与获取数据库 writer 之间可能发生超时。新 Office 提交在原事务中重查 Tool 仍在当前 Attempt 执行、期限未过、时间顺序有效；Run 取消与 writer fence 沿用原检查。过期返回明确 `DeadlineExceeded`，Tool executor 映射为原有 `DependencyTimeout`。失败时草稿、Artifact、Tool 成功和 Ledger 一起回滚。

精确重复提交先匹配原 fingerprint 和回执。即使调用期限随后结束，也能读取已提交的结果；不会重新获得写权限或更新完成时间。更换 writer generation 后，旧连接的提交继续被拒绝。

## 实际恢复验证

专项从固定 v51 合同构造调用，使用真实公共 SDK、原 SQLite journal、原子处理器和 `MusterworkToolExecutor`。测试在成功提交后、返回调用方前挂起响应，再取消该调用、重新打开原数据库并重建执行器；重复执行原计划只返回原模型可见结果。内核分派计数为一次，草稿、成功命令、Ledger 数量及输出配额保持不变。

另外验证了准备后超时的完整回滚、checkpoint 恢复与精确重复回执、旧 writer 拒绝，以及 Tool、Ledger、Artifact 时间的一致性。实际生成两版 PPTX 后再次导入，两版各 21 个 ZIP 部件的 CRC/XML 检查通过，文件逐字节等于上一阶段对应导出；本次时间修正没有改变文稿字节。

本次 7 项原生工具专项、71 项其他演示文稿测试、46 项 Artifact 库测试通过，合计 124 项不同测试。71 项中包含全部 23 项 Office 草稿/准备/导出回归。扩大执行 `--include-ignored` 时，另 3 项旧引擎用例分别因未配置 Chromium 路径、缺少浏览器生成清单、缺少本 worktree 的 Python 虚拟环境失败；没有移除测试或将失败计为通过。

runtime-artifact、SQLite、tool-executor、device-runtime 四个生产库的严格 Clippy（`--lib --no-deps -- -D warnings`）通过。最初包含依赖的严格检查遇到未修改的旧 capability 文件 `collapsible_if`；全依赖范围不宣称通过。

测试来源、命令、源码摘要及完整回归结果见[本阶段证据](../reviews/evidence/2026-09-28-product-native-recovery-verification.json)。上述恢复是受控丢响应、调用取消和数据库重新打开，不等价于整个桌面进程被系统杀死、设备掉电或真实模型会话验收。发布包性能、全量质量声明、Office/WPS 编辑往返仍未证明。

## 下一步

继续模型侧原生 Skill/发现、产品字体与导出环境解析、模板与历史文稿接入，再进行真实 Agent/桌面会话和完整故障注入。播放、共享 Home/PostgreSQL、完整高级内容和 P00/E0–E3 仍保持开放。活动目录仍为 `device.v50`，原生 `author@4` 尚未切为默认 PPT 引擎。
