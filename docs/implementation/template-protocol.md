# 模板共享协议与产品计算来源

2026-09-28，在[模板纯计算](template-computation.md)基础上，把模板查询与实例化纳入既有 `OperationRequest`。模板目录与文件传输不进入内核，SDK/CLI/WASM/MCP 共用相同的来源、参数、结果和错误语义。

模板专项、33 项原生/WASM/MCP 门禁和产品回归已通过。它不代表完整模板、Office/WPS、视觉质量或 Musterwork 替换验收完成。

后续已接通[产品目录读取与 Agent 模板实例化](product-native-templates.md)，并修复[原生标题读取与写回](native-titles.md)。下文保留本阶段的历史验证范围。

## 共享调用

`Invocation.snapshot` 是调用方持有的不可变来源。两种动作均使用 `presentations-author-model-v01-draft`：

| 动作 | 输入 | 结果 |
| --- | --- | --- |
| `describeTemplate` | 来源快照、`TemplateDefinition` | `describedTemplate.description`，含经过校验的模板摘要、定义和真实参数例值 |
| `instantiateTemplate` | 同一来源与定义、查询取得的摘要、新文档 ID、类型化参数 | `mutated.snapshot` 与收据，形成独立文稿的首版本 |

查询会恢复来源语义并验证全部参数目标，不只是计算 JSON 摘要。Agent 不必自行推算模板摘要或借助另一条私有工具。查询不产生新文稿或修改收据。无效或过期来源、重复/不存在目标和错误类型仍由同一模板引擎拒绝。

实例化的 `MutationReceipt.template` 保存模板来源、作用域映射和参数计算证据；外层 `transaction` 为空，`MutationCandidate` 没有新文稿的父版本。模板内部 `bindingTransaction` 是来源副本上的纯计算，不能成为新文稿的历史父事务。下一次编辑以输出快照的 revision 为前提。

普通创建/编辑收据省略新的可选字段，保持原有序列化。持久旧宿主只解码其原有 Import/Create/Apply/Export 动作，新模板动作在请求解码、程序化路由和计算适配处拒绝，不扩建其业务服务。

## 各入口

- SDK：`execute(invocation, inputs, exporter, cancelled)`；模板调用不需要 exporter 或资源字节。`Presentation` 的直接模板方法仍可使用。
- CLI：`mo-cli compute <invocation.json> <inputs.json> <temporary-directory> <new-output-directory>`；模板调用的文件输入 manifest 为 `[]`。
- WASM：`compute_document(invocationJson)` 返回共用计算收据，失败抛出序列化的 `Failure`。这是同步计算入口；浏览器的取消和调度仍由宿主 Worker 实现。
- MCP：既有 `mo_presentations_compute` 接收调用方文件；`mo_schema` 提供最新 `computation-invocation`、`computation-receipt` 与模板 Schema。工具数保持三个，不另建模板目录或存储服务。

所有结果明确只表示计算完成。输入快照内的资源声明不是资源授权或字节验证；真正渲染、导出时仍须提供并核对完整资源。默认来源、请求与结果额度、取消边界与模板自身限制共同生效。

## Musterwork 接入

只在用户指定的独立产品 worktree 工作。产品准备端口以 `OfficeMutationSource` 区分 `New`、`DraftRevision` 和 `TemplateSnapshot`。只有 `DraftRevision` 递增原草稿历史；模板快照经原内容端口读取并记录，输出草稿从第一版开始。

计算语法直接使用公共 SDK；没有产品自行复制模型、修改模板 JSON 或将原生模板伪装成 Create。读取集、资源闭包、取消、输出预留、checkpoint 与提交继续使用原 Runtime 和 SQLite owner。产品已在独立 worktree 固定新 SDK 和匹配 worker；默认 stable 目录保持原样。产品目录选择尚须经其原有 `PresentationTemplateSource` 接入，不能以该内部计算端口代表 Agent 选择模板链路完成。

## 验证组织

`tools/mo-mcp/template_check.py` 用真实 stdio 进程执行两个既有 MCP 协议档案的查询、两个独立实例、编辑与导出；随后重新导入原生 PPTX，再查询、实例化与写回。核对来源不变、结果 Schema、错误不留输出、PNG 差异、ZIP/XML 和未修改部件字节。

`tools/verification/template-protocol-parity.mjs` 消费这些真实请求和回执，再执行原生 CLI 与 WASM，比较完整成功结果及失败诊断。该流程进入当前验证 driver；Rust 另测来源/参数冲突、每个已观测取消边界、大小限制与旧接口隔离。

内核全仓 Rust 测试通过 1,041 项，普通运行另有 25 项忽略测试；适用的真实 worker 专项在原生门禁中显式执行，并不将全部忽略项计为通过。全仓严格 Clippy、Schema、TS 和客户端检查通过。完整 driver 首次因测试脚本误读原生段落结构在模板项失败，生产代码未变化；修正后重跑相关原生/WASM/MCP 全组，33 项通过。跨入口测试曾误从 stdout 读取 CLI 错误，按既有 stderr 合同修正。原始失败均保留。

产品专项包含 10 项计算/提交测试（其中 3 项模板测试）、46 项 Artifact 库测试和 15 项已有原生 Agent 测试。相关生产库严格 Clippy 通过，整个产品 Rust 工作区 all-targets 编译通过；另有两个其他测试文件的 10 条未使用导入警告，不称全产品 lint 无警告。初次 Agent 回归的一个原始设置测试携带旧 renderer 摘要而被拒绝；仅在另存的自有输入中换用真实能力发现返回的当前 renderer，完整 15 项重跑通过，没有放宽版本校验。

六份真实 PPTX 与两张重新打开后的预览均与上一产品阶段逐字节相同。所有 PPTX 的 CRC、21 个部件和 20 个 XML/关系文件均核对；原生编辑仍只改变目标页 XML。固定 SDK 包含 26 个生产库、54 个原有外部依赖和 564 个文件；实际归档及产品安装清单验证通过。当前 macOS ARM64 worker 是开发构建，其 42,346,384 字节不代表正式发行或桌面包体积。

命令、源码/材料摘要、结果与保留失败见[阶段证据](../reviews/evidence/2026-09-28-template-protocol-verification.json)。未使用真实用户文稿；文字计算中的 Unicode 测试也不等于中文字体渲染验收。

仍待页面级引用闭包与组装、高级参数、容量/布局压力验证，以及产品模板目录、Agent 调用、旧 HTML 历史迁移和完整一期替换门禁。
