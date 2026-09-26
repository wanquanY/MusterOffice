# 原生字符间距与共享笔位

2026-09-25，开发态实现。在[段落百分比间距](paragraph-spacing.md)之上，原生字符 `spc` 已贯通来源继承、字体塑形、候选断行、字形位置及下划线/删除线绘制。正负值均参与计算，原文字、属性词法和来源位置继续保留。

## 来源与布局策略

固定 ECMA-376 Part 1 §21.1.2.3.9 的 `spc` 整数单位为 1/100 pt，未声明时为零；来源模型同时保留单位小数。编译器支持 pt、pc/pi、mm、cm、in，通过精确有理数一次舍入到 Q32 EMU，最近值、半值远离零。最大 256 字节词法和 i128 数值范围分别检查，超限不调用组件。`100` 与 `1pt` 均精确等于 12700 EMU。

当前 draft 采用塑形字簇的视觉尾部间距：同一字簇的全部字形完成后追加一次间距，包括行末字簇；不在基字符、附加符号或必需连字内部插入间距。HarfBuzz cluster 与 Unicode 字簇并非同一概念，参见[官方字簇说明](https://harfbuzz.github.io/clusters.html)。非零原生字距关闭 `liga`、`clig` 可选连字，保留 `rlig`、组合、附加符号和脚本塑形。原生极小非零值即使舍入为零，也保留该塑形选择。

此策略尚需 Office/WPS 核验，尤其是行末字距、连字选择和复杂脚本外观。强负字距允许笔位反向；计算保留自然 advance 端点和追加间距后的端点，不把负值钳制成零。

## 共用计算与合同

`mo-text::geometry::FragmentPen` 维护一个字体片段的精确设计单位前缀和 Q32 间距前缀。每个字形提供原点、原笔位、自然 advance 端点和追加间距后的端点；溢出时不提交该步状态。候选断行、实际字形放置和装饰线共用这套计算，避免断行后才移动字形或只改变文字而遗漏下划线。路径仍引用原字体轮廓。

计算按字形线性推进，没有逐字形重新扫描前缀、分配独立堆对象或复制字体资源。整个断行搜索的候选数量和最坏复杂度仍受既有策略及预算约束；局部线性算法不等于整篇性能验收。

`GeometryStyle` 新增可省略的 `clusterSpacing`，为 signed Q32 EMU 规范整数字符串，默认零且零值不序列化。例如追加 12700 EMU：

```json
{"fontSize":"381000","baselineShift":"0","clusterSpacing":"54546084659200"}
```

低层调用者仍自行提供 OpenType features；只有原生来源编译器按上述 `spc` 策略选择可选连字。不同间距参与样式比较，等值有效样式可以合并而保留独立原始 run。既有不带该字段的请求保持原语义和响应。3 份请求 Schema/TS 同源更新：行几何、段落布局、段落路径。Rust 构造方需补 `cluster_spacing: Fixed::ZERO` 或目标值。

`SourceParagraphPlan.trackingConversionError` 在样式合并前记录分数转换误差。存在该误差时，页面保守增加 `2G + 2` raw Q32，其中 G 是文本框的字形记录总数，覆盖前缀、对齐及装饰线；原有基线和段落间距误差独立累加。该界限相对已选字体输出和断行拓扑，不覆盖目标应用差异。

## 实际验证

[冻结证据](../reviews/evidence/2026-09-25-character-spacing-verification.json)绑定源码、构建、输入与结果：

- 499 项 Rust 测试通过，新增 7 项；严格 Clippy、格式、70 份 Schema/TS 检查通过。
- 18 个新页面完成 Native/WASM/CLI 对照：15 个成功、3 个预检失败，覆盖正负/小数/极小/零字距、继承、等值单位、混合样式、换行、居中/右对齐、组合符号、双向文字、装饰线及必需连字。
- 重放 106 个前期页面、307 个更早请求及 24 个前期布局请求；新增 17 个布局请求。本阶段实际双端执行合计 472 个，旧响应与成功像素不变。
- 布局输入另核对默认零值兼容、负向笔位、分数、字簇、无文字，以及类型、范围、非规范数值和重复键拒绝。上述输入错误不调用组件；可表示的极大值在实际计算中溢出时返回数值限额错误，不发布部分布局。随后成功请求仍使用同一实例。
- 独立 XML / FontTools / Fraction 检查 15 页、19 个原生字距声明、33 个样式值、45 个字体度量值、16 行、74 个字形坐标和 2 个装饰矩形。比较 1,725,584 个内部像素，排除 74,416 个边缘像素。
- 独立布局数学核对 25 份布局、26 行、44 个字形记录及 321 个坐标值；Q32 计算一致，样本整数输出最大误差 1/2 EMU。

新增 `owned-tracking.ttf` 是由本仓库原创轮廓生成的 2572 字节测试字体，具有明确的可选及必需 GSUB 连字，实际验证塑形行为。它不是运行时字体包，也不代表语言视觉质量。旧字体、运行依赖和 11 个历史发行产物不变。

```sh
PYTHONPATH=.codex-work/font-tools-venv/lib/python3.13/site-packages python3 tools/verification/tracking-font-fixtures.py
MO_TRACKING_EVIDENCE_DIR="$PWD/.codex-work/character-spacing/cases" cargo test --workspace --locked
PYTHONPATH=.codex-work/font-tools-venv/lib/python3.13/site-packages python3 tools/verification/character-spacing-reference.py
node tools/verification/character-spacing-parity.mjs
node tools/verification/character-spacing-geometry.mjs
python3 tools/verification/character-spacing-numerics.py
node tools/verification/text-page-runtime-regressions.mjs .codex-work/character-spacing
python3 tools/verification/character-spacing-evidence.py
```

按[开发说明](development.md)构建当前 workers 和独立 `.codex-work/character-spacing/wasm-node`，保留旧输入及结果。参考使用原创轮廓和独立来源数值，已选字形和断行拓扑仍是输入，像素核对排除 2 像素边缘。没有新增真实字体、多语言质量、抗锯齿、Office/WPS 打开编辑保存或产品性能/安装包体积结论。完整排版、高级内容及 Musterwork 接入仍在推进。
