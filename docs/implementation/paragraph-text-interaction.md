# 段落光标、选区与命中计算

状态：段落计算层已实现并通过下述检查；整体编辑方案继续实施。属于原生演示文稿编辑 P1 的段落计算层；页面对象命中、坐标变换、键盘移动、IME 产品链路和发行验收仍需继续完成。

## 所属层和输入

`mo-text::interaction::paragraph_interaction` 接收既有 `ParagraphLayoutRequest`、最多 64 个查询和显式字体包。Rust、Native Worker、WASM 与 TS 薄接口共用同一个实现。`PreparedManifest::paragraph_interaction` 复用已验证资源，并接收与源编译器一致的首行/续行宽度。计算不访问系统字体、网络、文稿持久化或 UI。

查询只有三类：按逻辑位置取光标、段落坐标点命中、按逻辑选区取矩形。逻辑位置使用 Unicode scalar offset 与 `upstream`/`downstream` affinity。所有端点必须是内核固定 Unicode 18 字素边界；返回 scalar、UTF-8、UTF-16 三种偏移。无效查询在第一次字体组件调用前拒绝。

公共 JSON 不接收调用方提供的几何 map。Rust 返回计算生成的 `InteractionMap`，可在持有该次排版结果的计算层复用查询；宿主负责文稿 revision 与缓存生命周期。当前 API 未承诺增量排版。

## 与渲染共享的几何

交互计算复用 `flow::layout_flow` 的候选搜索、字体选择、双向文字重排和 `geometry` 的同一次行度量；`FragmentPen` 提供与渲染一致的 Q32 EMU 坐标、完整前缀缩放、字距及 glyph offset。无 UI 二次文字排版。

几何计算按需保留三类结果：普通布局不保留额外交互数据，路径渲染保留 glyph origin，交互计算保留 fragment baseline/font extents/精确行框。正常渲染不会因为新增交互 API 查询 GDEF。

Shaping cluster 可以起于扩展字素内部，例如 Indic syllable。交互层将 glyph cluster 的源区间扩展到固定字素边界，并合并重叠区间；保留原始 glyph 顺序和 pen 坐标，只在共同边界生成光标。

每个字素对应一个 logical-order cell，cell 持有前后两个光标边、行号、bidi level 与排版来源。行内查询索引不改变源文本顺序。选区返回覆盖逻辑区间的逐字素矩形；双向文本可能生成多个不连续的视觉区域。查询不把其外包围盒当作选区。

## 合字和不可见内容

- 单字素 shaping cluster 使用实际 pen 两端，避免在组合字符或 emoji 序列内部生成光标。
- 多字素 cluster 有唯一主 glyph（或一个前进 glyph 加零前进 marks）时，向同一个已验证字体实例批量获取 GDEF。采用恰好 `字素数 - 1` 个且按坐标非递减排列的值，将其加入真实 glyph origin，包括 GPOS offset。RTL 将视觉边界顺序映射回逻辑顺序。
- 字体没有对应 carets、数量不匹配、坐标顺序不满足规则，或多 glyph 无法唯一归属时，内核按 cluster 实际 pen 范围等分。整段有理位移只舍入一次；返回明确的 `clusterPartition` 和原因。组件错误不能转为等分成功。
- 结构换行、bidi controls 和零 glyph 内容拥有零宽边，优先锚到同一行逻辑前项尾部，其次后项头部，最后使用 strut。两遍线性扫描完成，避免长控制符输入的二次复杂度。
- 段内末尾换行保留一个空行；段落终止符结束当前段落，下一段由上层段落布局负责。CRLF 是一个字素，不允许插入内部光标。

字体 GDEF 的位置相对未定位 glyph；应用层必须结合实际排版。依据：[OpenType GDEF](https://learn.microsoft.com/en-us/typography/opentype/spec/gdef)、[HarfBuzz ligature caret API](https://harfbuzz.github.io/harfbuzz-hb-ot-layout.html#hb-ot-layout-get-ligature-carets)。字体数据层的资格范围见[合字光标](font-ligature-carets.md)。

## Affinity 和命中规则

`upstream` 使用该逻辑边界前一个字素的尾部；`downstream` 使用后一个字素的头部。两者在软换行和 bidi 边界可以位于不同位置。段落首尾缺失的一侧归一化到实际可用光标；末尾空行的 downstream 位于该空行。

命中先选纵向距离最近的精确行框，同距离选较早行。行内包含点击坐标的可见 advance cell 优先，再按水平边距离、光标条纵向距离、可见内容、段落基方向、downstream 和 scalar offset 确定结果。此顺序防止点击 RTL 字符内部时跳到相邻 LTR 字符的同 x 边界。`inside` 表示落在至少一个可见 advance cell 内，不是 glyph 轮廓命中或裁剪结果。零行高/重叠行采用同一确定性规则。

## 有界计算和失败

沿用文字/字素/行/字体/布局工作限制。新增最多 64 个字体实例组、4096 个去重 glyph 查询，每批最多 256 个 glyph。字体实例按 verified face、有效变化轴和方向分组。单次查询批次最多访问 1,048,576 个工作项，并最多返回 65,536 个选区片段。

所有阶段保留取消检查。字体、度量或布局先决条件未满足时，不返回 partial map；既有诊断仍在 layout 中。组件异常、格式错误、取消或查询预算超限使整次调用失败，不能泄漏已完成的前半份结果。后端隔离沿用文字组件既有协议。

## 验证与未覆盖范围

验证源码：

- `crates/mo-harfbuzz-sys/tests/text_interaction.rs`：真实合字 GPOS、非均匀 GDEF、bidi、字素、段落/段内换行、tracking、baseline、与路径渲染布局相等。
- `crates/mo-text/src/interaction/tests.rs`：明确分配策略、组件坏回复、逐检查点取消、先决条件与有界选区输出。
- `crates/mo-text/src/manifest/prepared_tests.rs`：复用已验证字体及首行/续行宽度。
- `tools/verification/text-interaction-parity.mjs`：真实 Native/WASM 字节相等、渲染器排版相等、TS facade 及固定上游字体语料。

`owned-interaction.ttf` 是原创测试字体派生资源。其 Hebrew/CJK 映射只测试脚本与坐标，不证明这些语言的字形质量。字体清单、生成脚本和固定摘要随仓库保存。

本层不证明页面变换与裁剪、嵌套组/表格/母版命中、视觉方向键移动、跨段落选区、IME、Desktop/Web 编辑体验、独立全屏窗口或 Office/WPS 互操作完成。当前交互查询尚未随新的 Musterwork 固定 SDK 发行。

## 2026-10-03 本机资格记录

macOS 26.2 arm64 / Rust 1.92.0 / Node 23.5.0 / wasm-bindgen 0.2.126。
125 项文字 Rust 检查、6 项真实 Native 字体测试通过；严格 Clippy、Rust schema 核对和生成 TS 类型检查通过。
43 批 Native/WASM JSON 字节一致，其中 33 批完整交互结果、4 批明确的未满足先决条件诊断、6 批显式输入/资源拒绝。
37 批正常接口求值同时核对 TS facade；布局结果与真实路径渲染器逐字段一致。10 批使用已固定并核对摘要与许可的
Noto Sans、Arabic、Devanagari、SC 和 Emoji 原始字体（宽/窄两种宽度），不使用系统字体。

真实 Devanagari 语料暴露 shaping cluster 起于 EGC 内部的情况；修复后保留该语料，另加 LTR/RTL 合成回归。
这些通过结果证明本机 Native 与 Node WASM 的段落计算，不替代前述页面/UI/平台验收。
源码、构建产物和输出摘要见[验证记录](../reviews/evidence/2026-10-03-paragraph-text-interaction.json)。

复现构建沿用已验证的 caret HarfBuzz 组件（[构建说明](font-ligature-carets.md)），并明确设置
`MO_HARFBUZZ_LIB_DIR=.codex-work/caret/harfbuzz` 的绝对路径。Native 集成还设置既有 Skia 组件目录。

```sh
cargo test --locked -p mo-text
cargo test --locked -p mo-harfbuzz-sys --test text_interaction -- --test-threads=1
cargo build --locked --release -p mo-text-worker
cargo build --locked --release -p mo-wasm --target wasm32-unknown-unknown
.codex-work/toolchain/bin/wasm-bindgen --target nodejs --out-dir .codex-work/text-interaction/wasm-node target/wasm32-unknown-unknown/release/mo_wasm.wasm
# nodejs output directory uses a package.json containing {"type":"commonjs"}.
pnpm build:editor-client
python3 tools/verification/fetch-font-corpus.py --verify-only
node tools/verification/text-interaction-parity.mjs .codex-work/text-interaction
```

脚本默认复用 `.codex-work/caret/{harfbuzz,text-component}`，可用
`MO_INTERACTION_COMPONENT_ROOT` 指定同布局且已验证的组件构建目录；不会自动下载/替换字体。
