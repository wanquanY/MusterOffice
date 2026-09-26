# Musterwork 原生草稿与同库提交

2026-09-27：接续[SDK 构建与内容桥接](product-sdk-bridge.md)，在产品现有 authoring owner 中实现原生草稿的创建、修订、放弃与恢复。以下 `MW:` 均为产品相对路径；本仓库不迁入产品私有源码、数据库或测试可执行文件。

## 合同与职责

新增 `presentation-draft-state/2` 保存原生模型的精确 ContentRef、内核 documentId、内核 revision digest、semantic digest、显式资源映射和页面顺序。产品 revision ordinal 与内核 revision digest 独立；修订同时绑定上一产品 pin 和上一内核 revision，不能用另一个 documentId 替换当前草稿的谱系。

原生模型 ContentRef 当前指向 SDK `SnapshotRecord` 的 JSON bytes，包含 document、revision、semanticDigest。元数据校验不等于执行 `Snapshot::restore`、验证实际资源闭包或证明渲染质量；后续生产准备器必须从已授权且已验证的实际内容恢复内核快照，不能让 Agent 提交这些字段便获得成功声明。

资源按内核 ResourceId 排序，映射到产品不可变 ContentRef；相同内容的多个 ResourceId 可复用同一引用，但元数据必须完全相同，且别名仍计入准入预算。此处没有沿用旧版仅图片/字体的 MIME 白名单，类型支持和解码仍由内核与所选操作负责，任意资源 MIME 不代表可执行或已支持。

当前原生持久元数据档案与已实现 Artifact /4 对齐：模型最多 32 MiB、单项资源及来源 PPTX 最多 128 MiB、资源绑定最多 1024 项、资源累计最多 512 MiB、页面身份最多 256 个。草稿可以暂时没有页面。这些是状态与材料边界，不是所有操作均可导出这些上限的承诺，更不是 Invocation 并发、磁盘或渲染内存预算；创作、来源预览和具体导出档案仍需分别准入。

`presentation-authoring-checkpoint/2` 是独立原生恢复封套，使用 office-draft/office-revision 状态种类。原生状态编码要求确定的字段、数字、UUID、摘要与字节顺序，拒绝额外字段、重复字段、错误版本和不完整字段；恢复后重新编码必须与原 bytes 相同。旧 `/1` 的状态、候选、review、checkpoint 与编码入口保持原合同，不将 HTML 状态改写为原生状态。

## 一个事务 owner

`MW:apps/agent-runtime/crates/domains/artifact/src/presentation_authoring.rs` 的公共草稿生命周期按 revision 类型复用，默认类型仍为旧 HTML revision。新类型定义自己的模型、资源及内核谱系校验；没有复制一套草稿关闭、产品 pin 或版本递增逻辑。

`MW:apps/agent-runtime/crates/runtime/artifact/src/presentation_office_authoring.rs` 的原生操作进入既有 `CommitPreparedPresentationTool`。SQLite 复用同一个 live Attempt 锁、物理写事务、Ledger append、Tool completion、版本化 receipt 与内容保留机制。现有 presentation_drafts、presentation_draft_revisions、presentation_content_refs 和 presentation_commit_commands 关系承载两代状态；本次没有数据库迁移、新队列或新任务 owner。

创建、修订和关闭的 SQL 写入原语由两种 revision 共用。读取侧先按明确 schema 解码，再核对租户、会话、草稿、Artifact、创建 Run、产品 pin、状态、时间和基准 Artifact 版本等冗余列。旧读取入口遇到原生状态会返回格式不匹配，原生读取入口也不会把旧状态自动转换。修订不能跨引擎；同 Invocation 重试保留原收据，即使草稿后来已经前进或关闭。

内容保留在同一事务里重新检查精确引用；取消、过期执行权、撤销资源或 receipt 写入失败不能留下草稿/修订/保留引用/成功 Tool 的局部提交。数据库原有的草稿转换与不可变历史约束保持有效。

## 验证范围

本阶段通过实际产品 SQLite store、Run/Attempt 和 Tool journal 测试事务，不用另建内存模拟 owner。原生创建使用产品已有自有两页快照与图片；修订测试保留同一快照，验证宿主序号和内核谱系的持久行为，不能计为内核执行真实内容编辑。

最终 SQLite 创作回归 44 项通过（含五项原生测试），三项原有浏览器/旧原生外部环境测试跳过；领域层 25 项、Runtime Artifact 层 52 项通过。领域、Runtime Artifact、SQLite 三个生产库的严格 Clippy 通过，全工作区 all-targets 编译通过但保留原有测试未使用导入警告。格式检查通过，未运行完整产品测试套件。测试及构建结果见[阶段证据](../reviews/evidence/2026-09-27-product-native-drafts-verification.json)。第一次损坏测试试图只改 document_hash，被现有数据库 trigger 拒绝。随后保留这个拒绝断言，并通过合法的状态列转换配合未更新的状态 JSON 构造不一致，验证读取与提交均拒绝；没有删除 trigger 或放宽生产约束。首次测试命令在产品根目录找不到 Cargo.toml，已在真实 Cargo 工作区重跑，失败日志保留。严格 lint 另发现共享 SQL helper 的一个多余借用，修正后重跑。

## 尚未完成

本阶段只接通 SQLite 原生草稿事务。新 Agent tool successor、模型实际字节准备/编辑执行、原生导出 candidate 与 Artifact 原子提交、预算和崩溃暂存管理、共享 Home 授权及 PostgreSQL 原生写入仍在实施。旧共享 HTML 工具拒绝原生 operation；云端旧提交入口也不会把新状态按旧格式落盘，尚未宣称原生云端支持。

默认产品 PPT 路由没有切换。Viewer/Player、历史迁移、全部高级能力、Office/WPS 编辑往返、跨平台运行、完整安装包和性能验收继续开放。下一步将真实内核准备结果绑定到这些原生草稿与导出候选，再进入同一事务的 Artifact/Ledger/成功回执提交；本阶段不是替换验收通过。
