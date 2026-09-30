# 从原生图表缓存到圆形绘图几何

本项把[图表来源查询](source-charts.md)、[布局声明](source-chart-layout.md)与
[有界曲线路径](chart-sector-geometry.md)连接。实际 PPTX 中的对象关系、图表部件、
系列、缓存点、角度和圆孔比例进入同一条原生/WASM 计算链。它是源图表的几何阶段，
尚不构成整张图表或模板的渲染准入；Musterwork 当前人工测试候选保持固定。

## 语义与来源

`source-cache-declared-circular-plot-v1-draft` 接受文稿 SHA-256、原生对象 ID、
绘图类型的物理 XML 序号，以及已经明确的绘图区中心、外半径和坐标容差。
中心和半径由调用者明确提供，不能把整个 graphicFrame 的矩形直接视为绘图区。
自动布局、标题、图例和数据标签占用的空间还需后续计算。

当前编译原生二维 `pieChart` 与 `doughnutChart`。饼图要求一个系列；环形图可有多个。
按系列 `order` 排序，并保留与其不同的稳定 `idx` 和物理 XML 序号。当前草案布局将
升序系列排列为从内到外的等宽圆环；该排列尚未经过 Office/WPS 多系列视觉校准，
不能视为应用互操作验收结果。所有圆环共用同一半径分界计算。

首扇区角度要求源文件显式声明，以十二点方向为起点、顺时针旋转。
[Microsoft 的属性说明](https://learn.microsoft.com/en-us/previous-versions/office/developer/office-2003/aa168774(v=office.11))
说明了该方向。环形图还要求显式 `holeSize`，支持整数及整数百分号拼写，范围为
10–90；[Open XML 文档](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.charts.holesize?view=openxml-3.0.1)
定义了百分比范围，[Office 实现说明](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oe376/95e4d4d3-ab4f-4218-8d9c-b32e0d804721)
另要求 `val` 属性。缺失属性仍返回有来源位置的未解析错误，后续须补齐标准和目标应用的默认值语义。

数据权威明确为 `sourceCacheSnapshot`。数值缓存必须有计数、连续且唯一的点索引，
原始十进制值进入精确比例计算；缓存物理顺序可不同于索引顺序。响应保留公式、
点的来源序号和格式代码，以及外部/嵌入工作簿引用。本阶段不打开或重算工作簿，
不宣称缓存与工作簿相符。缺值、稀疏索引、空串、错误值不会被改写成零。

系列和数据点的样式节点、标签节点、显式几何属性及未知子节点现在由来源查询保留。
不透明扩展根节点也保留物理位置，而不是在跳过扩展正文后误判为没有扩展。
非零或未解析的分离距离、三维点、未知几何和不透明扩展均阻止本阶段编译。
完整样式、标签、图例、默认值、爆炸图和其他图表类型仍属于既定一期目标。

## 精度、预算与接口

源文件的整数角度先经 Q96 区间转换为 Q32 圈单位；保留转换误差，并按外半径与
`2π` 换算为坐标界。圆孔及多系列半径的有理数分界也保留舍入界。源几何误差从
后续路径容差中扣除，最终报告它与路径误差之和。正宽度圆环若小到端点重合，则
返回精度失败，不丢弃该系列。

全部系列共享路径、命令、弧段、计算阶段和十进制预算，不在每个圆环重置。
来源查询另限制部件、XML、元数据及缓存点；数据点样式覆盖也计入点预算。
取消、预算和解析失败都不发布部分系列结果。输入源文件始终保持不变。

入口为 Rust `mo_kernel_api::compile_pptx_chart_geometry`（自带预算与取消回调）、
`compile_pptx_chart_geometry_json`、同名 WASM 导出，以及 CLI：

```sh
mo-cli compile-pptx-chart-geometry request.json source.pptx
```

JSON Schema 与 TS 类型从 Rust 生成；坐标为 Q32 EMU 字符串，严格拒绝未知字段。
它属于无界面计算 API；本批没有新增正式 MCP 能力声明或变更产品模板准入。

## 本批验证

7 项源几何编译测试、17 项图表来源测试、1 项公开 API 测试通过；覆盖缓存身份、
样式绑定、多系列共享预算、精度、稀疏值、错误值、扩展和取消。严格 Clippy、生成
合同和 TS 类型检查通过。另执行共享编译器库测试，具体记录见证据文件。

45 份实际源文件请求在原生/WASM 中完整响应一致：22 份成功、23 份预期失败，
全部请求与响应通过生成 Schema。成功项包括两个真实私有模板的圆环图；没有将
私有模板或其正文迁入公开测试目录。14 份既有来源查询也完成完整跨端回归。

9 帧实际 Skia 输出的全部像素及元数据一致，并传播几何与设备坐标量化的合计界。
独立程序直接解析 PPTX 的关系和图表 XML，用精确有理数与 90 位 Decimal 核对
22 个绘图、29 个系列、84 个数据点、3164 段曲线的 28,476 个采样位置；另检查
72 处扇区内部颜色、圆孔及外部透明像素。移位控制点、伪造来源误差、修改来源角度、
修改圆环半径四类反例均被独立程序拒绝。采样是回归证据，不代替解析误差推导。

查看实际诊断图后，多系列相邻圆环仍可见抗锯齿覆盖边界；后续绘制合成必须处理。
这些图使用固定诊断配色与视口，尚未应用源主题、描边、标签或图例，因此不代表
真实模板外观。全像素一致也不等于目标应用的视觉质量已通过。

复现工具为 [`source-circular-fixtures.py`](../../tools/verification/source-circular-fixtures.py)、
[`source-circular-parity.mjs`](../../tools/verification/source-circular-parity.mjs) 和
[`source-circular-oracle.py`](../../tools/verification/source-circular-oracle.py)。前者在新目录
生成自有 PPTX 与请求清单；跨端工具的配置需给出 CLI、worker、WASM、Skia 组件和
案例路径，独立程序复用该配置读取原始源文件。版本、产物哈希及逐案例摘要见
[验证证据](../reviews/evidence/2026-09-30-source-circular-geometry.json)。

## 后续验收

下一步接入图表主题与数据点绘制样式、标签/图例文本及绘图区布局，再完成整个
graphicFrame 的页面合成。不能解除当前整页准入约束来显示残缺的模板。原生编辑、
工作簿数据同步、其他图表类型、多系列目标应用校准、动画与 Office/WPS 往返仍要
分别完成；本批不是完整 PPT 替换或性能/安装包体积结论。
