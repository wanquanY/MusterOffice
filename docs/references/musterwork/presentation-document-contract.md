> Musterwork 历史设计快照 · 迁入日期：2026-09-24。来源：`docs/architecture/agent-runtime/presentation-document-contract.md`。
>
> 原文中的“当前”“已采纳”和商业 SDK 选择只代表来源项目当时的状态，不是 MusterOffice 的决定或验收结论。内核以[现行设计](../../design/presentations.md)为准；来源、摘要和链接转换见[迁移清单](README.md)。

# 标准演示文稿文档合同

状态：实施中。版本：v0.2，2026-09-14。本文记录新引擎已分配的协议名称，
不是全端启用或目标应用验收证明。完整目标见
[HTML 创作与模板编译设计](presentation-html-authoring-and-template-compilation.md)。

## 事实源和版本

`presentation-document/1` 是规范化 JSON 存储的声明式 HTML AST。唯一可变内容是
这份文档；HTML、`presentation-scene/1`、PPTX、对象映射和预览均是派生物。
旧 `slidespec/1..3` 的名称及字段含义不变。新文档不要求模板槽位或 `layout_id`；
模板来源只在页面的 `origin` 中保存确切引用。

可执行验证器在 document/schema.mjs（`Musterwork:packages/presentation-engine/src/document/schema.mjs`）
及其同目录模块，HTML 输入由结构解析器转入同一验证器。未知字段、无穷数、未配对
Unicode surrogate、危险属性和未知 token 拒绝；字符串 NFC 规范化，负零变为零，
键按字典顺序序列化。JSON 摘要使用此确切 UTF-8 序列化形式的 SHA-256。

文稿包含 `id/title/language/canvas/theme/assets/pages`。页和节点的 ID 在整份文稿中
唯一，页序与身份分离；画布使用 CSS px，原生边界统一采用 `1px=9525EMU=0.75pt`。
当前版本上限为 40 页、12000 节点、32 层节点嵌套、4 MiB 文档、每文本对象 30000
字符。各 Runtime/Backend 入站层必须一致执行此新合同；旧合同保持原有上限。

## 节点与布局

节点类型为 `container/group/text/image/icon/path/shape/connector/table/chart`。
布局为固定画布上的 absolute、block、flex、grid；样式、主题 token、富文本、
图片裁切和结构化数据分别验证。不能保存任意 CSS 字符串或 raw HTML。

`text.paragraphs` 保留自然段、run 样式、语言、列表及明确的 HTTPS/mailto 链接。
表格逐行逐列保存单元格；合并源单元格声明 `rowSpan/columnSpan`，覆盖位置必须为
`null`，不能在不可见续格中藏业务值。图表保存分类、系列、缺失值及散点图每个系列
独立的 X 数据；原生缓存、公式区域和嵌入 XLSX 必须一致。

矢量使用绝对 `M/L/C/Q/Z` 命令。HTML 片段的相对路径/圆弧需先转为绝对曲线；
不将不支持的路径偷偷栅格化。SVG 图片和原生路径是不同可编辑类别。

## 素材和字体

图片只能引用 `assets` 中的 ContentRef，`byte_length` 为正十进制字符串；引擎
从受授权暂存根读取 `<sha256>.png/jpg/svg`，重新检查摘要、长度、文件签名和
实际解码尺寸。引擎不读取 Agent 提供的绝对路径、网络图片或隐式关联文件。

字体由宿主配置 `presentation-fonts/1` 清单，包含 face 的 family、weight、italic、
文件摘要、许可与 embedding 许可类别，以及显式 fallback 表。缺字/缺 face 阻止编译；
显式 fallback 记录逐节点诊断。浏览器页面使用这些实际字体字节，不通过宿主字体
名称猜测是否可用。字体安装、目标转换进程字体环境和上传字体隔离仍需完成集成。

## 操作和模板

纯文档操作要求 `expected_sha256`，整批最多 256 个操作。所有操作在副本中应用，
最终整体验证后才返回。Runtime 另以持久 revision/CAS 事务提交；进程内成功不等于
持久提交。操作包含 document.set、asset.put/remove、page.insert/remove/move/
replace/duplicate/set、node.insert/remove/move/replace/set。

节点/页面替换保留被替换对象的 ID，复制分配独立 ID 并重写连接线端点。模板实例化
返回独立完整页，装饰及布局均可修改。`presentation-html-template/1` 包包含文档、
语义布局、类型化参数、原页映射和编译 profile；参数通过节点 ID 与属性绑定，禁止
字符串替换 HTML。必要参数缺失不能带着示例内容假装填充完成。

## 引擎与候选交付

在既有 CLI 中增加 `document_inspect/document_edit/document_parse_html/
template_instantiate/template_validate/import_extract/import_materialize/source_preview/author`。
旧 inspect/compile/update/preview 保持兼容。
路径只由暂存适配器传入，产品 Tool 不能直接暴露引擎文件路径。

`author` 返回候选目录，包含规范化 document、派生 scene、PPTX、对象映射、QA、
HTML 创作预览和最终 PPTX 的真实预览。只有最后写出的 `candidate.json` 标记
候选目录完整；Runtime 不得看到一个 PPTX 文件就认定业务成功。

`presentation-object-map/1` 固定文档与输出摘要，每个节点关联一个实际原生对象。
`presentation-quality/2` 分别记录原生语义证明、真实预览证明、视觉审阅、目标应用
证据和环境。确定性检查通过后状态为 `awaiting_visual_review`；无最终预览为
`draft_only`；布局/内容错误为 `needs_revision`。这些状态不能冒充已交付 Artifact。
目标应用证据尚未执行时明确为 `not_proven`。

## Runtime 草稿与交付清单

Runtime 持久草稿要求 revision 与 document SHA 双重匹配，模型通过当前 `base_ref` 表达基线；
摘要、版本和权限由 Runtime 校验。候选固定当前文档 ContentRef、页序、实际 PPTX/PNG/PDF/
对象映射/编译报告及字体 profile。`artifact.presentation.author@3` 的 `compile` 在确定性校验
通过后，同事务提交 Artifact 版本、文件、工具收据与 Ledger。草稿保持开放并推进 Artifact 基线，
后续改稿再次 compile 更新同一 Artifact；并发版本冲突须读取当前状态，不能覆盖新版本。
已关闭草稿不能继续推进。重复命令返回同一收据，不产生第二个版本。

`presentation-artifact/2` 是当前复合交付清单，MIME 为
`application/vnd.musterwork.presentation-manifest+json`。`output_pptx` 是唯一交付文件，
`source_pptx` 是可选上传/外部修改来源，`document` 是规范文档；不能按依赖顺序或 PPTX MIME 猜测输出。
`previews.source_pptx_sha256` 必须等于输出文件摘要。`quality.compiler_report` 保留编译候选证据，
`quality.status` 为 `compiled`，不存在 `quality.visual_review`。Agent 主动读取每页实际图片并修正，
视觉判断不依赖第二个模型的通过声明，也不构成独立交付门禁。完整可达依赖必须与 ArtifactVersion 完全相等；通用复合依赖上限提高到 1024，
以承载本格式的最多 512 素材与 40 页派生文件，各格式仍执行自身更小的限制。

`PresentationAuthoringChanged` 是类型化 Ledger 恢复事实。其 `checkpoint_ref` 使用
`application/vnd.musterwork.presentation-authoring-checkpoint+json`，不会自动取得公共下载授权。
Checkpoint 包含精确 UTF-8 状态编码；数据库索引列、编码内身份与内容 SHA 必须一致。

最终预览首先核对 PPTX/页数/页图/PDF 摘要，再使用受管 PDF 文本提取器检查对应
对象区域的必要文字。Agent 检查内容、布局与可读性；`compiled` 不声明做过 Office/WPS 验收。
普通用户创作工具没有 `review` 或 `publish` 操作。模板目录的管理员上架权限与普通文件导出是不同职责。

历史 `presentation-artifact/1` 仍要求完整且通过的 `quality.visual_review`；旧 author@1/@2
冻结任务继续按原契约执行。不能通过把历史清单改成 v2 或忽略旧证明来恢复失败任务。
引擎候选 `presentation-quality/2` 的旧 `awaiting_visual_review` 名称仍是内部兼容协议；
新工具在确定性候选验证后输出 `compiled`，不向 Agent 要求旧评审操作。

## 实施及证据

当前实际执行记录与剩余项见
实施记录（`Musterwork:docs/plans/agent-runtime/presentation-html-authoring-implementation.md`）。
方案的完整目标没有因当前引擎纵切完成而缩减。


## 模板 Agent 主动提交

新任务冻结 `presentation.compilation@2` 策略，只配置创作模型路由，复用共享 AgentLoop、
Model/Tool Journal 和私有维护执行身份。Agent 主动调用 `presentation_template` 的闭合操作 schema，
读取 source/final 页图、编辑、生成压力样本并调用 `prepare_stage`。最后必须实际调用
`submit_presentation_template_stage({output_ref})`；工具验证阶段输入来源、候选与工具收据、
压力样本和最终文件闭包，通过后才产生持久提交。普通最终文本或 JSON 内容不能提交阶段。

当前阶段结果为 `presentation-template-stage-output/2`，使用实际候选及预览证明；
不要求独立视觉模型响应。普通参数/原生渲染失败以带原因和后续操作提示的工具反馈持久化，
Agent 可在同一阶段调整再调用。日志提交故障、失效租约和取消仍由共享 Runtime 的恢复机制负责。

## 历史私有视觉审阅与维护执行

以下评审契约仅用于已冻结的旧策略和历史交付，不是当前新任务的前置条件。

`PresentationVisualReview` 保存多个实际模型调用批次，批次图片闭包按最终文稿页序完整拼接。
单次调用必须遵守准入冻结的 `VisionInputProfile`，不是固定假设整份文稿能放入一个请求。
同一候选的新审阅通过单调序列确定先后；时间戳相同也不能让旧通过替代新拒绝。
清单 `quality.visual_review.model_invocation_ids` 是真实物理调用列表，不含私有请求/响应内容。

`presentation.visual-review@1` 使用固定指令、`presentation.visual-review.output@1` SchemaRef、
实际最终 PPTX 预览 image parts 和 `presentation-visual-review/1` 逐页判定。模型输出的身份与每页图片摘要
必须完全匹配；blocking finding 与 passed 矛盾、缺页和未审阅页的推测均拒绝。

`musterwork.runtime.presentation-maintenance-snapshot.v1` 是独立的私有执行快照，
包含模板编译目的（reconstruct/parameterize/stress/verify/assemble）或单批视觉审阅目的；
不复用 DreamJob/Memory/学习信号。`musterwork.execution-grant.presentation-maintenance.v1`
使用独立签名域绑定模板/审阅固定 profile、租户/Run/主身份、单一 stateless 视觉路由和维护 surface。
正式调用仍必须由共用 Runtime AgentLoop、Attempt fence、Model Journal 与 Usage Outbox 执行。


### 私有执行身份和审计归属

模板编译和视觉审阅采用普通 Maintenance Activity/Run，按不可变 request_id 派生私有会话，
输入上下文只存在于已签名的专用 Snapshot 中。调度者和执行者是两个不同 Service Participant；
执行者绑定 `presentation.template-agent@1` 或 `presentation.visual-review@1`，不占用产品
AgentDefinition 的唯一内置主 Agent 槽位，也不产生 Memory 或 Dream 学习身份。

AdmissionSnapshotRef 是 Ledger 所有存储适配器共同使用的只读封存数据视图。私有请求索引、
输入 ContentRef、Run、Ledger/Outbox 由同一准入事务提交。请求索引只用于身份、恢复查询和内容保留，
终态以 Runtime Run 为准。模型调用、重试、Usage 和终态仍由同一个 Agent Loop/Model Journal 持久化。

Cloud 审计从 `security_audit.run_executor_authorities` 解析执行身份；Service Agent 仅在对应私有
请求、会话所有者、Snapshot schema/hash/proof 全部一致时具备该 Run 的审计身份。
普通 User/Agent 和 Dream 的既有映射保持不变。Desktop 的模型凭据准入来源显式记为
PresentationMaintenance（code 5），其主调用为 Chat 路由，不归入 EvolutionInduction。
