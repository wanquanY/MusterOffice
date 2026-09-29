# 上传、独立审查与模板入库的连续链路

2026-09-28，在 Musterwork 独立 `codex/musteroffice-integration` worktree 中，将此前[上传至定义和压力检查](product-native-flow.md)扩展到独立审查、最终验证、目录提交和签名读取。原主工作区没有参与写入、构建或测试，完整一期替换目标仍未完成。

## 连续执行范围

同一次自有两页 PPTX 上传，经原 Python 导入器、冻结策略、协调器和签名服务进入真实 gRPC、PostgreSQL 准入与环境匹配派发。原 Agent Loop 和 Tool 日志调用固定 SDK/worker，完成模板定义、8 个压力案例，以及来源和全部压力结果的 9 份最终检查。

本轮用真实后端 `PresentationReviewAuthorityService` 替换连续测试中缺失的审查端口。该服务使用原签名校验、冻结模型策略、PostgreSQL 锁与不可变审查准入。Rust 原审查协调器经 gRPC 取得签名子任务，再进入原任务队列。独立 worker 使用原 `PresentationReviewLoopPolicy` 执行子任务，产生实际模型请求、输出和任务完成记录。

该输入实际产生 18 个子任务。父任务有 9 次返回 `ReconciliationWaiting`、释放执行位置，再由原调度器恢复，保持原 Run 身份和 Tool 日志。没有手写已完成状态、跳过租约或直接更新 SQL 来恢复父任务。匹配环境的 worker 若先领取审查子任务，也按原审查策略执行，不丢弃已经领取的任务。

最终原提交证明同时核对计算结果、文字容量和独立审查证据，后端完成 `native_verify` 检查点，再由原协调器提交不可变模板版本 1。仅完成定义或压力计算的两个回归用例继续断言目录没有已发布版本。

## 入库后的正式读取

对刚入库的版本，测试通过原生产签名器签发当前 `musterwork.execution-grant.v8`，并生成自有设备凭据，调用真实 `PresentationTemplateSourceService.Resolve`。读取权属、精确版本、能力版本和资产完整性均由原产品实现验证。

`artifact.presentation.author@7` 获取的模板包与目录存储逐字节一致；本样本唯一来源资产是 10,269 字节 PPTX，与上传文件一致。固定引用重读返回相同结果。错误摘要、缺失版本、另一已签名用户、旧 author@3，以及签名后伪造调用者身份，分别被原接口拒绝。

这一步证明正式签名读取和资产传递。后续仍需将这个新入库版本直接交给下游真实 Agent，连续执行模板实例化、编辑、导出和产品查看；此前单独通过的实例化测试不能自动替代该连续验收。

## 验证与边界

3 项跨进程测试和 74 项相关 Python 回归通过，最终没有忽略用例。留存并独立核对 197 个文件条目，其中完整链路 105 个，定义/压力回归分别 14/78 个。检查包含 SHA-256、长度、实际 ZIP CRC/XML、PNG 尺寸、9 份检查和 18 份审查的页面覆盖；详见[本阶段证据](../reviews/evidence/2026-09-28-product-native-catalog-flow-verification.json)。运行耗时只记录为测试诊断，不作性能结论。

模型路由、工具选择与视觉判定使用确定性测试输入，账户目录和内存对象提供方也有明确的测试边界。审查子任务、签名、数据库、调度、计算和完成证明为真实实现。视觉判定通过不等于视觉质量验收；SDK 的 `quality_proven` 在这 9 份材料中仍为 `false`。正向流程使用既有独立扩宽文本框的自有输入重新计算全部阶段，没有修改旧完成记录或忽略原样本的溢出。

Python 静态检查及修改文件 Rust 格式检查通过。整个历史 `sqlite_agent_loop` 目标的严格 Clippy 仍报告 994 项既存诊断；新增 `cloud.rs` 和 `cloud/reviews.rs` 没有诊断，复用的旧 provider 仍有既存 unwrap 问题。没有添加 lint 豁免，也不宣称整个测试目标的静态检查通过。

本轮新增依赖只是测试工程引用已有 `backend-capabilities` workspace crate，没有新增外部组件。SDK/worker 字节未变，候选仍为 `device.v54 / author@7 / Skill v10`，正式 stable 未切换。完整 Cloud 二进制、下游 Agent 连续创作、Viewer/Player、历史迁移、完整高级内容、Office/WPS、Native/WASM、质量、性能与发行体积门禁继续推进。

账户、签名授权、任务、存储和目录始终位于接入产品侧。MusterOffice 继续只提供文稿计算与薄接入，没有增加业务权限、持久存储或面向人的页面。
