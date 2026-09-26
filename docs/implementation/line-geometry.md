# 显式行的水平字形几何

后续合同已增加兼容的 Q32 基线偏移形式，并用于[原生 baseline 百分比绘制](native-baseline.md)；原有整数 EMU 输入仍有效。下文保留本阶段的历史范围与证据。

2026-09-24 · 文本 E0 进展。`layout-lines` / WASM `layout_lines` 把[行上下文重塑](line-shaping.md)和[实例度量](font-instance-metrics.md)连接起来，计算字号缩放、基线、行框、水平片段次序和字形原点。它消费显式行计划，没有自动选择断点或绘制页面。

## 输入与结果

`LineGeometryRequest` 包含既有 `LineShapeRequest`、每个 shaping style 对应的几何样式、明确的 `strutStyle` 和行距策略。字号和基线偏移均为 EMU int64 十进制字符串；字号必须大于零。正基线偏移把文字向上移动，输出坐标为 x 向右、y 向下。

几何样式参与原来的有效样式归并条件。语言、字体、feature 等相同但字号或基线偏移不同的片段不能合并成一个布局样式，避免连字和坐标失去其字号归属。作者文本、原始样式和逻辑坐标不会被改写。

结果保留完整 shaping 证据、实际查询的字体实例度量，以及可选的完整 `layout`。每个字形位置引用 `fallbackItem`、`fragment` 和片段内 glyph 序号；glyph ID、cluster、轴和原始 advance/offset 仍保存在 shaping 数组。布局不复制或重新塑造这些字形。

行的 `advance` / `advanceY` 是有符号笔位移，`penMin/penMax` 是水平 pen 经过的范围，均不是轮廓或可见像素边界。offset 影响字形原点而不改变 advance；HarfBuzz 的 y 向上位移在输出时取反。二维 pen 延续到下一字体片段，支持非零 y advance，不能在每个片段偷偷归零。轮廓、ink bounds、命中区域和绘制继续实现。

## 显式度量与行距策略

当前 profile 为 `horizontal-hb-metrics-strut-q32-emu-v1-draft`。度量取自实际字体实例的 HarfBuzz horizontal ascender、descender 和 line gap，不合成缺失值。它沿用 HarfBuzz 14.5.0 的表选择和 MVAR/avar 语义，不宣称等同于 Office/WPS。

每行至少使用 `strutStyle` 第一候选字体的真实度量，空行也如此。其他实际选中字体与样式参与计算：ascent 为各实例的缩放 ascender 加 baseline shift 的最大值；descent 为负 descender 减 baseline shift 的最大值；gap 取最大值。字体绑定/变化轴相同的度量请求复用一次结果，字号和偏移在 Rust 几何阶段应用。最多 2048 个唯一度量实例，同一字体每批最多 256 个。

- `natural`：行高为 ascent + descent + gap。
- `exact`：使用明确正行高，可小于内容高，不隐式夹紧。
- `atLeast`：取自然行高和明确正行高中的较大值。

基线在 ascent 上加一半的额外行距，下一行从上一行 bottom 开始。负 gap、负度量和基线偏移保留有符号运算；最终自然行高非正时给出诊断。显式正行高可为真实零度量的空行提供行框。行高不表示内容不会越界；溢出策略另行实现。

## 数值规则

内部使用带 32 个小数位的有符号 i128 EMU，不使用 JavaScript Number 或逐字浮点累计。字号、advance 和分母相乘相除时检查范围；先做整数商和余数分解，再扩展小数位，避免仅因中间分子左移而错误拒绝可表示结果。

每个字体片段先累计精确的设计单位前缀。查询字形原点或片段总位移时才按字号和 `positionUnitsPerEm` 转为内部精度。不会把每个 advance 先舍入成整数 EMU 再相加。例如当前自有字体 300 个 A 在 1 EMU 字号下总 advance 为 180 EMU，逐个整数舍入会得到错误结果。

内部缩放和 half-leading 均按最近值舍入，恰好一半时远离零；最终输出整数 EMU 时使用同样规则。跨片段/跨行累加内部精度，输出的 `top/bottom/height` 各自量化，调用方不能通过累加已量化 `height` 重建后续 `top`。

这是明确的 Q32 数值 profile，不是任意精度有理数最终一次舍入。极接近半 EMU 的边界可能发生双重舍入差异，不能把本轮样本的最大误差当成所有输入的通用上界。后续宽度选择必须使用明确的内部精度比较规则；不能用显示层整数坐标反推字形能否放入。

## 片段次序与失败语义

段落 bidi 仍在原文上解析，逐行执行 L1/L2。几何阶段以每个选中片段的首个视觉位置安排整个片段；HarfBuzz 已处理 RTL 字形序列和组合标记顺序，不再逐字反转 glyph 数组。全为 X9 移除 scalar 的片段记录于 `removedByX9`，不生成位置。扩展字簇内部混合 bidi 层级暂时产生明确诊断，不宣称完整 L3/L4 或竖排支持。

任何未解决字体、缺失必要度量、tab、混合层级字簇或非正自然行高，都使完整 `layout` 为 null，并保留 shaping、度量与问题范围。Tab 停靠位置尚未实现，不能把它悄悄当成零宽空格。底层组件失败、输入失败、数值范围溢出或取消则返回错误，不发布部分结果。度量失败同样可能发生在所有字形已塑形之后；内存失效的进程/模块必须由宿主更换。

塑形和度量使用同一批已验证字体，不重新校验或隐式获取字体。每次组件调用仍上传字体并创建其字体对象；持久实例和缓存尚未实现。既有标量、行、片段、塑形与字形累计预算继续生效，4096 行上限不承诺所有复杂内容都可计算。

## 验证与开发入口

[本轮证据](../reviews/evidence/2026-09-24-line-geometry-verification.json)绑定实际源码与产物：

- 35 批新 Native/WASM 返回字节一致，包含混合 UPEM、字号/偏移、RTL/隔离、标记、天城文、中文、Emoji、变体轴、空行、4096 行、超过 JS 精确整数范围的坐标及错误输入。
- 62 个片段、405 个字形和 108 项实例度量与固定同版上游独立入口一致。该对照共享 HarfBuzz 算法，不是独立排版器。
- 独立 Python Fraction 校验 22 份完整布局、4134 行、401 个已定位字形及 33896 个坐标；Q32 profile 精确相等，并对未量化有理数几何核对累计误差界。样本最大 wire 偏差为 1/2 EMU。其 L2 次序独立重建，但使用已验证 bidi 层级。
- 实际 WASM 故障注入在塑形成功后的度量阶段触发分配失败，无部分结果、拒绝失效复用、新实例恢复。它不是所有故障位置的穷举。
- 新增 9 项 Rust 测试，含负 advance、非零 y advance、offset、跨片段 pen、缺失与真实零度量、舍入、范围、样式边界、取消。当前总计 180 项；旧 1727 批语义结果不变，总计 1762 批；运行 Schema 为 34 份，旧 32 份不变。

```sh
node tools/verification/line-geometry-parity.mjs
python3 tools/verification/line-geometry-reference.py
python3 tools/verification/contracts.py --line-geometry-report .codex-work/line-geometry/parity.json
target/release/mo-cli layout-lines .codex-work/line-geometry/cli.json .codex-work/line-geometry/cli.bin
```

构建依照[开发说明](development.md)，使用已有固定字体、组件和故障注入产物。本轮没有新增外部依赖、字体数据或 C++ 实现。真实产物字节数记录于证据，仍不能推断完整内核或 Musterwork 安装包。

下一步连接默认断行机会和真实行宽测量，落实作者强制换行、字簇安全、候选重塑和明确溢出结果；随后推进 tab、连字符、对齐/分散、目标应用度量、竖排和实际绘制。自动行宽、Office/WPS 视觉互操作、高级对象、播放及 Musterwork 替换门禁仍未完成。

语义依据：[HarfBuzz glyph position 与 buffer 接口](https://harfbuzz.github.io/harfbuzz-hb-buffer.html)。
