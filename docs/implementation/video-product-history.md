# 产品共享撤销与重做

2026-10-07。产品提交 `60dccdee30b011fd2726d3346c31318dfb2b77be`；Office SDK、WASM、worker、PPT 及稳定标签保持原固定版本。本增量修复产品历史导航，不增加内核业务存储。

## 责任与行为

MusterOffice 持有工程语义、条件逆操作和依赖冲突。产品此前每次将当前 head 作为撤销目标，连续撤销会来回切换，重开面板又丢失重做。现在每个已提交版本保存恒定大小的 undo/redo/next/inverse_of 指针，UI 和 Agent 读取同一个已提交历史位置。三次编辑 A/B/C 可以连续撤销至 B/A/初始，再连续重做 A/B/C；每步仍由 SDK 在当前工程上执行条件逆操作。

原生 revision envelope `/2` 将历史纳入候选指纹及原 SQLite/Agent journal 原子事务，写入时按实际父版本重新核对。浏览器 IDB schema 2 使用独立小节点 store，在原 strict 事务中保存。旧快照保留；旧记录没有操作意图，显式作为 legacy 导航边界，不通过相同内容摘要猜测历史。旧 `/1` canonical bytes 与指纹不变，不声明旧二进制可读取新格式。

封闭工具输出以不可变后继 Device v71、`artifact.video.author@3`、`musterwork.video.tool-result/2` 发布本地源码合同。Skill v54 的视频技能 @3 指导 Agent 从 inspect 的历史位置撤销/重做。@1/@2 输出保持原合同；原身份、技能 lineage、Cloud f52 和 PPT 能力不变。没有向生产发布。

## 证据与限制

- 7 项产品命令、6 项执行器、1 项 SDK 持久化、11 项 SQLite 检查通过，包括重新打开数据库、旧回执恢复、分支、冲突和伪造/删除历史拒绝。
- 6 项注册 Agent 实际 SDK/journal 检查通过，人工和 Agent 交替提交七个共享版本；需实际 worker 的单独探测项未计入本轮。
- 41 项共享 UI、10 项 Browser Host、4 项 desktop adapter、7 项消息展示检查通过；新旧工具/技能选择、生产类型、Clippy 和边界检查通过。
- Chrome 154 实际 Worker、固定 WASM、strict IDB、OPFS：连续撤销后刷新，再连续重做通过。独立生产 Vue 交互检查使用明确的内存 Host，不冒充完整登录页面。

产品详细设计 `docs/architecture/video-history-navigation.md`，本地证据 `.codex-work/video-history-report.json`。原 210 个 WIP 文件的字节、完整 tracked diff 与展开状态保持。本增量不代替完整会话页面、物理音画同步、导出或真实模型计费验收；完整目标继续执行。
