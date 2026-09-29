# 文档模型、编译计划与 Agent 协议

v0.4 设计提案 · 2026-09-24。上层约束见[主设计](../presentations.md)。以下结构是领域模型；单位、记录、操作全集和编译规则见[实施合同](../implementation/data-and-operations.md)，不是已发布 SDK。

本页定义领域语义；MCP 工具、Skill、Plugin、CLI/SDK 的映射与版本由[公共操作合同](../agent-interfaces.md)负责。相同事务不得因调用入口不同而出现不同的默认值、验证或提交语义。

整份模板实例化的固定来源、参数计算及文档作用域映射已进入[实现](../../implementation/template-computation.md)。原生来源对象的派生命名空间独立于实例文档身份；这不改变局部引用，也不代替部分页面复制时必须进行的引用闭包和身份重映射。模板目录、选择和保存继续由接入产品拥有。

原生导入的 V2 来源投影将 OPC core properties 中的标题纳入 Document 语义及普通原子编辑；历史 V1 的空标题保持未投影含义，不自动解释为删除原标题。保留式写回与兼容边界见[原生标题语义](../../implementation/native-titles.md)。

## 1. 四种表示及其所有权

| 表示 | 内容 | 可变性与责任 |
| --- | --- | --- |
| Presentation Document | 可编辑领域对象、关系、样式、时间、资源引用 | 唯一语义事实源；只能通过事务修改 |
| SourcePackage | 原始 OPC 部件/关系、XML 保留区、对象来源与未知扩展 | 绑定导入字节；由事务变更集驱动覆盖，禁止独立编辑 |
| CompiledPlan | 解析后的继承、布局、对象语义、时间图、资源/来源绑定 | 不可变派生物；绑定完整编译键 |
| Scene / FrameState | 绘制命令、字形、裁剪、合成层及某一时刻的值 | 不可变执行结果；不能反向成为文档模型 |

SourcePackage 不是另一份可写 XML 文档。导入时保留原包；修改已理解字段产生类型化覆盖，未理解内容按依赖规则保留。是否可安全组合由格式模块判定，见[互操作规格](ooxml-interoperability.md)。

## 2. 概念结构

```text
PresentationDocument
  identity, schemaVersion, revision, provenance
  pageSettings, metadata, sections, customShows
  themes, masters, layouts, notesMasters, handoutMaster
  slides[]
    id, layoutRef, visibility, background, objects[]
    notes, comments, actions, timingRoot, transition
  objectRegistry
    Shape | Text | Picture | Table | Chart | Group | Connector
    Diagram | Math | Media | retainedExtension
  mediaCatalog, resourceCatalog
  sourceBindings, compatibilityRequirements
```

`retainedExtension` 只代表可保留但未完全理解的外部内容；不能用于声称一期已承诺能力已经完成。页顺序与对象身份分离，重排页面不改变对象 ID。

各类对象保留语义及几何：Chart 持有系列/轴/数值格式/数据，Diagram 持有数据点/连接/布局定义，Math 持有数学树，Media 持有资源和时间属性。不能仅用通用 `path + text` 容器表达这些对象。

文档坐标以有符号64位整数 EMU 为交换基准，wire 使用规范十进制字符串；角度与比例采用已定义定点单位，时间用有理数。内部宽整数/有理运算检查溢出，超越函数的规则与量化阶段固定。不得经 JavaScript 不安全整数或浮点累计损坏输入；详见[数值合同](../implementation/data-and-operations.md)。

## 3. 继承及稳定引用

每个可继承属性分别存储 `unspecified / explicit(value)`、继承来源和解析后的有效值。有效值进入编译缓存，不能写回覆盖作者未显式设置的属性。主题颜色保留 token、变换与来源，不只是计算后的 RGB。

继承按属性族定义，不采用任意通用对象深合并：占位符匹配、文本层级、颜色映射、背景和图表样式各有解析规则。顺序基于主题、母版、版式、页面及对象的具体格式语义；显式“无填充/无效果”必须区别于未设置。切换主题或重置版式应重新解析，而不是丢掉原引用。

引用类型至少包括 `SlideId`、`ObjectId`、`ParagraphId`、`TextAnchor`、`TableCellId`、`ChartSeriesId`、`DiagramPointId`、`MathNodeId`、`TimingNodeId`。ID 在文档内唯一，复制操作分配新 ID 并返回映射；PPTX 的局部数字 ID 和关系 ID 通过来源绑定映射，不能直接充当全局身份。

文字持有逻辑字符串、段落、run 和显式换行；塑形簇、字形索引与自动折行只属于派生布局。公开文本操作以段落内 Unicode scalar offset 加边界亲和性定位，检查有效 Unicode；面向人的选择接口可返回 grapheme 边界。禁止跨端混用 UTF-16 code unit 与 scalar offset。

文字目标以稳定锚点记录，事务维护 scalar、grapheme、源格式索引三种映射。插入点在范围边界时由亲和性决定是否纳入；删除整个目标默认拒绝并返回依赖，调用方可显式选择删除效果或重定向。禁止静默让“第二段淡入”变成另一段淡入。

## 4. 事务及引用完整性

概念请求：

```json
{
  "protocol": "proposal/presentations",
  "operation": "document.transact",
  "documentId": "deck-1",
  "baseRevision": "rev-12",
  "requestId": "edit-43",
  "operations": [
    {
      "type": "text.replace",
      "target": {"objectId": "title-1", "paragraphId": "p-1"},
      "range": {"startScalar": 0, "endScalar": 2},
      "text": "年度",
      "targetPolicy": "preserve-anchors"
    }
  ]
}
```

该示例展示语义，不定义正式 wire discriminator。按 [ADR 0007](../../decisions/0007-kernel-only-integration-boundary.md)，接入产品先完成业务授权；内核执行：版本检查 → 操作/计算能力校验 → 在私有工作版本计算变更 → 更新显式资源引用 → 校验依赖与来源闭包 → 原子产生新 revision。任一步失败，原 revision 不变。此处私有工作版本是计算状态，不要求文稿数据库。

| 修改 | 必须共同处理 |
| --- | --- |
| 删除对象/页面 | 动画、连接线、动作链接、自定义放映、备注及来源关系 |
| 复制/分组/解组 | 子对象 ID、坐标变换、时间目标、裁剪、共享资源与来源映射 |
| 改文字/表格/图表数据 | 文本锚点、单元格或系列目标、布局缓存、工作簿/图表缓存 |
| SmartArt 改层级 | 数据连接、布局实例、派生 drawing、动画步骤 |
| 公式子树替换 | 节点 ID、行内基线、选择范围和原生公式绑定 |
| 更换母版/主题/字体 | 继承依赖、受影响页、原生字体映射和已编译计划 |

引用处理策略是操作的一部分：`reject / cascade / retarget(explicit mapping)`。缺省 `reject`；返回依赖列表供 Agent 选择。多操作可在同一事务内先重定向再删除，避免中间非法状态可见。

同一 requestId 与相同规范化请求在同一会话返回同一结果；不同请求内容冲突。跨进程幂等、持久日志和提交去重归宿主。核心不伪造跨崩溃的 exactly-once 保证。

## 5. 编译与增量失效

```text
Document + resource manifest + compatibility profile
  → Resolve inheritance and object references
  → Validate semantic and resource closure
  → Shape / layout native objects
  → Compile timing and composition dependencies
  → CompiledPlan
```

编译键包含文档语义摘要、来源绑定摘要、资源内容摘要、字体 face/实例、布局/兼容规则版本、渲染配置。纯派生缓存、上次 QA、墙钟和缓存路径不进入文档语义摘要；源包未知内容的改变仍进入来源摘要和文档版本身份，防止误用旧结果。

规范化编码明确键顺序、数字范围、缺省值和字符串规则；导入时不得为求摘要稳定擅自做 Unicode NFC 或改写作者文字。revision 是不可变版本标识，不是仅依赖页面截图的哈希。

失效按依赖图传播：文字变更影响对象和所属容器布局；主题变化影响引用该 token 的对象；字体改变影响所有对应塑形和依赖几何；图表系列变化同时影响数据包和动画目标。连接线、避让及容器约束使相邻对象也可能失效，不能固定假设只重算一框。

CompiledPlan 包含：原生对象描述、解析样式、文本逻辑到字形映射、布局框与视觉边界、时间求值图、合成依赖、PPTX 输出绑定和来源保留计划。所有派生对象反查文档对象；诊断不依赖不稳定的绘图命令序号。

## 6. Agent 接口族

| 接口族 | 主要输入/输出 | 边界 |
| --- | --- | --- |
| capabilities.describe | schema/能力/限制/字体/宿主设施版本 | 区分内核支持与当前宿主可用 |
| document.create/open/inspect | 文档或资源句柄；revision、语义与导入报告 | 默认不解析外部链接、不执行内容 |
| document.transact/diff | 基础版本、类型操作；新版本、差异、失效集合 | 不执行任意脚本或 JSON 路径写入 |
| resources.requirements | 固定 revision；资源/权限/解码需求 | 只提出需求，宿主决定获取 |
| compile/diagnose | revision、profile；plan、对象级证据 | 不自动修改内容 |
| render.sample / playback.* | 明确状态/事件/时间；帧与媒体意图 | 不持久提交文档 |
| export.pptx / verify.package | 输出句柄；摘要、结构/语义报告 | 不上传、不覆盖用户文件 |
| template.* | 文档、参数和约束；实例/验证结果 | 模型推理与模板任务编排由接入方负责 |

大结果使用句柄、分页查询和按对象裁剪；Agent 可以查询“本次变更的诊断”而非每次传递整套文稿。结构化操作和高层布局意图并存；自然语言解释由 SDK/产品生成，不让核心发起模型请求。

## 7. 诊断与可恢复失败

诊断至少有 code、severity、stage、object/subobject、resource、source part/path、documentRevision、timeRange/eventCursor、observed/expected、suggestedOperations、evidenceRefs。建议操作不自动执行；布局证据与 PPTX 重读证据分开标记。

错误分类：输入无效、语义/引用冲突、资源未就绪、当前宿主能力不足、格式无法无损表达、预算超限、取消、内部错误。资源未就绪可继续同一计算游标；文档 revision 变化必须重新建立依赖。损坏输入不能进入“无限重试等待资源”。

例如针对动画中的文字溢出，返回对象、触发事件和时间区间；针对外部未知扩展编辑冲突，返回受影响源部件与可保留范围。Agent 能据此修订请求，不需要猜测一条模糊的“渲染失败”。

## 8. 版本和验收

文档 Schema、协议、兼容规则、渲染算法、资源包分别版本化；能力查询公布组合。升级转换必须是显式纯转换，产生报告和新 revision，保留原 bytes；不能重解释历史摘要或修改旧 Artifact。

本规格的关键验收是：继承不压平；变更原子；引用无悬空；相同输入跨端得到相同语义与布局；增量结果等价于全量；来源未知内容仍可追踪；播放临时状态不污染文档。具体用例见[验证规格](verification-and-roadmap.md)。

组合的 `Appearance.stroke` 描述对象自身，不隐式修改后代样式。当前原生组的 Inherit/None 均投影为无线条声明，作者身份与原始声明仍区分二者；普通形状的显式 None 保留原生 noFill。可见组合描边的完整语义映射继续开放，见[实现和验收边界](../../implementation/group-export.md)。
