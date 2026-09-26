# 原生段落百分比间距

2026-09-25，开发态实现。在[基线偏移](native-baseline.md)之后，原生 `lnSpc`、`spcBef`、`spcAft` 的点值和百分比已进入共享 Rust 布局、字形路径、Native / WASM 页面绘制及 CLI。保留来源继承和物理序号，未增加运行依赖或字体资源。

## 来源语义

固定 ECMA-376 Part 1 §21.1.2.2.5、§21.1.2.2.9–12 描述行距及段落间距；百分比以行内最大文字字号为基准。原生整数表示千分之一百分比，带 `%` 的十进制表示百分数，参见 [Microsoft 词法说明](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/4e043105-2ae4-4b85-baa8-4324ca7a6367)及[行内最大字号说明](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.spacingpercent?view=openxml-3.0.1)。本阶段规则如下：

| 输入 | 计算行为 |
| --- | --- |
| 显式百分比行距 | 每行实际文字项中最大的原生字号乘以百分比；不把未使用的段尾字号加入最大值 |
| 显式点值行距 | `spcPts` 的 1/100 pt 精确转换为 127 EMU；零值保留，允许行重叠 |
| 缺省行距 | 继续使用已有 Natural 度量配置，仍需目标应用核验 |
| 百分比段前 / 段后 | 分别使用第一行 / 最后一行的文字字号基准，在断行完成后求值 |
| 点值段前 / 段后 | 精确固定间距；首段前 / 末段后按 `spcFirstLastPara` 决定是否应用 |
| 无文字项的行 | 使用段尾插入样式（strut）的字号基准；显式双向控制项不扩大行距 |

零宽文字标记仍按文字项处理；不能根据字形是否有墨迹、是否有 advance 判断其排版身份。显式换行后的行分别求最大字号。原来的自然行度量仍负责基线位置，显式行距控制行高；字号与字形轮廓不因行距而缩放。顶、中、底锚点根据包含实际段前/段后间距的总高度计算。

这修正了早期将显式 100% 当作 Natural 的临时规则。已有旧语料没有触发这项区别，回归输出不变并不表示二者在所有输入上等价。段前/段后的边界行选择、空行基准、基线与行距之间的关系仍属于当前 draft profile，尚无 Office/WPS 排版验收结论。

## 共享合同与精度

`LineSpacing` 增加 `styleMaximum`。`heights` 的长度必须与几何样式数量相等，各值为非负 Q32 EMU 的规范整数字符串。例如：

```json
{"kind":"styleMaximum","heights":["429496729600000","858993459200000"]}
```

示例对应两个样式的 100000 和 200000 EMU 高度。文字行选择该行文字项的最大值，空行和仅含控制项的行使用 `strutStyle`。不同高度的几何样式即使字号、字体和基线相同也不能合并；相等高度继续复用原有合并逻辑。既有 Natural、Exact、AtLeast 及整数 EMU 字段语义保留；Exact/AtLeast 原有正数约束不变。

3 份请求 Schema 和 TS 类型（行几何、段落布局、段落路径）同源更新。Rust 的 `LineSpacing` 及持有它的输入结构不再是 `Copy`，调用方按所有权要求显式借用或克隆。每个来源段落预先解析高度向量，候选断行循环不重新解析百分比。

原生百分比共用基线的有界 BigInt 运算：最多 256 字节词法，与字号做精确有理数乘法后一次转换为 Q32，最近舍入、半值远离零。未知保留属性不能静默丢失；词法超限和未支持声明在文字组件调用前失败，并保留形状及来源位置。

`FrameParagraphSpec` 保留固定值或样式高度向量；`FrameParagraph.appliedBefore/appliedAfter` 记录应用首尾抑制后的实际值。分数转换的不确定性单独保存在 `spacingConversionError`，不因样式合并、极小值舍入或间距抑制丢失。只要存在该误差，页面保守增加 `2L + 4P + 2` raw Q32 误差，其中 L 为总行数、P 为总段落数；基线及原有几何误差继续独立累加。此界限相对固定字体输出和已选断行，不认证断行拓扑或目标应用差异。

## 实际验证

[冻结证据](../reviews/evidence/2026-09-25-paragraph-spacing-verification.json)绑定源码、工具、构建、输入及实际输出：

- 492 项 Rust 测试通过，新增 5 项；严格 Clippy、格式、70 份 Schema 与 TS 检查通过。
- 17 个新页面请求完成 Native/WASM 与 CLI 对照：14 个成功、3 个预检失败。覆盖混合字号、多行、零行距、小数、继承、首尾间距抑制、居中/底部定位、空段和装饰线。
- 89 个前期页面请求及 307 个更早运行请求保持原响应和成功像素；另重放 12 个旧基线几何请求，并新增 12 个间距几何请求。本轮实际双端执行合计 437 个请求。
- 几何输入覆盖不等/等值高度、逐行选择、零值、空行、零宽文字与显式控制项，以及长度不匹配、负数、数值类型错误和重复键。失败输入不调用组件，后续成功输入继续使用同一实例。
- 独立 XML / FontTools / Fraction 参考核对 14 个页面、19 个段落、41 个间距声明、59 个样式值、57 个度量值、30 行、58 个字形坐标和 2 个装饰矩形；比较 1,625,744 个内部像素，排除 54,256 个边缘像素。
- 共享布局独立有理数参考核对 14 份布局、15 行、23 个字形记录及 180 个坐标值；Q32 计算一致，样本整数输出最大误差 1/2 EMU。

复现入口：

```sh
MO_SPACING_EVIDENCE_DIR="$PWD/.codex-work/paragraph-spacing/cases" cargo test --workspace --locked
PYTHONPATH=.codex-work/font-tools-venv/lib/python3.13/site-packages python3 tools/verification/paragraph-spacing-reference.py
node tools/verification/paragraph-spacing-parity.mjs
node tools/verification/paragraph-spacing-geometry.mjs
python3 tools/verification/paragraph-spacing-numerics.py
node tools/verification/text-page-runtime-regressions.mjs .codex-work/paragraph-spacing
python3 tools/verification/paragraph-spacing-evidence.py
```

按[开发说明](development.md)构建 Native workers 和 Rust WASM，为本阶段生成独立 `.codex-work/paragraph-spacing/wasm-node`；保留旧阶段输入和输出，证据封存使用独占新文件。沿用已有 FontTools、NumPy、Pillow、lxml、jsonschema 开发工具。原创合成字体的已选字形和行拓扑仍为参考输入；像素核对排除 2 像素边缘，不认证抗锯齿、真实字体、多语言排版或目标应用保真。

11 个历史发行产物保持不变。本阶段没有新增产品耗时、RSS 或安装包体积测量。完整字符/段落语义、自动适配、竖排、高级内容、生产 Agent/Artifact/Worker、Office/WPS 往返和 Musterwork 替换验收仍未完成。

后续[字符间距](character-spacing.md)已将原生 `spc` 接入共享笔位、断行及装饰线，并在新产物上重放本阶段请求。本页保留段落间距阶段的历史范围。
