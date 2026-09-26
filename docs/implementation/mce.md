# OOXML 兼容内容处理

2026-09-24 · 已实现公共 XML 逻辑投影并接入 PPTX 来源读取/叶文本修改；完整标准覆盖和多分支协调编辑仍待完成。

## 依据与处理范围

实现依据为 [ECMA-376 Part 3 第五版（2015）](https://ecma-international.org/wp-content/uploads/ECMA-376-3_5th_edition_december_2015.zip) §7–9。使用命名空间 URI 判断能力，不依赖前缀拼写；前缀从声明所在节点的词法作用域解析。文档不能自行增加内核理解的命名空间。

`mo-xml::mce` 提供独立于演示领域的流式处理器。固定应用 profile 指定理解的命名空间和暂停兼容处理的扩展元素。当前 PPTX profile 为 `musteroffice.pml-source-mce/1`：PML/DML/关系的 Transitional 命名空间、XML 命名空间和无命名空间；`p:extLst`、`a:extLst` 及其完整内容原样透传。此 profile 只说明兼容预处理选择，不意味着 PML/DML 所有对象已经实现。

| 构造 | 实际处理 |
| --- | --- |
| `AlternateContent` | 至少一个 Choice，后接至多一个 Fallback；可作为物理 XML 根；选择第一个 Requires 全部被理解的分支，否则选择 fallback，没有匹配则不输出分支内容 |
| `Ignorable` | 按 URI 继承；未理解且可忽略的元素丢弃整个逻辑子树，相应属性从逻辑事件移除；原始 XML 不变 |
| `ProcessContent` | 按展开名称或命名空间通配符继承；未理解且可忽略的包装节点不输出，但继续处理其子内容；禁止被展开包装上的 xml:base/lang/space |
| `MustUnderstand` | 对实际处理内容检查；不理解则返回兼容错误；未选分支的后代不会触发能力要求 |
| 应用扩展区 | 扩展元素本身及后代暂停 MCE 处理，不擅自解释其中的兼容属性或删除内容 |
| 无可忽略声明的未知命名空间 | 通用 MCE 层允许透传，交给所属领域判断；不是自动接受所有未知语义 |

错误的分支顺序、空 Requires、未绑定前缀、非法控制属性和不合法的 ProcessContent 声明被拒绝。未实现的 MCE 属性明确报告兼容错误；没有假装覆盖旧版扩展构造。未选/忽略的完整子树仍受底层 XML 语法、字节、节点、深度与取消限制，但本实现不声称对其内部所有 MCE 语义作全量标准认证。

## 逻辑投影与原始定位

处理器同时提供两种事件：每个原始物理元素的观察事件，以及处理后的逻辑内容事件。逻辑事件保留物理 Start 序号、当前选中兼容分支祖先及扩展区标记。节点深度与原始序号分别计算；选支或展开节点不会把后续编辑指向错误的 XML 位置。

投影从不序列化或覆盖源 XML。PPTX 读取器使用逻辑事件建立对象索引，使用同一事件携带的物理序号建立私有编辑绑定。叶文本修改仍由 XML 语法和旧值条件验证，其他节点、所有未选分支及未修改压缩条目保持原字节。每个新候选重新进行兼容投影并核对预期对象内容。

继承规则使用共享不可变作用域；普通节点复用作用域引用，仅有规则声明时分配新作用域。无需先复制完整 XML 树；单部件字节缓冲、底层 XML 限额和来源对象限额仍然存在。尚无高负载性能证据，不能据此宣布性能目标达成。

## 来源合同与编辑条件

`SourceIndex` 返回 `compatibilityProfile` 和主部件 `mainCompatibility`；每个 surface 返回 `compatibility`。记录包括原始 AlternateContent/Choice/Fallback 序号、Requires URI、是否选中，以及忽略/展开计数。`ignoredElements` 计数的是被丢弃子树的根节点，不是所有后代数；`ignoredAttributes` 不包含被移除的 MCE 控制属性。

`SourceRun.editConstraint` 明确返回当前约束：`compatibilityBranch`、`structuredLeaf`、`dynamicField` 或 `timingReferences`。分支外普通叶文本可修改；分支内部不能仅修改选中表示后宣称其他表示也已同步。即使逻辑文本是叶节点，只要原始文本节点含物理子元素，也不允许通过覆盖把它们删除。

原始观察事件会检查未选分支中的 `p:timing`。在文本范围 retarget 实现前，该部件所有文本修改受阻，避免别的应用选择另一分支后使用失效范围。无实际变化的普通文本请求或空编辑列表仍可逐字节返回源包；动态字段本身不具有普通文本修改绑定。

## 实际证据与后续工作

MCE 阶段新增 7 项 XML 测试和 2 项 PPTX 集成测试，当时全仓 70 项 Rust 测试、67 组来源 Native/WASM 对比通过，使用 21 份合成包和 1 份自有 LibreOffice 重存文件，另有独立 lxml 树投影和 ZIP 字节保留检查。可复现入口见[来源编辑说明](pptx-source.md)，该阶段源码/构建/语料摘要见 [MCE 历史证据](../reviews/evidence/2026-09-24-pptx-mce-verification.json)；后续继承实现与当前计数见[来源继承](source-inheritance.md)。

本轮没有新增外部运行依赖。标准 PDF、原始下载和开发中间文件留在忽略缓存，不进入发行包。

下一阶段需要把来源绑定提升到完整文稿模型，协调多种兼容表示的编辑、动画范围 retarget、Strict 与新版扩展能力，并让主题/版式、字体、布局和导出消费同一份语义结果。当前不代表完整 Office/WPS 互操作或 Musterwork 替换条件达成。
