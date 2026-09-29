# 产品原生模板生产入口

2026-09-28，在已接通[原生模板查询与实例化](product-native-templates.md)之后，Musterwork 隔离工作区新增原生来源导入、定义校验和模板包生成入口。计算复用产品已固定的公共嵌入 SDK；本阶段没有增加二进制组件、registry 依赖或内核产品职责。

后续已实现[冻结环境与真实来源渲染](product-template-render.md)。下文保留本阶段事实；自动上传维护和目录提交仍继续实施。

## 实现与边界

产品 `crates/infrastructure/musteroffice/src/template.rs` 的 `OfficeTemplateComputer` 由既有 `OfficeHost` 创建，与普通 Agent 文稿计算共用进程内计算额度。`import` 接收原产品已授权、已读取的 `ContentRef` 和实际 PPTX 字节，校验内容身份，再调用 SDK `compute_import`。原生 PPTX 作为完整显式资源保留；不经过 HTML 重建，不把页面转成图片充当可编辑内容。

导入返回 SDK 的 `SnapshotRecord`。产品或其原维护 Agent 必须提供明确的公共 `TemplateDefinition`，再调用 `define`。SDK `DescribeTemplate` 校验来源 revision、语义摘要、参数目标、约束和真实源例值。生成的 `presentation-template-package/3` 继续包含原样 SDK 快照、定义及产品资源引用；包摘要覆盖精确输出字节，模板语义摘要仍由内核计算。

生产入口与原 `author@7` 模板消费入口使用同一个类型化包校验器。校验器要求资源声明、模型及实际输入构成完整对应关系，拒绝多余、缺失、身份冲突或被修改的字节。资源别名逐项计入预算，同一内容只做一次摘要核对。原上传限制为 64 MiB、40 页；模板包为 8 MiB，资源最多 1,024 项、每项 128 MiB、合计 256 MiB，并服从更低的宿主输入预算。序列化在写入时实施字节上限。

调用取消或异步调用方被丢弃后，已排入阻塞执行器的工作继续持有原计算额度，直到观察取消并释放；不能靠反复取消建立无界后台队列。这些状态只属于当前计算，不产生另一个任务表或持久执行系统。

`OfficePreparedTemplate` 表示定义与材料通过计算校验。它不代表布局压力、视觉质量、Office/WPS 互操作、目录入库或公开发布通过。没有提供定义的导入也不会自动变成一个“模板已完成”的结果。

## 原产品流程的后续衔接

当前 `TemplateImporter → ConfiguredTemplateCompilation → TemplateCompilationCoordinator` 的自动上传仍使用旧 HTML 流程。新增 `import/define` 已编译进入接入库并由实际原生导出专项调用；自动上传尚未调用这两个入口。现有候选 Agent 的目录读取已使用共用校验器。

后续沿原 `CompilationJournal`、维护 Runtime、内容端口和版本事务接入：

1. 冻结原生继任编译档案和 SDK/渲染/字体材料，保留旧政策及历史任务的原语义。
2. 原任务读取已固定的 PPTX，调用上述导入入口；原维护 Agent 对来源快照提出明确参数定义。单份来源的模板参数不需要把文稿拆成 HTML 页再拼回。
3. 参数压力样例、实际导出及逐页预览通过共享 SDK 计算，按精确文稿、模板、资源和环境身份保存验证材料；不使用旧 HTML 的 layouts/quality 字段伪装原生结果。
4. 原编译协调者复核完成阶段、generation/lease、owner、预期模板版本和全部材料后提交原目录。计算器不拥有提交或发布权限。

此安排复用已安装 SDK，尚不需要给 Python 后端添加另一个本地 CLI 或 MCP 计算实例。Cloud/Web/Device 原路由与受控接续仍需完成相应接线和验证；本阶段不改变 stable。

## 验证范围

阶段原始命令与源材料清单保存在隔离工作区 `.codex-work/integration/template-source/`，见[验证证据](../reviews/evidence/2026-09-28-product-template-source-verification.json)。新增 7 项计算测试、19 项 Agent 回归、57 项后端测试及 7 个后端子测试通过；接入库全部 target 按 `--no-deps -D warnings` 检查通过。初次测试编译错误及测试 lint 失败记录保留，均已修复，未降低检查范围。

实际原生实例保持与前一阶段相同的全部 PPTX 字节；21 个部件中，目标页及标题元数据为预期变化，另外 19 个部件保留。六份既有 PPTX 与两张 PNG 也逐字节一致。后端专项通过原 SQL/签名/gRPC 代码传输新入口实际生成的包，存储使用摘要校验的内存测试提供方。未声称线上上传、真实模型会话或 Office/WPS 视觉验收已通过。

替换目标保持：完整模板生产与消费、页面组装、HTML 历史迁移、真实桌面/模型链路、完整一期高级内容及 P00/E0–E3。当前 `device.v54 / author@7 / Skill v10` 仍为候选配置。
