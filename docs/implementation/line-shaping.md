# 行上下文与重塑

2026-09-24 · 文本 E0 进展。`shape-lines` / WASM `shape_lines` 从逻辑段落、显式字体和计算得到的行尾计划重新塑形，保留完整段落的脚本与双向语义，同时隔离实际行外的塑形上下文。它供后续行宽求解反复测量候选行，不把软折行写入作者文本。

## 为什么需要独立的行上下文

对一段文字塑形后直接切字形数组，会把连字、阿拉伯连接或上下文替换带到错误的行。仅设置 HarfBuzz `BOT/EOT` 也不等于清空前后文：本轮实际用“بب”复现，给第一字设置两个边界标记但仍保留第二字为后文，结果中的主体 glyph ID 为 19；真实单字行的主体为 14。数字只属于固定 Noto 字体和版本，不是通用字符映射。

新入口为每行明确限定上下文，重新执行字体候选、缺字/UVS 检查及部分字体区域重塑。行内仍保留跨样式/字体片段的上下文；行外文字不进入这次塑形。完整片段再次求值，而不是拼接旧 glyphs。原有段落/原子候选接口维持原行为。

## 合同与坐标

`LineShapeRequest` 包含既有 `ParagraphShapeRequest` 与 `lineEnds`。行尾是严格递增、覆盖文本、对齐扩展字簇的 Unicode scalar 坐标，空文本使用 `[0]`。最多 4096 行/输出 item；累计塑形次数、片段和字形预算沿用字体回退限制，行数上限不保证任意复杂内容都落在其他预算内。

这是低层的显式计算计划。它允许对齐字簇的词内候选位置，以支持后续紧急断行或候选测量；它本身不判定 UAX #14、Office/CJK 禁则、作者强制换行或连字符策略是否合法，也不自动决定行宽。调用方必须在最终选择阶段执行这些规则。

完整段落先解析有效样式、Script/Script_Extensions 和 UAX #9；实际行再执行 L1/L2。不能把每行当成新的 bidi 段落，否则跨行中性字符、括号和 isolate 会丢失语义。原来同一 item 的尾部空格若经 L1 回到段落层级，会重新分开。扩展字簇内存在混合层级时，继续使用首个有效层级并保留诊断，不隐式拆开组合字。

输出 `items`、`fallback` 和 `shapedItemIndices` 仍按逻辑顺序保存。每个 `lines` 项给出三个坐标系的范围，以及这些数组的半开索引范围。`bidi.lines.visualOrder` 仍只是 scalar 的 L2 映射，不是最终 glyph 绘制次序。Tab、显式控制项、段落结束符保留为 item，目前不产生布局 advance。

底层组件接收行内 scalar 坐标；Rust 把 run、glyph cluster、缺字位置映射回原段落。全局 feature 范围先完整验证，再与行范围求交并平移，保持重叠 feature 的顺序；不相交项不发送。字体轴、语言、候选顺序不被改写。分段与行计划的产生都不修改作者字符串或样式记录。

## 资源与失败

复用 `cascade::Context` 的已验证字体资源，一次输入中相同绑定不因行数重新读入和校验；每个实际组件批次仍会上传字体，尚无常驻 C++ 字体对象缓存。计入预算的上下文是实际行长度乘塑形次数，不是每行重复计整个段落。字节坐标一次建立，避免按行从头查找 UTF-8 偏移。

所有行尾、原始样式、全部候选及字体字节都在第一次组件调用前验证。实际计算失败不发布前面成功行的部分结果；第二行的真实分配失败会永久使组件失效，后续拒绝复用。取消点保留整批失败语义；失效组件的进程/模块替换仍由宿主负责。

后续持久缓存键必须包括实际上下文、行边界、已选方向/脚本、字体实例和 feature 语义。当前逐项缓存仅存在于一次候选求值内部，不跨行或跨请求复用旧结果。

## 验证

[本轮证据](../reviews/evidence/2026-09-24-line-shaping-verification.json)绑定实际源码与产物：

- 34 批 Native/WASM 返回字节一致；59 个选中片段的 139 个字形与独立同版上游 CLI 对比，包含 glyph、cluster、flags、advance 和 offset。
- 连字中间的候选行尾、阿拉伯单字行/lam-alef、跨行 bidi isolate/括号、L1 尾部空格、天城文字簇、中文、Emoji ZWJ、混合字体、变化轴与全局 feature 范围均有实际输入。
- 未解析字体范围继续显式存在；空段、控制项、非法字簇切分、未覆盖/重复行尾、未使用坏样式/字体、4096 控制行及累计塑形预算均覆盖。低层显式计划的成功不表示完整排版成功。
- 真实第二行内存失败后无部分结果，失效复用拒绝，新实例恢复；另保留上述“只改 BOT/EOT”的实际反例。
- 新增 5 项 Rust 测试，当前总计 171；旧 1693 批主内核语义结果不变，总计 1727 批，运行 Schema 为 32 份。

上游对照共享 HarfBuzz 算法，L1/L2 复核共享已经过官方测试的固定 bidi 实现；本轮不能替代目标 Office/WPS 验收、全面模糊测试或视觉比较。没有本轮完整布局性能或安装包体积结论。

## 开发入口与后续

按[开发说明](development.md)重建 Rust CLI/worker/WASM 以及生成合同。组件与字体沿用已验证固定版本；故障测试使用已有带注入构建。

```sh
node tools/verification/line-shape-parity.mjs
python3 tools/verification/contracts.py --line-shape-report .codex-work/line-shaping/parity.json
target/release/mo-cli shape-lines .codex-work/line-shaping/cli.json .codex-work/line-shaping/cli.bin
```

下一步用[实例度量](font-instance-metrics.md)和本入口连接字号缩放、基线与行宽求解，再完善连字符、tab/对齐、竖排、行框、溢出及目标应用策略。当前没有自动行宽选择、最终位置、绘制、页面布局或替换就绪声明。

依据：[HarfBuzz cluster 语义](https://harfbuzz.github.io/working-with-harfbuzz-clusters.html)、[buffer 上下文接口](https://harfbuzz.github.io/harfbuzz-hb-buffer.html)、[UAX #9](https://www.unicode.org/reports/tr9/)。
