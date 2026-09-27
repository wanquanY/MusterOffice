# 计算合同与直接 SDK

2026-09-27 · 按 [ADR 0007](../decisions/0007-kernel-only-integration-boundary.md)拆分实际计算与历史宿主适配。本文记录已实现接口及验证范围；完整一期与替换验收仍以[实施目标](kernel-boundary-roadmap.md)为准。

## 同一计算实现

新增 `mo-presentation-operations`，承接原服务中的创建、导入、原子编辑、导出计算、资源输入及诊断。其输入是文稿/版本前提、操作和调用方提供的不可变资源，不包含账号、权限、持久 Job、数据库或业务提交。

新请求版本为 `musteroffice.computation/1-draft`，字段仅有 `contractVersion`、`requestId`、`profileId`、`action`。严格解码拒绝旧版本、重复字段和额外宿主字段；请求字节上限仍为 32 MiB。`requestId` 标识计算，不提供跨进程幂等存储。版本前提仍在原子编辑中验证。

历史 `mo-operation-service` 保留 `musteroffice.operations/1-draft`、原摘要域、权限检查、持久任务和旧收据语义，通过薄适配调用新计算库。未复制第二套文稿算法，也未把旧请求静默解释成新协议。旧持久宿主和 MCP 尚未从其自身运行链路退出，不能据此声称全部 Agent 接入边界已经完成。

纯计算结果不再额外持有完整快照 JSON 副本：使用有界计数写入器检查序列化长度及取消。历史宿主确需旧 JSON 时在兼容层生成。这项结构变化减少了一份保留数据，但没有据此宣称实测内存、延迟或完整包体积收益。

## SDK 调用与所有权

`mo-embedded-sdk` 新增 `Presentation`、`Inputs` 和 `ExportOptions`：

- `Presentation::create/import/from_snapshot` 创建进程内文稿；`edit` 原子更新该值，失败或取消时保留原值；`snapshot/document/into_snapshot` 供调用方查询或取走数据。
- `Inputs` 借用字节切片或 `ReaderAt`，维护有界资源元数据；不复制全部输入为自己的内容库。导入/导出继续核对实际长度、摘要和文件结构。
- `Presentation::export` 使用调用方配置的 `NativeExporter`，返回实际 PPTX、预览和诊断候选。调用方读取输出、保存到自己的位置，再释放候选；不要求初始化 JobStore、账号或业务提交器。

原生计算隔离保持不变。宿主明确提供 worker、其摘要、临时空间与期限；执行协议升级为 `musteroffice.native-export/2-draft`，旧私有协议被明确拒绝。SDK 与 worker 必须成对固定版本。暂存只服务本次计算，宿主决定启动/维护时的崩溃恢复调用和持久资源策略。

这里的直接 SDK 是已有计算能力的易用接口，不代表完整高级内容已实现，也不替代宿主的文件、网络、权限、业务任务或 UI。

## WASM 与发行依赖

主 `mo-wasm` 改用纯计算库，公开 `computation_schema_json`；不再导出旧宿主 `operation_schema_json`。历史入口移至独立开发兼容模块 `tools/mo-host-compat-wasm`，便于旧消费者显式迁移，不捆绑进正常计算 WASM。

四份新计算 Schema 与 TS 类型从相同 Rust 定义生成：请求、错误、修改收据和导出收据。旧宿主 Schema 保留原版本，不能替换为语义不同的新版本。

SDK 构建器明确拒绝 `mo-operation-service`、`mo-standard-host`、SQLite 和 Tokio 等进入原生 SDK 生产闭包。worker 的绘制/字体依赖单独计入完整交付。独立消费者从打包材料编译并读取最终文件，才算该入口的实际消费证据；源码归档大小不等于安装包或运行体积。

## 验证状态

当前专项验证已完成 SDK 创建/编辑/真实导出/重新导入、失败与取消、原生执行恢复和旧 worker 协议拒绝。原生专项为 12 项通过，另一个子进程辅助测试由父测试调用，不算独立正常用例。

固定两页样本的 12 个交付文件中，PPTX、两份预览及其余内容/字体/素材共 10 个文件保持历史字节。仅交付上下文中的 worker 摘要、依赖它的设置摘要，以及质量文件中的上下文引用变化；已逐字段核对。此比较验证该样本的内容保持，不能代替视觉语料、Office/WPS 或高级内容验收。

完整[阶段证据](../reviews/evidence/2026-09-27-computation-boundary-verification.json)已记录 20 个检查步骤通过：全仓 Rust 919 项通过、19 项忽略，随后显式运行真实 renderer 4 项、历史宿主导出 9 项和 Native exporter 12 项；这些专项与普通回归有重叠，不能相加为独立测试总数。历史 MCP 的 18 项测试通过，未运行该轮独立 MCP 客户端协议探针。

主 Native/WASM 文稿计算 27 对、4 份新计算 Schema 及独立兼容 WASM 的 10 份旧合同通过核对。113 份原 Schema 和 117 份原生成类型逐字节不变，新增各 4 份计算合同/类型。外部 SDK 示例仅在全量检查之后移除一个无用导入；最终独立工程已严格 Clippy 并重新运行，生产计算源码保持全量检查时的字节。

最终复查补回历史适配器生成旧 JSON/收据之后的取消检查，避免额外封装工作结束后返回已取消结果。受影响服务/宿主 75 项通过、13 项忽略，9 项真实旧宿主导出另行通过，严格检查通过；详见[最终取消行为复验](../reviews/evidence/2026-09-27-computation-boundary-final-verification.json)。这次补充只修改两个历史适配文件，纯计算/SDK/worker/Schema 字节不变；未把之前的全仓结果称为补充后的全仓重跑。

最终 SDK 包含 23 个内部生产库、54 个已锁定 registry 包；打包/验证器 9 项测试通过。外部工程从该包编译，导出两页、12 个资产；逐文件、收据及最终存储检查均与同版本直接调用一致，使用后包内容不变。独立解析核对 15 个原生对象、8 个文字 run 和 10 个 XSD 部件。该数字是样本结构证据，完整视觉、原生编辑和目标应用质量声明仍保持 `not_proven`。

上述实际执行使用 macOS arm64 开发构建。没有新增运行依赖版本，没有在本轮改动 Musterwork 源码或迁移产品数据库，也没有宣称发行体积、延迟或内存的实测收益。薄 MCP/CLI、Musterwork 新合同适配、完整高层播放与高级内容、I01–I20 和完整替换门禁仍未完成。
