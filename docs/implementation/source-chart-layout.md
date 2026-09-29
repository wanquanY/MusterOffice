# 原生图表坐标轴与布局声明

2026-09-30。状态：来源读取、公开 Native/WASM 查询与局部验证完成，**不代表图表已经
能够绘制、编辑或准入模板库**。Musterwork 早晨候选仍固定此前 `a86266c` 运行内核，
本批来源查询未重新激活到产品。

在[图表系列、缓存与工作簿引用](source-charts.md)之上，`SourceChartPart.axes` 按原始
来源顺序返回分类、数值、日期、系列四类轴；同一图表部件内轴 ID 必须唯一。共享部件
仍只读取一次，查询最多累计 4096 个轴，并受既有 XML 元素、元数据、部件字节和取消预算
约束。不会根据 `axId` 或 `crossAx` 引用凭空创建轴，也不静默移除不存在的引用。

`SourceChartPlot.layout` 与轴的 `layout` 返回有类型名称的属性声明，以及原 XML 中
复杂样式节点的物理序号。当前属性包括柱方向、分组、按点配色、间距和重叠、环孔与首段
角度；轴的位置、显隐、刻度、标签、交叉方式、单位和分类/日期选项；缩放的方向、上下界
及对数底。轴的数值格式单独保留 `formatCode` 与 `sourceLinked`。标题、字体、线条、
网格、单位、数据标签及系列线的复杂标记仅绑定来源，尚未求值和绘制。

每个声明同时保留原始 `sourceOrdinal` 和 XML 属性值：属性节点不存在、节点存在但
没有 `val`、显式词法值是三种状态。不会把缺省删除属性当成 `false`，也不会提前把
`1.0000000000000001`、指数写法、负零或未知枚举转换为浮点数或推断默认值。这里的
`layout` 是**来源布局声明**，不是已计算的坐标；数值域、轴关系一致性、默认规则和完整
XSD 验证仍属于后续语义处理，不能把读取成功当作输入完全有效。

读取仅接受当前位置的原生命名空间和直接子节点，沿用同一 MCE 投影。未选择的分支、
扩展内容及外来同名标签不伪造声明。单值声明、轴 ID 和单例复杂节点的重复会失败；
标准允许重复的 `serLines` 保持物理顺序。标量包含子内容时不会被当作普通属性接收。
没有新依赖、网络、文件权限、持久存储或 UI。

字段位置与结构参照官方 [ValueAxis](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.charts.valueaxis?view=openxml-3.0.1)、
[Scaling](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.charts.scaling?view=openxml-3.0.1) 和
[DoughnutChart](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.charts.doughnutchart?view=openxml-3.0.1)，
以及既有 ECMA-376 Transitional XSD 固定材料。本次没有采用任何商业图表 SDK。

## 验证与实际模板发现

- 六项新来源测试、九项原图表测试、两项公共 API 测试通过。覆盖全部轴种类、原始数值、
  三种缺省状态、MCE、错误命名空间、重复项、缺失引用、预算与不虚构默认值。
- 严格 scoped Clippy、Rust 格式、自动生成 Schema/TS 及 TS 编译通过。
- 15 份完整 Native/WASM 响应相同，含四份实际图表查询、所有新增自有正负例及 MCE
  fallback。剔除新增加的 `axes` 和 `layout` 字段后，六份旧响应逐字段不变。
- [独立 XML 检查](../../tools/verification/pptx-chart-layout-check.py)使用另一套 XML 读取器，
  对 14 份正负输入核对物理序号、父节点、轴类型/ID、数值格式及 127 条属性/复杂标记引用。
  MCE 样本另做选择结果断言和跨端比较，不冒充未投影 XML 的独立全量比较。
- 两份实际平台模板各有柱状图和环形图。独立 XSD 检查发现，两份柱状图均存在数据点样式
  `dPt` 的顺序错误，并列了三条轴引用、实际仅定义两条轴，其中一条引用不能解析。
  这也不符合原生二维柱状图的两条 `axId` 结构。两份环形图部件通过该 XSD 检查。
  **部件 XSD 通过也不证明整份文稿可渲染或 Office/WPS 往返通过。**

私有模板没有迁入源码，原始输入未修改。本阶段没有通过忽略缺失轴、清除数据点样式、
删除图表或降级渲染来发布模板。下一阶段须分别处理明确的输入规范化、缓存/工作簿数据
一致性、计算布局/样式及原生绘制，并保存修复前后的来源摘要与视觉证据。

证据：[来源布局与实际模板验证](../reviews/evidence/2026-09-30-source-chart-layout.json)。
复现自有输入可运行 `MUSTER_OFFICE_CHART_LAYOUT_FIXTURES=<新的已有目录>` 下的
`cargo test -p mo-presentation-source --test chart_layout`；每个输入目录包含源 PPTX、
查询及预期状态，再交给现有 `pptx-chart-parity.mjs` 和独立 XML 检查器。测试材料内的工作簿
占位字节只用于来源引用测试，不是有效 XLSX，也不计入工作簿验收。
