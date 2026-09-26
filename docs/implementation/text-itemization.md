# 自动文字分段与段落塑形准备

2026-09-24 · `mo-text::itemize` 已将固定 Unicode 18 字簇、双向层级与脚本数据连接为逻辑片段；`mo-text::paragraph` 将这些片段直接交给现有字体候选与隔离塑形组件。Native CLI/worker、WASM 与 TS 合同已接通。当前输出字形和布局准备信息，尚无按宽度断行、行高/基线、页面位置或字形绘制。

## 固定数据和分段规则

[数据清单](../../crates/mo-unicode/data/script-manifest.json)固定官方 Scripts、ScriptExtensions、PropertyValueAliases 的版本、长度和 SHA-256，沿用 Unicode-3.0 许可。运行表 `scripts.bin` 为 16670 字节，包含 179 个规范短标签、999 条主属性范围、210 条扩展范围和 120 个扩展集合。未知/私用字符保持 `Zzzz`；不从字符区块、主机语言或字体名称推断脚本。

Script 与 Script_Extensions 是数据属性；它们没有规定唯一的排版分段算法。本实现依据 [UAX #24 revision 41 的实现建议](https://www.unicode.org/reports/tr24/tr24-41.html#Implementation_Notes)，固定 `unicode18-script-bidi-grapheme-items-v1` 策略：

- 一个扩展字簇保持完整，以其中第一个显式脚本为锚点；附加符仍附属于该字簇。不同脚本的附加符或多个双向层级产生诊断，原层级不丢弃。
- Script_Extensions 限制可延续的脚本集合，再按上下文取交集。例如 `Aーア` 分为 Latin 与 Katakana，长音符不会无条件接到 Latin。
- 成对括号引用开括号所在片段的脚本；双向隔离内容分别解析，并恢复外部括号/脚本上下文。配对采用固定 BidiBrackets 数据，尚未实现语言相关引号规则。
- 只有上下文不足的多脚本集合按规范短标签排序选取第一个，并返回 `ambiguousScript`；不推断作者语言。全部为 Common/Inherited 的片段保留 Common。
- 按脚本、段落求得的层级和有效样式形成连续逻辑片段，保留 scalar/UTF-8/UTF-16 范围。制表符、显式换行、段落结尾及双向格式控制单独保留为布局项。

括号匹配使用每类型栈头，避免大量不匹配闭括号触发反复逆向扫描。所有开括号仍在输入预算内保留，没有到固定深度后静默截断。字簇边界不代表字体切换或自动折行已经安全；实际 shaping cluster 与 unsafe flags 仍须参与后续决策。

## 两个开发入口

`mo-cli itemize-paragraph request.json` / WASM `itemize_paragraph` 接收单段逻辑文本、方向策略及按 scalar 行进的样式结束位置：

```json
{"text":"A(γ)B","direction":"autoLeftToRight","spans":[{"end":5,"style":0}]}
```

返回范围 `[0,2)` Latin、`[2,3)` Greek、`[3,5)` Latin，以及双向层级和三个坐标系。样式跨度必须完整覆盖并对齐字簇；该底层接口的样式 ID 表示已经求值的塑形属性，不应直接传作者 run ID。空段用空 spans。沿用双向单段边界，内部段落分隔符会拒绝。

`mo-cli shape-paragraph request.json fonts.bin` / WASM `shape_paragraph` 在此基础上接收显式 `fonts` bundle 和 `styles` 表。每个样式包含 language、features、有序 font candidates/axes、dotted-circle 选项及 glyph 预算。原生继续使用 `mo-text-worker --paragraph`；WASM 继续调用独立 HarfBuzz 实例。

有效样式相同的相邻作者跨度会合并，避免把 `office` 中的连字无意切断。计算键忽略语言标签大小写、独立轴参数顺序和同摘要/face 的绑定别名，作者记录不改写；候选和 features 顺序保留。字形仍以全文逻辑文本作为上下文，不先应用 L2 重排。控制项不要求字体提供可见字形，但仍保留在完整上下文和 itemization 结果中；`shapedItemIndices` 明确字形结果对应哪些逻辑项。

相同 bundle 范围与 face 的绑定在批次内复用一次校验，额外样式检查复用同一组借用对象。未使用的样式、后续候选和控制项样式同样在首个组件调用前校验；无效轴或语言不能被前面的成功字体掩盖。组件失败整体返回错误，不转成缺字回退或部分成功。

## 预算和当前限制

单段 65536 scalars / 262144 UTF-8 bytes；最多 256 个样式跨度、4096 个分段输出项、65536 条诊断。自动塑形受现有候选层的 256 个文字项、32 个字体/候选、累计上下文和 glyph 预算约束。大参数在克隆片段前检查；取消在本实现各遍历处及同步组件边界检查，未据此宣称有确定的硬取消延迟。

`evaluated` 仍可能包含 `unresolved`。后续已接入[混合字体回退与边界重塑](font-fallback.md)：同一片段的 Latin 与 Emoji 可分别选择显式候选，实际字簇/连接边界参与约束，部分范围重新塑形。响应 profile 升级为 `unicode18-auto-items-hb14.5-mixed-font-reshape-v2`，使用 `fallback` 字段；选中片段具有自己的字体单位，不能直接当作页面坐标。

作者在字簇内部改变样式、Emoji/GSUB-only 变体策略、语言相关标点、竖排、脚本覆写及完整目标应用行为尚未完成。不同层级或脚本的异常字簇保留原信息并发出诊断，不能把得到字形当作其视觉质量已经验收。

## 验证与复现

```sh
python3 tools/unicode/fetch.py --manifest crates/mo-unicode/data/script-manifest.json --directory .codex-work/itemization
python3 tools/unicode/generate-scripts.py --check
cargo run -p mo-unicode --example script_properties --release --locked --offline > .codex-work/itemization/native-properties.txt
python3 tools/verification/script-properties.py
node tools/verification/itemization-parity.mjs
python3 tools/verification/contracts.py --itemization-report .codex-work/itemization/parity.json
```

上述命令需要先按[开发说明](development.md)构建 Native/WASM 和固定 HarfBuzz、准备[公开字体语料](font-resources.md)。独立数据检查遍历全部 1114112 个码点，真实 Rust 查询逐一核对 1112064 个有效 scalar。跨端测试包括全部 175 个已分配显式脚本的代表字符、括号/隔离/样式/字簇、24003 字符的深层和不匹配括号、预算及错误输入。

[最初分段证据](../reviews/evidence/2026-09-24-text-itemization-verification.json)记录当时的 197 组分段、34 组段落塑形 Native/WASM 对比；28 个自动片段的 111 个字形、字簇和位置与同版未修改 `hb-shape` 一致。当时全仓 144 项 Rust、1606 批跨端对比、26 份合同通过，旧 1375 批结果保持不变。该不可变记录不代表升级后的响应合同。

[最新回退证据](../reviews/evidence/2026-09-24-font-fallback-verification.json)在新产物上重跑全部用例：197 组纯分段结果不变，34 组段落按新的混合字体响应核对，30 个选中片段和 112 个字形与上游一致。另新增 17 组混合字体用例；合计 152 项 Rust、1623 批跨端对比。实际边界重塑分配失败的整批拒绝、实例失效及替换恢复已单独验证。

这些证据验证文字流水线的这一段连接，不关闭字体/文本 E0、完整演示能力或 Musterwork 替换门禁。下一步推进行宽拟合及行边界重塑，并把字体度量与实际布局连接。
