# 原生字体名称与显式资源绑定

2026-09-25。本阶段为来源文字编译补齐两层基础实现：真实 PPTX 的字体名称求值，以及宿主字体资源清单到已有段落塑形管线的绑定。前者位于 [`mo-pptx::source::text::fonts`](../../crates/mo-pptx/src/source/text/fonts.rs)，后者位于 [`mo-text::manifest`](../../crates/mo-text/src/manifest/mod.rs)。两者均为 Rust 库入口；本阶段尚未连接成来源页面文字绘制，也没有新增 CLI/WASM/Agent 操作。

## 原生名称与来源

`drawingml-explicit-theme-typeface-draft-v1` 接收此前计算得到的字符样式、显式 Latin/EA/CS/Symbol 槽和可选主题脚本键，按原始声明选择字体名称。直接名称、六种 `+mj-*` / `+mn-*` 引用、形状 `fontRef`、主题默认项 `style/fontRef`、主/次字体集合与主题覆盖部件分别保留来源。原始字符字体的 PANOSE、pitchFamily、charset 与选中主题字体自己的元数据分开保留，不互相覆盖。

字符槽的直接声明优先；不存在时使用对象、已匹配版式占位符、基础主题 txDef/lnDef/spDef、已匹配母版占位符的首个 fontRef。显式 `none` 终止选择。主题字体集合由已有关系驱动的 `theme_selection.fonts` 决定，覆盖部件替换整个集合，不擅自向基础主题补字段。

当前 draft 对非空的 Latin/EA/CS 成员先使用成员名称；成员为空时，才按调用者明确提供的 Hans/Hant/Jpan/Arab 等四字母键查找 supplemental 项。缺少键、无匹配、重复匹配、空名称、未知主题 token、主题 scheme 保留语义均产生诊断。该入口不把语言标签猜成脚本键，也不自动按 Unicode 切换字体槽；自动分段、语言、复杂文字字体规则须由后续来源文字编译器结合既有 Unicode 管线处理。Symbol 不从 fontRef 猜测字体。

主题默认项读取器只在真实 `style` 的直接子级捕获 fontRef；spPr 或 line/fill/effect 引用内部的同名节点不会被误读。未知字体集合属性/保留扩展进入 `retained_ordinals`，通过同源 Schema/TS 导出。未知内容保留不等于已解释，也不等于文件通过全量 XSD。所选字体节点不读取文件、网络或系统字体。

依据 ECMA-376 Part 1 的 fontScheme、majorFont、minorFont、fontRef 和字体声明定义，以及 [Office 字体元数据差异](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/e6784cb7-1547-4ee5-addc-730cac8b4d00)。Office 未实现 PANOSE 的说明不意味着可丢弃原始声明。字体继承组合、非空成员与 supplemental 的冲突策略、显式空字体行为仍是待 Office/WPS 实测的 draft，不能算互操作验收。

## 名称到实际字体资源

`explicit-font-resource-manifest-draft-v1` 明确给出资源 SHA-256、范围、face index、字体原始 name 表记录，以及 regular/bold/italic/boldItalic 各槽对应的 face 与变体轴。资源范围通过既有批次加载和校验复用；新增名称/轴检查在首次组件调用之前执行，不重复读取、哈希或验证同一 face。未用于正文的 face、名称记录和已填实例槽也必须通过检查，空段落和只有控制字符的段落同样适用。

family 绑定只接受 name ID 1/16/21，subfamily 接受 2/17/22，PostScript 接受 6，并核对指定物理记录的实际解码文本。`exactFamily` 必须与已核实的 family 名称精确相同。替代名称需要显式 `substitution` 配置摘要和原因，结果保留该选择。这个摘要仅标识宿主提供的配置，不证明用户授权、字体权利或视觉等价。

找不到名称或对应样式槽会报错；不隐式回退到其他样式、不合成加粗/倾斜。宿主仍须正确选择具有对应视觉样式的字体实例；当前名称/轴校验不能证明宿主把 regular face 放入 bold 槽就是视觉正确。当前每个样式选择一个明确候选，缺字仍由已有塑形结果报告；后续须结合既有多候选回退、语言/脚本与目标应用配置构建完整资源选择。

默认清单上限为 32 个资源绑定、256 个 face、256 个 typeface、1 MiB 累计名称/轴标签/配置说明，每个实例最多 64 个轴；段落复用既有文字、样式、资源字节等限制。复制到结果中的替代说明另作累计限制。源字体名称求值默认限制名称长度 4096 字节、待查 supplemental 项 4096 个，并在扫描时检查取消。这些是操作边界，不是产品峰值 RSS 或安装包预算。

完整设计中的 FontManifest 还包括字体交付/嵌入、授权与可编辑性策略、真实覆盖回退和宿主资源生命周期。本阶段只完成上述计算子集；不得将这个 draft 清单视作完整产品 FontManifest 已交付。

## 真实执行与独立验证

新增 15 项 Rust 测试，其中 10 项读取/改写真实 PPTX，5 项验证清单绑定、轴、未使用项、资源冲突、预算、取消与控制文字。此前 24 份样式语料的包及结果共 48 个文件逐字节不变。全仓 428 项 Rust 测试、严格 Clippy、格式检查、mo-pptx/mo-text 的 WASM 目标编译和 68 份 Schema 生成检查通过；一份来源响应 Schema 及相应 TS 分片扩展了字体保留序号。

独立 ZIP/XML/MCE 参考覆盖 30 个请求、27 份不同的 PPTX：21 个名称结果、9 个诊断、19 个主题绑定、3 个脚本补充绑定和一个真实主题覆盖部件。参考独立选择 XML 声明并核对物理来源。固定官方 XSD 校验 121 次参与部件：119 次通过；两份故意非法探针为 spPr 内的假 fontRef 和 majorFont 上的未知属性。参考不证明 Office/WPS 的字体选择行为。

开发示例 [`manifest_shape`](../../crates/mo-harfbuzz-sys/examples/manifest_shape.rs) 在独立开发进程内调用真实 Native HarfBuzz。原创合成字体的 `A A` 得到 glyph ID 2/1/2，advance 均为 38400，单位为每 em 64000；仅验证一个 face、调用一次组件。另一套 Python stdlib SFNT 解析从 name、cmap、head、hmtx 读取预期名称、字形和宽度，与真实输出核对。该字体用于验证资源/计算，不是视觉质量样本；这是单个 Native 探针，不是新增 Native/WASM 对比或页面排版验收。

在仓库根目录、已准备固定的 Native HarfBuzz 组件和 ECMA XSD 缓存后运行：

```sh
mkdir -p .codex-work/text-fonts
MO_NATIVE_FONT_EVIDENCE_DIR="$PWD/.codex-work/text-fonts/fixtures-final" cargo test -p mo-pptx --test text_fonts --locked
python3 tools/verification/native-font-reference.py
cargo test -p mo-text --lib --locked
cargo run -p mo-harfbuzz-sys --example manifest_shape --locked > .codex-work/text-fonts/native-manifest.json
python3 tools/verification/manifest-shaping-reference.py
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo check -p mo-pptx -p mo-text --target wasm32-unknown-unknown --locked
```

XML 参考使用既有 lxml；字体参考只用 Python 标准库。本轮没有增加运行时依赖或改变锁文件，只为开发示例声明已有的 serde_json 开发依赖。冻结证据见[字体库级检查](../reviews/evidence/2026-09-25-native-fonts-library-verification.json)；此前的 10064 批仍只认证旧的[运行时产物](../reviews/evidence/2026-09-25-text-body-verification.json)，不能转算成新库能力覆盖。

后续实现已将清单资源跨段落复用，并连接自动布局与字形轮廓，见[清单布局](manifest-layout.md)；上述 428 项及单个塑形探针保留为本阶段证据。下一步将来源 run、字体槽/语言、字号/基线、段落布局和文字画笔统一连接到页面编译，再执行实际双端绘制与目标应用对照。当前来源页面仍拒绝含文字的可见对象；autofit、多栏、竖排、项目符号、字段、完整高级对象与播放，以及 Musterwork E0–E3 均需继续完成。本轮没有新的发布构建、桌面完整包体积或产品性能结论。
