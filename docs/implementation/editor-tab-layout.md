# 编辑文字的原生制表位排版

状态：显式左制表位计算已实现；高级制表位与 Office/WPS 互操作尚未验收。

## 统一计算路径

制表符保留一个 Unicode scalar 和自身 RunId，不替换成空格，也不请求字体提供 Tab 字形。
`LeftTabStops` 明确传入递增位置、正的默认间距、首行和后续行的起点偏移。
候选行和最终排版共用 `geometry::order::place_line`：将 Tab 与塑形片段按同一 bidi
visual order 排序，直接在 Q32 pen 上推进。换行判断、后续 glyph 原点与最终行边界因此一致。

当前左制表位 profile 选择严格位于当前位置之后的显式位置；显式列表耗尽后，使用以左边距
为原点的默认等距网格。首行缩进纳入当前位置，后续行不继承首行偏移；负缩进使用欧几里得除法。
不按字形宽度估算间距，不逐步累加已舍入坐标。显式列表最多 32 项，保留既有行候选、scalar、
字体、几何和取消预算。无 Tab 的请求省略新字段，既有输出保持不变。

Tab 的字体样式参与行高和基线，使用该样式的显式主字体度量；它没有字形塑形或字体回退探测。
实际布局保留 Tab 两端，交互 map 使用 `tabEdges`，由原光标、选区、命中和视觉移动计算消费。
空白 Tab 区域有真实宽度；它不再落入没有可见推进的控制符邻接位置。

## DrawingML 接入及边界

源编译器读取级联后的 `defTabSz` 和 `tabLst`，使用实际左边距及首行缩进。
已有 Office 默认 profile 的 `914400` EMU 不变；[微软实现说明](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oe376/e05f6e35-a35d-4f00-924f-8172fe339bf5)
说明了该默认值。[DrawingML TabStop 文档](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.tabstop?view=openxml-3.0.1)
定义位置相对左边距和按递增顺序使用。

此增量接入水平、LTR、左对齐段落的默认和显式 `algn="l"` 制表位；缺失位置/对齐、
非递增列表、非正默认间距、无法精确转换的值仍返回来源诊断。RTL 段落、段落居中/右对齐、
居中/右/小数点制表位尚待实现，不能据此声称完整 DrawingML Tab 支持。
普通左对齐段落内仍按已有 Unicode bidi 规则排列双向文本。

Tab 的下划线/删除线尚未接入绘制跨度：源编译器明确拒绝这些组合，避免静默漏画。
底层几何请求没有格式默认值，未提供 tabs 时保留既有 Tab prerequisite 诊断。
这些边界不由产品端改成空格、丢弃格式或自行定位来规避。

## 验证

`mo-text/src/interaction/tab_tests.rs` 覆盖精确网格、缩进、换行/不换行、负字距、
双向文本、仅 Tab 行、不同字体样式、光标/选区/命中/移动和错误预算。
`mo-harfbuzz-sys/tests/text_frame_interaction.rs` 对比真实渲染与交互的同一 glyph 原点；
`source_frame.rs` 验证未支持属性在组件计算之前拒绝。
`tools/verification/editor-cell-text-parity.mjs` 使用实际编辑、初始化、渲染、公开查询与历史操作
比较 Native/WASM/TS；具体执行与依赖摘要见[单元格文字证据](../reviews/evidence/2026-10-03-editor-authored-cell-text.json)。
