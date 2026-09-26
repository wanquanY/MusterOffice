# 统一宿主分发与运行能力发现

状态：P07 的标准操作入口与发现功能已实现并完成本阶段验证，源码和产物由[阶段证据](../reviews/evidence/2026-09-26-operation-discovery-verification.json)绑定。完整 MCP/Skill/Plugin/SDK、浏览器任务 owner 和 Musterwork 嵌入适配尚未完成。

需求追踪：本阶段属于 P00/P07，为[接入验收](../design/agent-interfaces.md)的 I08（跨入口同义）、I10（权限）、I11（单 owner）与 I16（合同/能力一致）提供底层实现和部分证据；尚未达到这些项目各自的完整产品验收范围。

## 同一个分发入口

此前 `StandardHost` 同时负责 SQLite 状态和控制请求分发，其他宿主需要复写 JSON/参数校验、动作选择、同步与异步偏好和错误封装。现在 `mo-operation-service` 提供 `OperationHost`、`dispatch_host` 与 `dispatch_host_json`，计算层仍不持有数据库、文件路径、网络、进程或隐式身份。

标准宿主实现该接口并将其委托给已有资源/任务/文档方法。原来的 `StandardHost::dispatch/dispatch_json` 入口保留，内部调用共享实现；没有复制另一个队列、提交事务或导出算法。创建/修改/导出的权限与 profile 检查和能力描述共用 `ServiceOperation`，控制入口在调用宿主前先验证授权，宿主直接方法也继续检查权限。

`OperationHost` 是同步的执行 owner 接口，适合专用宿主工作线程。它包含文档/任务方法与独立二进制上传、授权范围读取；`AuthorizedAsset` 绑定描述和实际 `ReaderAt`。它不是完整浏览器异步存储桥，也不强制成熟产品另建 SQLite owner。未来嵌入适配将现有资源、任务、取消、执行和事务能力映射至相同操作语义；Musterwork 的 Artifact/Resource/Ledger 提交仍由产品 Runtime 持有。

## 当前可调用的发现接口

本地 `mo-host` 的 NDJSON 接收：

```json
{"operation":"capabilities"}
{"operation":"getSchema","id":"host-request"}
```

`capabilities` 返回当前 executor 摘要、可用动作、权限要求、profile、控制/二进制通道、版本语义、输出模式、有效限额和已配置 renderer 身份。内容不包含 principal/scope、数据库路径、文件路径或用户文稿。调用者不具备权限的动作从目录中排除；这是发现时的视图，后续调用仍重新授权。

导出动作已实现但未配置 renderer 时，目录明确返回 unavailable 及 `previewRendererNotConfigured`。配置真实 Native renderer 后才标记 available，每次执行仍校验其身份与实际调用结果。可调用不等于已通过视觉/播放/外部应用验收：目录固定声明 `completeFeatureCatalogue=false`、`fullPresentationAcceptance=false`，不将操作目录当成完整原生格式能力注册表。

目录里的 `presentations.create/apply/export` 是领域动作标识；当前 NDJSON 使用 `submit` 包装 `OperationRequest`，并非直接把领域名称放入 `operation` 字段。具体参数应读取生成的 host/operation Schema。独立资源字节走 binary port 或已有 CLI `append/read-asset`，不作为 JSON/base64 混入控制请求。

`queuedExecution=explicitRun` 表示当前标准宿主的 job 模式先接受排队，由操作员执行 `run`；没有后台调度器。创建针对新文稿，apply 使用当前 head 的比较提交，export 指定不可变历史版本，read 选择指定版本或当前 head。这些差异在目录中分别声明。

## Schema 的唯一来源与 Native/WASM

`getSchema` 支持严格枚举的十份合同：host 请求/响应、操作请求/任务、能力目录、Schema 文档、作者文档、上传请求/状态和资产描述。未知 ID 或额外字段被拒绝，Schema ID 不是 URL 或文件路径，不触发外部加载。

Schema 从同一 Rust 类型生成，`mo-common::runtime_schema` 统一附加与磁盘生成器一致的 `$id`。返回 `id/digest/schema`；摘要使用 `musteroffice.operation-schema/1` 域绑定 Schema ID 与规范 JSON，不是漂亮打印后的文件摘要。首轮严格相等测试发现运行查询遗漏了生成器附加的 `$id`；修复共享身份逻辑后保留完整相等断言。

`mo-wasm::operation_schema_json` 接受 JSON SchemaId 字符串，例如 `"host-request"`，调用相同纯服务实现，返回同一 Schema 文档。它供浏览器宿主在配置 owner bridge 前发现合同，不创建浏览器任务，不授予资源权限，也不代表完整 WASM 导出已实现。身份 `CallContext` 仍没有 Deserialize；权限枚举能出现在只读目录中，不使请求获得权限。

## 有效限额共用

标准宿主的 `export_limits` 同时用于真实导出和能力描述：单资产/模型/字体、总输出字节与产物数量取计算默认预算和宿主配置的有效交集。此前宿主配置较小单资产上限时，输出存储会因模型/字体默认预留额高于该上限而提前拒绝，即便实际输入很小；共享预算在预留前限制模型和字体的最大声明，保持真实字节检查。

限额表示准入容量，不是当前剩余配额或 RSS 保证。所有输入/输出仍计入实际资源 owner 的事务预留；图片、字体、worker 整包/像素、数据库/WAL 和进程内存的完整预算继续单独实现，不能从这个目录推导整机内存上限。

## 验证和剩余工作

定向测试使用真实 SQLite owner，检查权限交集、未配置 renderer、只读发现不创建任务、严格拒绝授权字段注入、全部 Schema 与生成文件一致、通用宿主端口的资源字节及重启读取。新增显式真实 worker 用例把宿主单资产限额设为 96 KiB，同时核对目录的有效模型/字体限额和实际完整导出。

本轮 826 项全仓 Rust 测试、格式、严格 Clippy 和 109 份生成 Schema/TS 检查通过；其中五项普通测试为本阶段新增。十份可查询 Schema 在实际 release CLI、新 WASM 和磁盘生成文件中完全一致，含规范摘要；WASM 的两个非法输入也被明确拒绝。两份旧 Schema 变化、两份新增，其余 105 份旧 Schema 保持不变。纯操作服务的 WASM 编译与依赖图没有引入 SQLite 或原生进程组件。

两项私有交付及八项标准宿主导出集成另用真实 worker 显式执行。第一次并行导出测试有一项因临时目录的时间戳重名而尚未进入业务验证就失败；六个宿主测试文件现共用独占目录分配，以进程内原子序号保证并行唯一性，遇到已有路径不覆盖。保留并行执行和首轮失败日志，修复后重跑全部 50 项受影响普通宿主测试、严格 Clippy、格式及八项真实导出集成，均通过；826 项全仓结果发生在这次纯测试夹具修正之前。

独立验证执行 37 次实际 release 调用，核对带/不带 renderer 的能力差异、十份 Schema，以及此前 v4 库的任务和全部 12 个公开资产；原任务、全部表内容、资产字节及原库文件均保持不变。另复用封存的导出验证程序执行 31 次 release 调用和两份 v3→v4 副本迁移，生成两页 12 个公开文件。独立检查通过引用/摘要、PNG 像素、10 个相关部件 XSD、15 个原生对象和 8 段文字。输入仍是自有合成字体及 `A A` 文字，不能据此宣称商业文稿保真。

当前 CLI/WASM 的 72 次文件/编辑配对调用和两页实际绘制均保持历史结果，页面元数据及所有像素一致。此检查与 Schema 双端查询分别执行，不把 Schema 一致视为绘制或完整 WASM 导出 owner 的证据。

本轮验证程序位于 `tools/verification/operation-discovery-*.py`，产物位于 `.codex-work/operation-discovery/`，最终实际发现/旧数据/导出目录是 `reference-1/`。工作区与文件/页面重放复用按摘要验证的已封存程序，在独立目录中执行；早期 Schema 身份和测试目录失败记录保留。证据记录各次执行及实际源码/产物身份，不覆盖上一阶段结论。

## 模块体积成本

同一锁定工具链和 release 配置下，Rust WASM 模块从 7,791,982 字节增至 8,765,461 字节，增加 973,479 字节；gzip level 9、mtime=0 后从 2,270,761 增至 2,423,678 字节，增加 152,917 字节。运行 Schema 生成逻辑是本次真实链接成本，后续应评估同源预生成、按需加载与发行 feature 划分，不能用删除合同或失去查询能力换取数字下降。

这只统计 Rust WASM 模块，不包括 JS、绘图/塑形组件、字体、媒体或产品运行时，不是完整内核和桌面安装包测量，也未新增 FPS 或产品性能结论。

后续继续实现 MCP 传输及工具/资源/任务映射、能力与版本协商、异步浏览器/嵌入宿主、SDK/Viewer 和 Musterwork 全消费者。完整高级对象、动画/转场、媒体、Office/WPS 实际往返及最终性能/发行门槛仍属于原目标。
