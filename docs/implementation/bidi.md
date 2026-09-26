# 固定 Unicode 双向文字分析

2026-09-24 · `mo-unicode::bidi` 已实现显式段落的双向层级解析和显式行的 L1/L2 求值，接入 Rust、Native CLI、WASM 和生成的 TS 类型。它保留逻辑文本及 scalar/UTF-8/UTF-16 映射，为后续 itemization、字体塑形、断行与布局提供数据；没有据此完成整段文字排版、L3/L4 绘制、竖排或 Office/WPS 兼容验收。

## 算法和数据来源

核心复用固定 `unicode-bidi 0.3.18` 的未修改 Rust 算法。其随包数据是 Unicode 16，当前关闭全部默认特性，并显式实现 `BidiDataSource` 的字符分类和 normalized opening bracket 两个入口，分别提供本项目固定的 Unicode 18 数据。不能只关闭 `hardcoded-data`：库的括号方法仍有默认数据，必须同时覆盖。已检查实际 Cargo feature 闭包，没有启用旧的 hardcoded 数据。

[组件记录](../../components/unicode-bidi/component.json)绑定 crates.io 归档长度、SHA-256 和许可选择；保留 [MIT 文本](../../components/unicode-bidi/LICENSE-MIT)、COPYRIGHT 与 AUTHORS，没有增加其他传递运行依赖。这是组件开发采用，不是选择整个项目的公开许可证。

[数据清单](../../crates/mo-unicode/data/bidi-manifest.json)固定六份官方 Unicode 18 文件：DerivedBidiClass、BidiBrackets、BidiMirroring、UnicodeData 及两份完整测试。`@missing` 的区域默认值必须先展开，再应用显式分类；不能把未列出的 RTL 区域一律认作 L。括号规范等价从该版本 UnicodeData 的 canonical decomposition 推导，不调用操作系统或语言运行时的 Unicode 规范化。

运行表 `bidi.bin` 为 14007 字节：20 字节头部、977 条分类/镜像属性范围、130 个括号记录和 438 条镜像字符映射，共标记 570 个 mirrored 码点。表保留现有 [Unicode 许可](../../crates/mo-unicode/data/LICENSE-UNICODE)，不含完整原始数据或测试语料。

独立参考将全部 1114112 个码点展开，逐点对照生成表；另核对全部括号和镜像映射。3282 个范围边界/邻居/中点及映射探针在实际 Native/WASM 查询中核对，包括新括号 U+2E62/U+2E63。U+221D 与 U+1DB10 的镜像映射跨越 BMP/SMP，不能按固定 UTF-16 宽度原地替换。数据、规则版本和实际参数分别记录，依据 [UAX #9 revision 52](https://www.unicode.org/reports/tr9/tr9-52.html)。

## 显式段落与行合同

入口：Rust `analyze_paragraph`、开发 CLI `mo-cli bidi-analyze request.json`、WASM `analyze_bidi(requestJson)`。批次请求包含 `paragraphs` 和 `characters`；字符查询接受有效 Unicode scalar，拒绝 surrogate 及越界数。

```json
{"paragraphs":[{"text":"אב  גד","direction":"leftToRight","lineEnds":[4,6]}],"characters":[11874,11875,8733,121616]}
```

每段要求明确的方向策略：LTR、RTL 或按 P2/P3 推断的 auto-LTR。`text` 是调用方交付的一个段落，可保留尾部段落分隔符或 CRLF；内部 B 类分隔符会报错，文档/纯文本层负责划段。没有把相邻段落合在一起进行双向分析，也不暗中把作者换行替换成其他字符。

`lineEnds` 是布局确定的 scalar 行尾，必须递增、覆盖全文且不切开扩展字簇。空数组表示整段作为一个分析行；它不是自动按宽度断行。空段保留一个起止都为零的空行，便于后续布局和定位。CRLF 作为一个字簇，不允许行尾落在二者之间。

结果 profile 为 `unicode18.0.0-uax9-r52-explicit-paragraph-l2-v1`，包含：

- `paragraphLevel`：段落基准层级。
- `resolvedLevels`：I1/I2 后、逐行 L1 前的每 scalar 层级。
- 每行起止的 scalar/UTF-8/UTF-16 坐标、L1 重置后的层级，以及 L2 的 visual-to-logical scalar 索引。

按 X9 移除的项在层级数组中保留 `null`，只从 L2 索引列表中省略。它们没有从作者文本删除；ZWJ/ZWNJ 等仍须以原始逻辑上下文送入塑形器。**不能按 `visualOrder` 重排输入字符串后再塑形。** 此列表也不是最终字形绘制次序：字簇、组合字、连字和镜像由后续塑形/布局处理，避免把同一字簇拆开或重复镜像。

上例段落层级为 `[1,1,1,1,1,1]`；第一行的尾部空白在 L1 后变为 `[1,1,0,0]`，两行 L2 索引分别为 `[1,0,2,3]` 和 `[5,4]`。段落求值不能提前覆盖掉这些逐行差异，实际换行改变时需重新求对应的 L1/L2。

## 预算、取消和行处理成本

默认每段最多 65536 scalars、262144 UTF-8 bytes、65536 行；批次最多 1024 段、累计 262144 scalars / 65536 行、65536 个属性查询，JSON 仍受 32 MiB 限制。请求验证、数据查询及行遍历有取消检查；失败不返回前面段落的部分结果。

上游提供的便捷 L1 接口会为每行克隆整段层级。本实现按标准对当前行的可见 scalar 执行 L1，再调用同一库的 L2，避免在多行文本上重复复制全文。没有改变原始段落层级；本轮实际验证一个段落含 16384 个显式行。索引到坐标的查找只读取已计算的字簇边界。

上游段落解析与 L2 调用都是同步计算，没有内部取消回调。核心在调用前后检查取消并限制输入；外部硬取消仍需宿主进程/Worker。没有通过 panic、中途替换分类或返回部分排序冒充可中断执行；生产 Worker、完整资源计量和延迟指标仍待完成。

## 验证与复现

```sh
python3 tools/unicode/fetch.py --manifest crates/mo-unicode/data/bidi-manifest.json --directory .codex-work/bidi
python3 tools/unicode/generate-bidi.py --check
python3 tools/verification/bidi-properties.py
node tools/verification/bidi-conformance.mjs
node tools/verification/bidi-parity.mjs
python3 tools/verification/contracts.py --bidi-report .codex-work/bidi/parity.json
```

[本轮证据](../reviews/evidence/2026-09-24-bidi-verification.json)绑定实际源码、锁文件、生成表、归档和 Native/WASM 产物：

- `BidiTest` 的 490846 行数据展开为 770241 个类型/方向用例；`BidiCharacterTest` 为 91707 个字符用例，合计 861948 个用例全部通过，0 跳过。每个用例分别经过真实 Native 和 WASM，组织成 843 个调用批次；核对 L1 层级、L2 顺序及测试中明确给出的段落方向。
- 22 组附加跨端验证覆盖属性查询、行尾空白、三个坐标系、隔离/嵌入深度溢出、16384 行、错误边界和各类预算。6 项新增 Rust 测试包含所有本实现取消检查点、逻辑文本保留及新括号/镜像映射。
- 既有 510 组跨端回归在新产物上保持结果不变。总计 1375 个 Native/WASM 调用批次、134 项 Rust 测试、22 份 Schema/TS 合同通过；不把 861948 个标准用例和调用批次数混加。

官方语料本身只覆盖至 L2，且不验证段落自动划分和绘制相关 L3/L4。本模块的单段输入、实际行尾、字簇边界和坐标另外验证，仍不代表自动 script 分段、字体切换、行宽拟合、基线、镜像字形、竖排或目标应用排版已完成。下一步将这些层级与 script/样式和字体塑形范围连接，保持逻辑锚点贯穿布局与编辑。
