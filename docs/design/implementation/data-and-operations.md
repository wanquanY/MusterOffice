# 数据、操作与编译合同

v0.4 设计基线。补充[领域模型](../presentations/document-model.md)；公共传输格式见[合同目录](../../contracts/README.md)。本页固定语义，JSON Schema 负责可表达的形状检查，引用、继承、布局及跨对象约束由语义验证器负责。

## 1. 数值、身份与默认值

| 项目 | 规定 |
| --- | --- |
| 长度/坐标 | 交换格式为规范十进制字符串的有符号 64 位整数 EMU；`1 pt = 12700 EMU`。尺寸要求非负，特定对象要求严格正值；JavaScript 不得先转 Number 再回字符串 |
| 角度 | 整数，单位为 1/60000 度；保存作者的正负方向，计算时作显式归一化 |
| 比例/颜色变换 | 整数百万分比；允许的上下界由具体属性规定，不能全局假定 0–1 |
| 时间 | `{ticks: canonicalIntegerString, timescale: positiveInteger}`；时间点用有理数比较和约分，避免把 29.97 fps 当 30 fps 累计 |
| 数据数值 | 图表用有限十进制数值及原有 lexical form/格式代码；明确区分 absent、blank、error、zero、date。禁止 NaN/Infinity 进入语义模型 |
| 标识 | 文档内稳定字符串，类型由所在记录/引用限定；对象、段落、单元格、系列、图点、数学节点、时间节点独立身份。复制显式返回 ID 映射 |
| 版本 | `documentId` 表示谱系；`revision` 是不可变版本句柄，宿主保存父版本/操作摘要/语义摘要/来源摘要。存储递增序号与 revision 不混用 |
| 文本 | 有效 Unicode scalar 序列，不作隐式 NFC/NFKC；锚点以 scalar offset + affinity 表示，grapheme 边界用于交互选择 |
| 可继承值 | `inherit`、`value(T)`，以及该属性允许时的 `clear`；字段省略等同 inherit；显式无填充、空字符串、0 均是 value，不等于缺省 |
| 序列/字典 | 页面、层叠、段落、系列等是有序 ID 序列；记录表按 ID 查询；顺序不得依赖 hash map 遍历 |

内部几何的加乘/矩阵组合采用检查溢出的宽整数或有理数；需要超越函数的几何在固定数值规则下计算，并在规定阶段量化。溢出是定位到对象的错误，不允许 wrap 或跨端饱和差异。JSON wire 不直接暴露 Rust 指针、usize、UTF-16 下标或 GPU 对象。

语义摘要包含显式属性、逻辑顺序、资源内容摘要、原生扩展及关系；来源摘要覆盖保留的源包。派生缓存和墙钟不参与。对象 ID 是语义关系的一部分，不能为了摘要相同随意重编号。规范序列化固定 UTF-8、键顺序和数值字符串规则，协议描述文本与用户文字不作相同的“清理”。

## 2. 记录集合和所有权

| 记录 | 必需语义 | 允许的派生数据 |
| --- | --- | --- |
| Document | id/revision、pageSettings、主题/母版/版式表、页面顺序、分节/放映、元数据、资源/来源索引 | 只允许另存索引，不能将有效继承值覆盖回作者属性 |
| Slide / Master / Layout | 类型化关系、占位符、背景、对象顺序、各自文本/颜色映射；Slide 另持 notes/comments/timing/transition | 解析后的占位符匹配与背景 |
| ObjectCommon | id/type、父容器、transform、z-order 所属序列、styleBinding、可见性、替代文本、阅读顺序、sourceBinding | 视觉 bbox、hit-test、合成层 |
| TextBody | 段落、run、显式换行、列表/制表/域、段落/字符样式、内边距、分栏、竖排、autofitPolicy | 行、簇、字形、软折行、基线；不可写成作者强制换行 |
| Shape / Connector / Group | 预设及调整值或自定义路径/公式、连接端点/附着、子对象变换 | 展开路径和路由；不得丢掉调整柄/连接关系 |
| Picture | 资源、裁剪源域、透明/效果、链接或嵌入意图 | 解码图、mipmap、显示纹理 |
| Table | 行列 ID/尺寸、cell ID/跨度、富文本、边框归属、样式和首尾行列标记 | 单元格布局、共享边界胜出结果 |
| Chart | 类型、plot、axis/series/category ID、数据引用、数据值/公式/格式、标签/图例/样式、workbookBinding | 刻度/图元/标签位置、数据缓存；数据与工作簿必须一致 |
| Diagram | data point/connection ID、layout tree、algorithm/constraint/rule、style/color bindings、作者覆盖 | 布局实例、派生 drawing、分步目标映射 |
| Math | typed tree、node ID、token/structure/style、行内/独立、断行与对齐属性 | math box、stretch glyph、基线和选择区域 |
| Media | asset/track、封面、裁剪、trim/volume/fade/loop、bookmark、跨页策略、字幕/替代信息 | 解码状态只属于播放会话 |
| Model3D / Ink | 场景/材质/相机/灯光/模型动作；或笔画采样/笔刷/压力/时间/变换 | mesh/纹理/GPU buffer 或 stroke mesh |
| Timing / Transition | 有身份的时间图、typed target、事件条件、持续/重复/fill/easing、效果参数、跨页关系 | 排序后的求值计划及 checkpoint |
| Note / Comment / Action | 原生内容、作者/线程、锚点、受限动作类型、引用 | 显示状态和宿主激活结果 |
| SourcePackage | 原 bytes 资源、part/relationship/namespace 图、源路径及保留区、known-field overlays | 受影响闭包和写回计划，不能作为第二套可写文档 |

样式属性以明确的文字/形状/表格/图表/媒体属性集合表达。未知字段属于来源保留区，并有格式命名空间；不得塞进无约束 `properties:any` 后绕过类型与版本控制。对象可扩展不等于任意 JSON 可执行。

Schema 演进顺序：领域记录 → 类型化操作 → Reader/Writer 映射 → Schema/SDK 生成 → 正反例。每种记录必须给出默认值、必填项、数量/范围、继承行为、引用规则和对应原生字段；不能由 TS、Rust、MCP 各维护一份不同枚举。

## 3. 操作全集和行为

传输工具只包装下列领域操作；域外授权与持久化由宿主执行。允许批量提交，不能提供任意 JSONPath 写入或任意脚本。

| 操作族 | 操作和输入 | 成功结果/引用约束 |
| --- | --- | --- |
| document | create/open/query/diff/validate/requirements/close | 固定版本句柄、分页游标、能力与资源报告；query 游标绑定 revision |
| slide | insert/delete/duplicate/move/updateSettings/setLayout | 新页/复制映射；页动作、自定义放映和转场匹配引用共同校验 |
| definition | putTheme/putMaster/putLayout/putNotesMaster/setHandoutMaster/deleteDefinition | typed 完整定义或 typed 属性更新；默认共享修改，想局部变化必须显式 fork |
| object | insert/delete/duplicate/move/setTransform/setAppearance/setAccessibility | 禁止更改 type 假装转换；delete 默认 reject 依赖，明确 cascade/retarget 才处理 |
| group / connector | group/ungroup/attach/detach/updateGeometry | 保留世界坐标和连接关系；复制或拆组输出全部 ID 映射 |
| text | splice/splitParagraph/joinParagraph/setRunStyle/setParagraphStyle/setBodySettings/setField | scalar 范围、文本/typed inline 内容；维护锚点、显式换行及时间目标 |
| table | insert/delete/move/resizeRowOrColumn/merge/split/setCellText/setStyle | 合并不能静默丢非空单元格；需指定内容合并策略并返回映射 |
| chart | setType/putAxis/putSeries/removeSeries/setData/setLabels/setStyle | 更新数据、缓存、工作簿投影、系列/分类时间目标；修改外链数据必须明确内嵌/保持链接 |
| diagram | putPoint/putConnection/remove/reparent/setLayout/setStyle/setOverride/resetOverride | 校验连通/有向结构与布局规则；保留作者覆盖，不能以新图形替换旧 SmartArt 身份 |
| math | replaceSubtree/setStyle/setAlignment/setDisplayMode | 新旧节点映射及行内锚点更新；LaTeX 输入若提供须先转换为 typed tree 并报告不支持项 |
| media / model3d / ink | setSource/setPlayback/setBookmark/setScene/setModelAction/putStroke/removeStroke | 更新依赖、时间目标和原生资源关系；资源替换不能改变授权域 |
| timeline / transition | putNode/removeNode/reorder/setCondition/setTarget/setEffect/setTransition | 定时图合法、目标存在、循环符合有界放映控制策略；不得隐式改为“同时播放” |
| note / comment / action | put/remove/reply/resolve/setAction | 删除锚点依赖策略显式；链接只记录语义，执行归宿主 |
| resource | attach/replace/detach | 只接受已授权资源句柄；引用中的实际 hash 变化使缓存与质量证据失效 |
| template | describe/instantiate/bind/validate/stress | 模板版本固定；参数同时检查对象/子对象/时间引用；实例复制后与源模板版本隔离 |
| compile / export | compile/sample/export/verify | 同一 revision/profile/resources 对应 plan、暂存文件、实际字节报告；无产品数据库副作用 |
| playback | create/prepare/input/evaluate/seek/pause/resume/dispose | 会话句柄、序列化状态摘要、FrameState/MediaIntent；不产生文档编辑版本 |

大对象替换允许通过 typed `put` 操作，便于机器生成和前向完整表达；常见细粒度操作避免 Agent 重传整页。两类操作使用同一个语义验证器和变更集，不维持两套规则。

## 4. 事务、撤销和并发

事务顺序固定为：信封/版本检查 → 所需权限和 capability 检查 → 构建私有工作版本 → 按列表应用 → 全局引用/来源闭包检查 → 产生 immutable revision 与 changeSet。中间状态不可被查询或播放观察；冲突/失败时无部分提交。

每个操作有 `operationId`；事务有 `requestId/baseRevision`。同一幂等范围内相同 requestId+规范请求摘要返回原收据；不同摘要返回 `REQUEST_ID_REUSED`。旧 revision 返回 `REVISION_CONFLICT` 并附当前版本及可查询的差异引用。**不自动把 Agent 修改重放到新版本**，因为可能改变用户文字、图表数据和动画目标。

undo/redo 是新的有版本操作。宿主保存有限历史和 inverse changeSet/被删除资源句柄，内核计算可逆修改；遇到已被其他修改改变的依赖时报告冲突。清理历史前确认没有旧 Artifact、活动会话或迁移引用。多端协作可将有序事务提交到同一 CAS owner，本期不要求用 CRDT 重写整个 Office 对象图。

ChangeSet 至少包含 created/updated/deleted IDs、ID/anchor maps、受影响定义与资源、来源覆盖、时间引用变化、布局失效集合及新摘要。宿主必须把 revision、收据和业务提交放进自己的持久事务；核心的成功不等于产品 Artifact 已发布。

## 5. 编译 DAG 与缓存

```text
parse/source index → typed document
typed document + profile + resource hashes
  → inheritance resolution → resource/semantic closure
  → text/native-object layout → geometry/visual bounds
  → timing/interaction compilation → composition plan
  → native write plan + render plan + diagnostics
write plan → staged bytes → reopened projection → verification receipt
```

每个节点键为 `(node kind, semantic input digest, source digest when relevant, resource digest, rules version, output profile)`。节点输出不可变，依赖图采用反向索引失效；重算到输出摘要不变时可停止向下传播。异步资源到达只恢复对应等待节点，不能混用前一次 revision 的资源。

与外部状态有关的解码器、音频时钟和 GPU 句柄不写进语义缓存；它们通过 session generation 和资源摘要绑定。计划序列化必须带 schema/engine/rules/feature-registry 摘要；不匹配时重新编译，不能反序列化旧内存布局。

来源写回与视觉编译分别有依赖：只改批注通常不重塑全文，但必定产生不同文件与证据；换字体可能不改作者文字却改变大量布局。增量正确性的 oracle 是同输入的全量计算，而不是“看上去只动了一页”。

## 6. 查询、失败和可恢复性

query 支持 slide/object/type/capability/source-part/time-range/changeSet 过滤；返回逻辑对象、解析属性及 provenance 时明确区分两者。大结果分页/资源句柄交付；诊断可按 severity/stage/变更范围查询，不默认把整个源包送给模型。

稳定错误码族：`INPUT_INVALID`、`REVISION_CONFLICT`、`REFERENCE_CONFLICT`、`RESOURCE_REQUIRED`、`HOST_CAPABILITY_MISSING`、`TARGET_INCOMPATIBLE`、`SOURCE_PRESERVATION_CONFLICT`、`LIMIT_EXCEEDED`、`CANCELLED`、`INTERNAL_FAILURE`。带对象/属性路径、实际/期望、可重试条件和证据句柄；失败不能退化成“返回一个空白 PPTX”。

`RESOURCE_REQUIRED` 只有在输入本身合法、需求明确时可恢复；损坏 ZIP/XML、无效数学树、越界时间参数是最终输入失败。恢复必须保持原 revision、engine/profile 和资源授权域，否则产生新的执行与收据。

正式 API/ABI 实施时应从统一 IDL 生成 Rust/TS 类型及 JSON Schema，并用同一 corpus 比较所有入口。当前设计信封 Schema 已落盘，**完整领域 IDL 及生成器尚属 E1 交付项**；这里的字段合同不能被几个信封校验结果冒充已全部实现。
