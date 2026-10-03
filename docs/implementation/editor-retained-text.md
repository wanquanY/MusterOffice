# 保留型 PPT 的文字范围命令与能力查询

状态：内核计算增量；共享编辑器、固定产品 SDK 与整体产品验收仍在进行。

## 原生范围与保留边界

`prepare_text_edit` / `KernelRequest.prepareText` / `PresentationEditor.prepareText` 接受
RetainedSource 对象中真实 ParagraphId 的选区。同一段落可跨多个原生 Text 片段替换；
端点必须为内核 Unicode 字素边界，倒选区先归一化，插入样式由起点 affinity 选择原片段。
段落文字中原生 Break 计一个 U+2028 scalar；字段保留实际显示文字。不将原生内容转换为自建 TextBody。

内核按片段从后向前展开普通 SpliceText 事务，保留全部 ParagraphId、RunId、片段种类、原始空片段、
源绑定、关系与资源。删除文字可以把片段变空，但不移除其结构。事务身份绑定完整高层命令摘要与
原片段序号；重复计算产生相同候选，宿主继续负责权限、历史归属与 CAS 提交。

只有具备可写源绑定的 Text 片段可被修改。字段、Break、动态/计时/兼容分支/其他保留约束均返回
结构化限制。跨段落替换、初始化、局部字符样式，以及插入 CR/LF/U+2028/U+2029 需要协调原生
结构和格式保留，当前返回 nativeStructureRequired 或 retainedParagraphBoundary。
Tab 可插入原生文字叶；能否渲染仍取决于[实际制表位支持](editor-tab-layout.md)，能力查询不保证排版成功。

原生表格段落沿既有 source profile 投影。调用者使用公开页面返回的 ParagraphId，不能从物理单元格
索引合成模型身份；保留模型的 cell 参数不接受自建 CellId。旧 profile 不被暗中升级。

## 选区与普通事务

`rangeChange` 表示用户一次完整替换的语义映射，宿主对已有选区应用一次。
`receipt.changes.anchorMaps` 保持普通事务中逐个原生拼接的映射，供底层事务语义使用；二者不能
叠加，也不能假设删除范围内部的逐次映射与一次范围替换相同。历史继续由原普通事务重新计算，
不改写回执以伪装成另一种事务。反向范围映射交换前后范围。

返回 selection 根据替换后的完整段落重新分割，在组合字符跨原片段连接后仍位于有效字素边界；
rangeChange 保留精确 scalar 坐标。每段最多 262,144 UTF-8 字节、65,536 原生片段，展开事务遵守
10,000 操作上限；修改后完整段落也执行分割预算。取消、旧 revision、非法 XML 字符及整图限制
在返回候选前失败，不发布部分结果。

## 能力查询合同

`text_capabilities` / `KernelRequest.textCapabilities` / `PresentationEditor.textCapabilities`
接收 Snapshot 与 object、可选 cell/selection，返回绑定文稿和 revision 的 initialize、replace、delete、
characterStyle 可用性及 replacementPolicy。查询复用实际执行的目标与范围检查，不创建候选或新 revision。
公共边界恢复并核验 Snapshot；篡改摘要、错误目标/选区和未知字段仍是错误，不包装成普通禁用菜单。

每项能力为 available 或 unavailable + TextEditRestriction。实际执行的同类失败在原 INPUT_INVALID
诊断内携带 textRestriction，产品不需解析错误文字。无可用选区、没有文字体、覆盖单元格、非文字对象、
受保护原生片段与结构限制可分别解释。没有文字体的自建形状/可见单元格只开放显式初始化。

replace 和 delete 分开：选区起点之前的受保护字段可能是 Before affinity 的插入样式所属片段，
此时非空替换不可用，而仅删除选中的可写文字仍可用。空折叠替换沿普通零长度 splice 语义，需要合法插入片段。
能力结果只描述给定模型/选区前提，不授予编辑权限，不保证任意输入字数、字体、源材料或渲染通过。

## 验证与剩余工作

Rust 覆盖 Unicode/倒选区/空片段/affinity、全部原生限制、字段与软换行、结构与非法范围、取消、
预算、修订和普通撤销重做；真实 PPTX 测试保留其他部件、关系和未知扩展，撤销后字节恢复。
公开 Kernel、Native CLI、WASM 与 TS 比较能力、诊断、事务、PPTX 导出/导入，以及原有页面重准备链路。
执行结果、运行程序与源状态见[本批证据](../reviews/evidence/2026-10-03-editor-retained-text.json)。

原生结构/格式编辑、完整对象能力矩阵、共享 UI/IME、独立放映窗口、新固定 SDK 和真实浏览器、
Electron、Windows/Linux、Office/WPS 与性能验收继续实施。本计算增量不代表完整产品可合并验收。
