# 原生表格边框来源计算

本阶段将物理单元格的六条边、样式的八类边框、主题线条矩阵与共享线条／填充计算接通。它提供可追溯的边框属性和网格边缘身份，供后续整页绘制使用；不是表格像素或 Office/WPS 兼容验收。

## 计算与边界

`SourceTableBorderQuery` 使用明确来源摘要、部件、表格 native ID、物理行列地址和边缘。公共 Rust Kernel、CLI `pptx-table-borders` 与 WASM `resolve_pptx_table_borders` 共用这一计算，返回 `stroke`、`fill` 和 `topology`。不存在页面、账户、存储或网络依赖。宿主继续负责输入字节和结果提交。

线条几何属性从原对象线条的合并器中抽取，保持线宽、端点、复合线、对齐、虚线、连接、箭头及默认值的同一计算。表格的线条填充调用已有通用填充／颜色引擎，覆盖无填充、纯色、渐变和图案，不受旧对象线条查询的纯色返回结构限制。`FillTarget` 增加 `tableCellBorder` 和 `tableStyleBorder`，后者表示真实样式区域中的边框声明，也是延迟求值 `lnRef` 颜色上下文的身份。

每批只构建一次相关表格的有效网格和样式绑定；颜色与描边读取同一绑定，使用各自整批预算及取消检查。单元格直接属性优先，其后按十三个样式区域逐项继承；主题引用使用实际 `lnStyleLst` 项，`idx=0` 不凭空生成无填充。直接 `noFill` 保持明确无填充。共享样式的原始 XML 部件、引用位置、属性位置和 `phClr` 的工作精度上下文全部保留。未知表格声明、相关线条声明、缺失样式、无效网格、摘要不符、取消及预算超限保持明确诊断。

`topology.position` 是声明网格中的边缘编号，不是设备坐标，也不代表最终可见笔画。相邻两侧指向同一水平或垂直边段，但分别保留各自声明。RTL 交换左右的物理邻居与编号；合并区域内的边段标记 `insideMerge`，覆盖单元格的原始内容继续可查询，不在这里删除。绘制合并区域的外框、忽略内部边或选择冲突的胜方仍属于后续绘制策略。

区域内部的边采用 `insideH`／`insideV`，区域外缘采用相应四边；由边缘两侧是否属于同一区域判定，分离的条带不被误合为一个连续区域。原生声明的定义见 [TableCellBorders](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.tablecellborders?view=openxml-3.0.1)，区域优先顺序见 [MS-OI29500 2.1.1265](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/90de4085-5fbf-47a7-bc37-d59509481d0a)。本阶段的区域邻接解释属于草案计算协议，仍须 Office/WPS 校准，不能借用 Word 表格的边框冲突规则作为演示文稿结论。直接单元格的 `lnBlToTr` 与样式的 `tr2bl` 保留不同来源身份；非对称端点和虚线方向的最终绘制尚未验收。

## 验证与后续

源码测试见 [table_borders.rs](../../crates/mo-pptx/tests/table_borders.rs)。[公共跨端核对器](../../tools/verification/native-table-border-parity.mjs)比较 Rust 库、原生 CLI、WASM 及通用填充入口；[XML 核对器](../../tools/verification/native-table-border-readback.py)从原文件独立定位属性和主题引用、重建合并网格与 RTL 邻接，并检查 XSD 和公开合同。最终运行结果、尝试及源码摘要记录在[本阶段证据](../reviews/evidence/2026-09-29-table-borders-verification.json)。

438 项相关 Rust 测试、严格 Clippy、工作区全部目标、Schema／TS 和 Native／WASM 构建通过。8 份自有 PPTX 经公共接口完成 29 次跨端对照，包含 432 个边框目标；独立检查 5,351 次来源定位、116 次占位颜色引用、1,224 个实际声明的线条属性、432 个拓扑、52 个 XSD 部件和 58 次 JSON Schema。来源定位计数包含重复访问，不代表不同 XML 节点数。另重放 144 个普通线条、247 个线条颜色、572 个填充颜色案例，共 1,926 次跨端与冻结返回字节的比较，全部保持。

下一步完成边框冲突与方向校准，以及共享描边绘制对全部样式的支持，再把真实表格背景、单元格填充、边框和多个文字框接入同一静态／保留式页面管线。还需合并行高计算、整体预算、Native/WASM 像素和 Office/WPS 原生编辑往返。Musterwork 的 SDK、导出 Worker、播放材料仍需在这些路径完成后于独立集成 worktree 一起升级；一期完整能力及替换验收目标继续开放。
