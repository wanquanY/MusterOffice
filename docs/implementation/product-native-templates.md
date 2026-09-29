# 产品原生模板查询与实例化

2026-09-28，在用户指定的 Musterwork 独立 worktree 继续实现。内核仍只提供模板计算；目录、账号、模板文件、授权、资源传输、草稿及提交归原产品 owner。

后续已实现[原生模板生产入口](product-template-source.md)，与此处消费端共享包校验；自动上传协调与完整模板质量仍继续实施。下文保留本阶段的验证范围。

## 候选接入

Device private 候选继任为 `device.v54 / artifact.presentation.author@7 / Skill v10`。稳定配置仍为 `device.v50 / author@3 / Skill v7`，旧版合同和处理器保持原意。

`template` 接受可选 `template_ref`、JSON pointer 与输出长度限制。未给引用时使用当前调用已固定的产品模板选择；给定目录 ID 与版本后，由既有 `PresentationTemplateSource` 解析精确版本。结果包含包摘要、内核模板摘要、类型化参数、真实例值及有界查询结果。

`instantiate_template` 要求查询得到的精确 `template_ref`（含 SHA-256）、`template_digest`、新文稿 ID 和参数值。产品重新取得同一不可变版本，校验资源闭包，通过公共 SDK 计算，再以原来的 `BeginComputed` 和 Artifact 事务提交独立草稿。新文稿从首版本开始，不继承模板所属产品草稿的历史。

## 包与资源

产品 `presentation-template-package/3` 封装公共 SDK 的 `SnapshotRecord`、`TemplateDefinition` 以及资源 ID 到产品 `ContentRef` 的映射。SDK 记录不由产品重写或自行计算语义摘要。目录选择摘要对应完整包字节；模板摘要由共享模板计算验证。

后端 SQL 目录与签名 RPC 负责授权后传输包和真实资源。SDK 负责文稿与参数语义。接入器验证包大小、摘要、完整资源声明和资源实际字节，将材料登记在原 Invocation 的内容额度与读取集中。普通 SDK 调用不因此需要目录、数据库或持久任务。

包限制为 8 MiB，最多 1,024 项资源，单项 128 MiB、合计 256 MiB。声明之外的资源、缺项、摘要冲突和错误资源身份均拒绝。原生来源 PPTX 是显式资源，不从任意宿主路径读取。旧 HTML 工具不能消费原生包，旧布局检查接口也不能用 HTML layout/slot 字段解释 SDK 参数。

## 验证状态与余项

产品 19 项原生 Agent 回归通过，其中四项覆盖模板查询、独立实例、版本与闭包冲突、取消、数据库重开、原生标题编辑及实际导出。专项使用实际注册的 Tool、原 Runtime/SQLite 提交、真实 SDK/worker 和自有 PPTX；模板源在 Rust 测试中是进程内端口，同一真实包另交给签名后端 RPC 测试。后端 57 项测试与七个子测试通过，使用生产 SQL/授权/gRPC 代码和摘要校验的内存 blob 测试提供方，不宣称已经访问线上存储或真实模型会话。

稳定版和候选版各 70 项 Tool/Skill/发行材料测试通过，候选会话中工具与 Skill 共同准入通过。两个接入生产库按 `--no-deps -D warnings` 严格检查通过，产品 Rust 工作区 all-targets 编译通过。产品基础依赖有两处既有 Clippy 问题，其他测试文件另有十条未使用导入警告，不宣称全产品 lint 无警告。

原生实例导出暴露的标题读取/写回缺口在内核解决，见[原生标题语义](native-titles.md)。真实输出 21 个部件中，只有目标页和标题元数据变化，另 19 个部件字节保留。六份既有 PPTX 与两张预览保持上一阶段的全部字节；独立运行与完整回归产生相同原生实例 PPTX。

固定 SDK 包含 26 个生产库、54 个外部依赖和 567 个文件；此次相对前一接入阶段未增加 registry 依赖。新 worker 为 macOS ARM64 debug 开发构建，42,660,544 字节不代表发行包或桌面打包体积。内核 1,048 项 Rust 测试、相关 33 项原生/WASM/MCP 门禁及 13 组模板完整结果对比通过，详见[阶段证据](../reviews/evidence/2026-09-28-product-native-templates-verification.json)。原始失败记录保留。

模板原生生产和目录导入、页面级引用闭包与组装、高级参数、旧 HTML 历史迁移、真实桌面/模型链路、完整 PPT 能力与替换门禁继续实施。本阶段不代表可将 stable 全量切换。
