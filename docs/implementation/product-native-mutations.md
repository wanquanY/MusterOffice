# Musterwork 真实内核创建、编辑与事务证据

2026-09-27：接续[原生草稿事务](product-native-drafts.md)，将真实 SDK 的创建与原子编辑接到产品内容存储和既有提交 owner。以下 `MW:` 为产品相对路径；本仓库不迁入产品私有源码、数据库或测试二进制。此阶段仍没有切换用户工具和默认 PPT 路由。

## 实际计算与内容

`MW:apps/agent-runtime/crates/runtime/artifact/src/presentation_office_mutation.rs` 定义产品计算端口。输入包含 Invocation、草稿身份、已解析的精确请求 ContentRef、可选基准修订和完整结果资源映射。领域与 Runtime 不导入内核具体模型语法；`infrastructure/musteroffice` 使用已固定 SDK 解析 `OperationRequest`，并要求内核 requestId 等于此次 Invocation UUID。

适配器只接受无基准的 Create 或有基准的 Apply。通过产品已授权、完整校验的范围读取器读取真实 JSON；拒绝重复键和未知字段。基准必须是实际 `SnapshotRecord`，文稿身份、内核修订、语义摘要和页面顺序均与持久元数据一致。随后调用 SDK `compute_mutation`，由原内核恢复并校验语义摘要、执行整批原子编辑，返回真实模型和含 ChangeSet 的内核收据。

基准和结果各自必须具有完整资源映射，ResourceId、摘要和 MIME 与模型逐项匹配；实际资源字节在准备阶段通过原内容端口校验。排序的模型与绑定采用线性配对，避免逐项扫描全表。产品资料、请求和模型的相同 ContentRef 复用同一读取器；资源别名仍计入模型绑定预算。

模型、收据和产品修订的全部静态检查在写入前完成。输出身份由 Invocation、用途、MIME 和实际摘要确定；重试得到相同精确引用。写回后重新打开实际产品存储进行字节验证，再返回候选。该候选不提前写草稿、Artifact、Ledger 或成功状态。

## 同一提交事务

新增 `BeginComputed` / `AdvanceComputed` 进入原 `CommitPreparedPresentationTool`，不增加任务数据库或调度器。`presentation-authoring-checkpoint/3` 保存实际计算的请求引用、逻辑请求摘要、收据引用和有序精确读取集。文档持久状态继续使用 `/2`；历史 checkpoint `/1`、原生元数据 checkpoint `/2` 的语义保持不变。

过期版本先按原 CAS 规则返回冲突，再校验对应 head 的计算读取集，避免把正常的并发版本冲突误报为无效命令。

新恢复封套严格解码并重新编码核对原字节，拒绝未知/重复字段、缺少状态、错误版本和非规范表示。读取集必须包含精确请求、排序唯一，不能把输出收据列作输入。恢复数据不是外部 Agent 提交的成功证明；生产工具必须通过实际准备器生成候选，新工具授权和组合入口继续实施。

SQLite 在原 Attempt fence 和写事务内检查当前 head 的模型及全部资源是否存在于计算读取集，再沿原内容保留机制重新授权请求、旧模型、被删除的输入资源、结果和收据。即使某个资源已从新模型移除，它仍是本次计算的输入，不能因为不在结果中而跳过提交复查。原草稿、修订、内容保留、Ledger、Tool completion 和幂等收据共用一个事务。

## 预算与取消边界

Runtime 注入共享计算 Semaphore 和材料预算。没有可用计算名额时返回 ResourceBusy，不新增局部队列。阻塞计算持有名额直到实际工作结束；取消接收 Future 不提前释放仍在计算的名额。每个阶段检查取消，JSON 范围读取按 64 KiB 检查取消；存储 I/O 的诊断类别与内核失败 detail 保留。

计算名额和材料上限不等于全 Invocation RSS、磁盘、文件句柄配额或崩溃后的候选回收。模型 JSON 仍有有界物化和候选序列化副本，没有宣称峰值内存验收。写入期间取消或进程退出可能留下私有预写内容，需要后续原 Runtime 的 Invocation 保留/回收管理；不能用自行删除确定性输出的方法破坏并发重试。当前不开放生产路由。

## 依赖与验证

本次在既有产品基础上增加内部 Artifact/Runtime 依赖和测试消费关系，将已有 serde_json 用于生产适配器。registry 包的版本与 checksum 与 SDK 桥接阶段一致，未选择新的外部组件；固定 SDK、计算核心与 worker 未修改。全量发行许可核查仍未完成。

## 验证与后续

最终原生专项 11 项（含六项真实计算测试）、相关领域/Runtime/桥接库 82 项通过，三个生产库严格 Clippy 通过，产品全工作区 all-targets 编译通过但保留既有测试未使用导入警告。本轮没有运行完整产品测试套件，也没有重跑需显式 worker 材料的导出集成；该项在普通库测试中保持忽略。新增测试使用实际 SDK、SQLite 内容和现有 Run/Attempt/Tool journal，创建文稿后真实改标题、改页面文字并删除资源，检查新旧模型、内核修订与收据；还覆盖恢复、重试、过期版本、撤销计算输入、失败回滚、非法 JSON/资源闭包、伪造语义摘要和取消后的计算名额。

扩大创作回归曾为 49 项通过、1 项失败、3 项既有外部环境测试跳过。唯一失败是新计算提交对过期版本先报告无效读取集；修正为先检查原 CAS 后，在最终 11 项原生专项中通过。未重复运行未受该顺序调整影响的旧创作测试，不把此前整轮记录改称全通过。早期还修正了测试漏传 Attempt、夹具对内容去重的错误假设，以及实际提交 owner 漏掉新创建分支的问题；失败和最终日志均保留在[阶段证据](../reviews/evidence/2026-09-27-product-native-mutations-verification.json)中。

这组测试复用历史测试工具身份以验证既有物理事务，不代表新的生产工具路由或共享 Home 授权已实现。默认 PPT 路由、新 Agent 工具、原生导出 candidate/Artifact 提交、共享 Home/PostgreSQL、完整任务预算、Viewer/Player、历史迁移、高级内容、Office/WPS 往返和 E0–E3 继续开放。
