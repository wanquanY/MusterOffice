# 自动行宽与段落布局

2026-09-24 · 文本 E0 进展。`layout-paragraph` / WASM `layout_paragraph` 把默认 Unicode 断行、完整字簇、行边界重塑和水平几何连接起来，按显式框宽生成实际行、字形原点与溢出证据。它不向作者文本插入软换行。

## 当前计算合同

`ParagraphLayoutRequest` 包含逻辑 `paragraph`、对应几何样式、strut、行距、正 EMU `width` 和显式 `overflow` 策略。没有系统字体、隐式资源获取或宿主 DOM 测量。完整原始样式与所有字体资源在第一次组件调用前验证，包括未使用的候选。

当前 profile 是 `unicode18-egc-farthest-fit-preserve-spaces-q32-v1-draft`。它使用固定 Unicode 18 / UAX #14 r57 默认机会，与扩展字簇边界求交；被排除的机会在 `suppressedGraphemeBreaks` 中保留。该字簇限制属于明确的高层定制，不把它写回默认 UAX #14 分析结果。SA 位置继续由 `breaks.complexContextScalars` 报告，尚无词典分词或 Office/CJK 专项禁则。

每一行必须在最近的强制断点之前或该断点结束，选择可放入框宽的最远合法位置。测量使用实际候选行的字体回退和重新塑形，保留完整段落脚本、样式与 bidi 解析，再对候选范围执行 L1/L2。不能把候选行作为新段落，也不能从整段旧字形中切出一个近似宽度。

当前放入条件为水平 pen 最小值不小于零，最大值不大于框宽，使用[几何层](line-geometry.md)的内部 Q32 EMU 精度比较。它不是 ink bounds 检查；offset、轮廓和效果可超出 pen 范围。即使 1.2 EMU 的最终坐标舍入成 1 EMU，也不能放入宽 1 EMU 的框。

本 profile **保留并计算所有空白的 advance，包括行尾空格**。因此它尚不等于目标应用的空白悬挂、压缩或分散策略；这些策略需要统一影响断行判断与最终放置，不能只从宽度上偷偷扣除空格。目标应用 profile、tab、连字符、对齐、竖排和完整段落属性继续实现。

## 不假定宽度单调

连字、上下文替换、kerning 和有符号 advance 不保证候选行尾向后移动时宽度一直增加。当前算法从最近强制断点向前检查合法候选，找到的第一个可放入候选就是最远可行候选。它不使用依赖单调性的二分搜索，也不在第一次遇到较短行溢出时断言更长行均不可放入。

这是一版有明确工作预算的正确性实现，最坏成本不是线性的，也没有通过完整产品的性能门禁。候选场景中的初始化工作已移出循环：字体只校验一批；段落脚本/样式/bidi 与 scalar/byte 边界预先准备；候选只建立范围内的行计划。Rejected 候选及时释放，选中候选的完整字形直接进入最终布局，不再重复塑形；度量只对最终字体实例批量查询。

初始化中的 itemization 与可复用 bidi 对象仍各自解析一次段落。常驻组件字体对象、候选批处理、可靠的宽度界和增量缓存尚未实现；后续优化必须保留断点、字形与内部宽度等价性，不能以猜测或降低质量换速度。

## 溢出、显式换行与未决前提

- `keepUnbreakable`：没有任何合法候选能放入时，保留到最近合法位置，标记该行 `overflows`。不截断内容或缩小字号。
- `emergencyGrapheme`：先检查所有正常合法候选；均不能放入时，在首个合法断点之前按完整字簇寻找最远可放入位置。没有可放入字簇则保留一个完整字簇并标记溢出。`emergency` 明确记录这种额外断点，不能把它当成未经定制的 UAX #14 结果。

每条 decision 保留三个坐标系的行尾。作者显式 line break 结束文本时，生成一个具有实际 strut 行框的末尾空行，因此最后两个行尾可以相同；段落终止符如终端 CRLF 不额外创建该空行。空段本身有一个空行。内嵌多个 bidi 段落仍由上层文档层拆分，本入口只接收一个逻辑段落。

同时补正 U+000C form feed 的 itemization，将其识别为显式行控制，不送入字体作为普通字形。U+000B、U+2028、U+000C 及终端 CRLF 有对应验证。

遇到 tab、soft hyphen 或 contingent object 时，当前返回明确 `FlowIssue`、空 decisions 和 null geometry。这些内容需要停靠、条件字形或内联对象尺寸，不能静默按零宽成功。无法测量的缺字体范围或混合 bidi 层级字簇同样不会发布部分计划。度量缺失等最终几何前提继续由 `geometry.issues` 与 null `geometry.layout` 表示，不能将请求求值成功等同于完整排版成功。

底层输入/组件/数值失败和取消返回错误，不保留前面已选出的行。失效组件仍由宿主按进程或模块替换。实际第二候选分配失败已验证这一点。

## 预算与可复用范围

最多 4096 次候选、2097152 个累计候选 scalar、4096 个最终行/item；候选重塑共享既有 1024 shaping run、1048576 上下文 scalar/探测字形等累计预算，最终保留的 fragment/glyph 也检查总量。命中预算时整批返回 LIMIT_EXCEEDED，不自动改成较差断点。最大行数包含末尾空行，4095 个显式换行可以产生 4096 行；4096 个显式换行需要的额外行超过当前预算。

`work` 分别报告候选、候选 scalar、已验证 face、塑形调用和探测字形；componentCalls 是塑形调用计数，不包含随后度量调用，也不是耗时或内存测量。字体资源、脚本、文本和 style 参数均属于一次请求的不可变上下文，没有跨请求偷用旧字形。

## 验证与开发入口

[本轮证据](../reviews/evidence/2026-09-24-paragraph-layout-verification.json)绑定真实源码、生成合同与产物：

- 37 批 Native/WASM 完整响应字节一致，包含实际框宽变化、中英/阿拉伯/天城文、CJK 标点、Emoji、混合字号和基线、轴与全局 feature、强制/紧急换行、空行、预算与失败。
- 72 个选中片段、138 个字形及最终字体实例度量与固定 HarfBuzz 上游入口对照。该对照共享算法，不是独立排版引擎。
- 另一套正向穷举选择逻辑通过显式行入口测量 157 个候选，核对 23 份短文布局的全部断点/紧急/溢出决定；宽度以独立 BigInt Q32 运算求出。它共享字体塑形与 bidi，长预算语料不做此穷举。
- Python Fraction 独立核对 24 份完整布局、4161 行、138 个已定位字形、33588 个坐标，Q32 规则完全相等；样本相对未量化有理数几何的最大 wire 偏差为 1/2 EMU。
- 新增 9 项 Rust 测试，其中包含非单调宽度模拟反例、舍入边界、已有字形复用、整批取消、预算和预解析 bidi 范围。实际第二候选内存故障验证无部分结果、永久失效和新实例恢复。
- 当前总计 189 项 Rust 测试、1799 批 Native/WASM 对比、36 份 Schema；旧 1762 批语义结果与 34 份 Schema 不变。

```sh
node tools/verification/paragraph-layout-parity.mjs
node tools/verification/paragraph-layout-selection.mjs
python3 tools/verification/line-geometry-reference.py --paragraph-layout
python3 tools/verification/contracts.py --paragraph-layout-report .codex-work/paragraph-layout/parity.json
target/release/mo-cli layout-paragraph .codex-work/paragraph-layout/cli.json .codex-work/paragraph-layout/cli.bin
```

本轮无新依赖、字体或 Unicode 数据，C++/TS 组件适配器保持原字节；产物大小只属于不完整开发构建，没有完整安装包或加速比结论。完整字体策略、Office/WPS 视觉与可编辑往返、绘制、播放、高级对象、Agent 产品接口和 Musterwork 替换门禁仍未完成。

依据：[UAX #14 revision 57](https://www.unicode.org/reports/tr14/tr14-57.html)、[HarfBuzz buffer 与位置语义](https://harfbuzz.github.io/harfbuzz-hb-buffer.html)。
