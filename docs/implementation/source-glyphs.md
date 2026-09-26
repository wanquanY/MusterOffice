# 原生文本到局部字形路径

2026-09-25。[清单布局](manifest-layout.md)之后，`mo-presentation-compile::source_text` 已把真实 PPTX 的 run、来源样式、字体名称与显式资源清单连接到塑形和字形路径计算。该阶段产出局部、未着色的段落路径；来源页面尚未绘制文字，Musterwork 替换门禁保持未完成。

## 计算边界

`prepare` 接受不可变 `SourceIndex`、预期文件摘要、对象引用、预算及取消回调。它为整个对象计算一次来源样式，在任何字体组件调用前完成各段原生输入准备；任何未支持的输入返回诊断，不返回半个准备对象。私有字段的 `PreparedSourceText` 不能从外部 JSON 构造，读取接口只提供不可变引用。

原始 run 的物理序号、类型和 Unicode scalar 范围，与计算用样式区间分别保存。普通 `a:t` 不归一化、不改大小写；`a:br` 映射到 U+2028，空 run 保留零长度来源。相同有效样式可以跨 XML run 合并，避免把组合字符和连字人为切开。若样式变化切入扩展字簇，则返回边界诊断。段尾样式单独保存，不能覆盖已有 run。

每个段落计划保留源段落序号、原始文字、来源范围、字体声明/主题绑定、计算字体样式和几何字号。塑形和路径结果另绑定源文件摘要、对象、段落下标和物理序号，不能只凭临时样式编号认领来源。

`shape_paragraph` 和 `paragraph_paths` 复用调用者已加载的 `PreparedManifest`。字号由百分之一磅精确换算为 EMU；kerning 缺省关闭，显式阈值按原生字号比较后生成 `kern` 特性。粗体/斜体必须有清单实例，不合成、不借用其他槽。缺字体、缺实例、字符覆盖和轮廓问题继续保留原有错误/诊断。

`paragraph_paths` 的 `SourceGlyphFlow` 是后续 Rust 文本框编译器的内部衔接输入，显式提供行宽、行距、溢出策略及边界容差。它不会从原生段落属性推导这些值，也不会应用段落对齐、缩进、项目符号、文本框内边距/锚点、旋转或文字画笔。返回值是局部字形几何，不能把它当成页面或 Office 排版结果。来源页面原有的文字拒绝检查保持有效，直至实际文本框、画笔和误差预算接通。

## 字体选择的明确范围

本阶段配置为 `drawingml-source-glyph-input-draft-v1`，是可复核的计算策略，**尚非 Office/WPS 兼容认证**。

先由已有 Unicode 18 分段计算脚本和双向上下文，再选择 DrawingML 的字体槽。Latin/Greek/Cyrillic 及没有强脚本的 Common/Inherited 使用 Latin；Han/Kana/Hiragana/Hangul/Bopomofo 使用 EastAsian；代码表中明确列出的 Arabic、Hebrew、Indic、Thai 等使用 ComplexScript。表外脚本、混合/歧义字簇保留诊断，不把所有剩余脚本都猜成 Latin。

Han 的补充字体键来自显式 `lang`，没有可用键时再看 `altLang`：中文 Hans/Hant 优先于地区，CN/SG 对应 Hans，TW/HK/MO 对应 Hant；日文、韩文分别使用 Jpan、Hang。只读取语言标签前部的脚本/地区位置，不把 extension/private-use 中的字符串当成地区。无明确语言时，直接声明的 EastAsian 字体仍可用；需要主题补充列表却没有脚本键时返回诊断。没有读取本机区域或系统字体。

Common 字符的脚本上下文沿用既有分段算法；纯 Common 内容及空行 strut 的插入样式，在本 draft 中明确选择 Latin。该规则、语言冲突优先级、更多脚本/符号映射及目标应用差异仍须后续验证和扩展，不能以此替代完整文本兼容计划。`a:sym` 与 symbol charset 当前返回诊断。

非零字符间距、基线上下偏移、大小写变换、kumimoji、normalizeHeight、字符方向覆盖及动态 field 尚未接入此字形编译器，当前明确拒绝；并未从一期范围中删除。下划线、删除线、填充、效果和链接仍保存在完整来源样式中，不由未着色路径层绘制。页面编译必须处理它们后才能发布视觉结果。

## 资源与失败

准备阶段的文字字节、计划分配及字体绑定预算由整个对象共享，不能每段重置。计划预算对复制的字体名称/来源和缓存条目计费，防止共享长声明被大量复制放大；这是保守分配核算，不是 RSS 测量。下游排版、路径与组件仍沿用各自预算。跨对象/整页预算、预检和原子发布尚待页面编译器完成。

字体字节只由已验证清单借用。核心没有文件路径、文件读取、网络、权限或系统字体依赖。示例程序作为开发宿主持有明确路径；生产接入和隔离实例池没有因此自动完成。

## 验证与复现

全仓 445 项 Rust 测试、严格 Clippy、格式检查、来源编译器 WASM 目标编译及 68 份 Schema 生成检查通过。新增 11 项测试覆盖跨 run 字簇/来源、混合脚本、主题语言补充、空 run/空段落/换行、段尾隔离、kerning、明确未支持语义、过期摘要、全部准备取消检查点、对象共享预算、清单预检与组件失败。

真实 Native 示例读取 8 份原创 PPTX，准备 9 个段落，执行 9 次塑形和 18 次路径计算；缺少字体/粗体实例的 3 次塑形、6 次路径计算返回预期错误。其余 12 次路径计算返回场景结构，其中包含空段落。独立 XML 核对 15 个文字范围、25 个字体绑定和物理来源；Fraction 参考核对 22 个字形的 44 个坐标，另核对 28 个路径/整体边界。这些合成字体测试不证明真实字体视觉质量，也没有测量产品速度或安装体积。

```sh
mkdir -p .codex-work/source-glyphs
MO_SOURCE_GLYPH_EVIDENCE_DIR="$PWD/.codex-work/source-glyphs/fixtures" cargo test -p mo-presentation-compile --locked
cargo build -p mo-harfbuzz-sys --example source_glyphs --locked
python3 tools/verification/source-glyph-reference.py
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo check -p mo-presentation-compile --target wasm32-unknown-unknown --locked
```

`source_glyphs` 示例对每个 PPTX 加载一次已验证清单并跨段落复用；Python 参考会实际执行当前 Native 示例，再用独立 ZIP/XML 和 Fraction 计算核对结果。示例的字体只使用仓库已有原创合成语料。所有临时输出位于 `.codex-work/source-glyphs/`。

本轮没有新增公开运行时操作、Schema 或 TS 类型，没有新增外部依赖版本，没有重建历史发布产物，也没有新增 Native/WASM 执行对比。此前 10064 批仍只认证[文本框阶段冻结产物](../reviews/evidence/2026-09-25-text-body-verification.json)。完整页面文本、媒体/高级内容与播放、Agent 分发、Office/WPS 互操作和 Musterwork E0–E3 继续推进。

本轮冻结记录见[来源字形库级证据](../reviews/evidence/2026-09-25-source-glyph-library-verification.json)。

后续[原生水平文本框布局](source-frame.md)已从来源属性推导行宽和位置，并通过整框预检/失败处理连接实际 Native 字形；本页保留上述阶段自身的范围与历史证据。
