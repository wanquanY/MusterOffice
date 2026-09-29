# 保留来源的统一文档与事务

2026-09-27，架构评审 F2 的贯穿实现。这里记录已实现的字段范围；不将来源保留等同于完整 PPTX 编辑、Office/WPS 互操作或视觉验收。

## 唯一可变事实源

作者与导入文稿使用同一个 `Document`、`Snapshot`、`Transaction`、revision、请求摘要和 `ChangeSet`。导入对象进入原有 `objects` 表，不另设来源对象注册表或来源业务 revision。`SourceBindings` 保存原始包的授权资源 ID、原生部件/对象/run 地址和不可变编辑约束；原始字节由宿主保存。

领域 ID 使用带用途域的摘要，由文档身份、原始包摘要和原生地址生成。名字不是修改凭据，部件内原生数字 ID 也不是全局 ID。同一导入历史内多次编辑/导出的 ID 保持稳定；从外部重新导入另一份文件是新的来源，不宣称能自动恢复先前的身份映射。

未完整提升到作者模型的对象使用 `RetainedSource`，保留 shape、picture、group、connector、graphicFrame 的区别、分组归属和已读取的段落/run。原生几何、图片关系、主题/富文本样式和未知扩展仍由原始包提供。表面记录中的未提升样式不代表已解析为默认样式；静态作者 API 会拒绝来源文档，正常交付使用来源计划和共享资源编译器。

`Object.transform` 为可选直接声明：没有完整原生位置/尺寸时为 `None`，不把继承求值结果冻结成假定的作者位置。作者对象仍必须提供完整变换；原有作者 JSON 表达、摘要与历史 revision 不因这个类型调整改变。

## 同一事务与保留式写回

普通 run 的 `SpliceText` 与作者文本共用 Unicode scalar 范围/锚点变换算法。`SetTransform` 修改有完整直接坐标且允许编辑的原生对象；未变的旋转/翻转属性存在性及 group 子坐标保持原样。受兼容性分支、结构化叶、动态字段、时间引用、保留声明或签名保护的字段拒绝修改。

当前来源事务开放这两类已实现的字段操作。结构增删、换主题/样式、资源替换、标题和时间图修改尚无协调保留规则，整笔事务明确失败；不允许先成功提交、再静默丢弃不能写回的操作。无修改的来源导出逐字节等于原包。

[`SourcePlan`](../../crates/mo-presentation-source/src/source/document.rs) 从真实原包重新建立可信绑定，并核对文档中每项来源身份、约束和结构。它只接受上述字段的差异，构造预期的类型化声明覆盖层。手工伪造约束、原生地址或其他模型字段会失败。

旧的格式级文本/坐标工具与来源计划共用 [`PreservedPlan`](../../crates/mo-presentation-source/src/source/preserve.rs)：一次索引、合并文本与属性修改、每个部件一个 rewrite、一次封装写入。对象地址按索引查找，不按每条修改重复扫描整页。writer 在宿主 sink 封存后独立读取实际输出，比较预期声明；未改动的 OPC 条目、关系及未知内容保留。格式工具不拥有独立业务事务或持久化状态。

## 编译、资源与交付

[`PresentationPlan`](../../crates/mo-presentation-source/src/plan.rs) 从同一文档生成作者声明计划或经过验证的来源字段计划。两者共用原生继承/主题/图片/文字语义和资源页面编译。保留来源的预览直接使用覆盖后的声明与原始包的资源，不依赖先导出再解析 PPTX。

封闭的 `SourceImages` 同时绑定逻辑计划摘要与原始包摘要。普通 `PackageRead` 始终表示实际包，不能伪装成逻辑计划。来源批次 `--preview-retained-document` 一次传入模型、原始包和字体，循环外准备包/计划/字体；每页渲染和输出仍有预算。内嵌 export worker 使用同一页面批次计算入口。

交付的 `preview-evidence/3-draft` 使用 `planSha256` 和实际输出的 `pptxSha256`。接收端读取交付内保留的原始资源，重建计划并分别核对两种身份。v1 文件来源和 v2 作者计划证据保留各自的旧规则，不交叉接受。来源文稿的原包作为资源随可编辑模型交付，后续事务仍以原始来源为保留依据。

页面依赖选择当前页、版式、母版、主题及其对象绑定；原始包摘要保守地影响所有页。局部字段修改按可见依赖使页面失效，不把完整来源绑定表重复装入每页缓存键。

## 入口、权限与兼容性

- `presentations.import` / MCP `mo_presentations_import`：请求包含 `documentId` 与已有资产到 `resourceId` 的绑定。宿主检查 Create、ReadAssets 权限和资产身份；纯计算导入产生普通 MutationCandidate，经现有 job、取消、fence、CAS、幂等回执和 revision 存储提交。
- `presentations.create` 拒绝客户端直接提交带来源绑定的文档；来源绑定通过真实导入建立。后续编辑继续用 `presentations.apply`，导出继续用 `presentations.export`，没有平行的业务操作系统。
- CLI `pptx-import` 与 WASM `import_pptx_document` 使用相同的纯导入计算，传入二进制原包及其预期摘要。它们返回普通 SnapshotRecord，不授予宿主资源访问或发布权限。
- 原有 author-model profile 的 wire 名称保持兼容，现涵盖统一文档的已实现字段操作。新增合同需重新生成 Schema/TS；Rust 使用者需适配可选 `Object.transform`。

当前版本化导入接受 Transitional PPTX presentation，要求显式页尺寸；有母版/版式时须具有模型所需的主题/母版关系。放映、模板、宏容器、Strict 及缺失必要关系的文件明确拒绝。底层只读来源查询的能力范围不因此扩大或缩小。

数据库仍使用 schema 6，新增对象/操作属于 JSON 合同扩展。旧二进制不能解释新增来源文档或 import job；回滚部署必须兼顾新数据和备份，不能把 Git revert 当成数据降级。无新来源文档的旧作者数据保持原始表示。

## 验证口径

定向回归验证混合文字/坐标事务、稳定 ID、锚点、局部失效、无操作逐字节保留、未知扩展和未改条目、伪造绑定、保护字段、取消及过期 revision。宿主回归覆盖实际上传/导入、权限、调度、提交、重放和重启读取。

真实 worker 对照直接来源计划与实际导出回读的像素和完整元数据，并检查原始包传输一次和交付回读；Native/WASM 对照真实导入的模型/身份以及同一编辑事务。最终执行结果和限制以[整改记录](review-corrections.md)及其验证证据为准。

## 表格来源投影版本

2026-09-29 新导入使用 `presentationml-retained-fields-v3-draft`，在 V2 标题及字段语义上增加物理单元格文字。V1/V2 保持原有表格保留语义，基线重建按 profile 执行，禁止手工换版本冒充迁移。表格普通文字复用已有事务和保留式写回，详见[表格来源实现](table-source.md)。这项变化不增加内核权限、存储或产品页面，普通调用仍是纯计算。
