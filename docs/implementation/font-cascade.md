# 字体资源复用与原子片段候选选择

2026-09-24 · `mo-font::VerifiedFont` 和 `mo-text::cascade` 已接入 Native/WASM。当前完成显式资源、调用方分段的有序字体候选求值。自动 script/bidi 分段、片段内多字体回退、主题家族/样式匹配和段落布局仍需继续实现；本模块不是完整字体选择系统的验收结果。

## 不可伪造的字体资源

`VerifiedFont::load` 校验实际 SHA-256、face、目录/checksum、所用元数据及 cmap 后，持有不可变借用字节、元数据和已检查的 cmap 表。字段私有，不能从 JSON `FontInspection` 构造，也不能修改字节后继续使用旧核对结果。`metadata` 只读，`query` / 无分配的 `query_one` 复用所选表，仍检查 scalar/selector、查询预算和取消。

`mo_text::shape_verified` 接受该资源，并再次核对请求摘要与 face；它不重复扫描字体。既有 `inspect` 和 `shape` 则包装相同实现，保持原有接口及输出。资源对象不证明轮廓/GSUB/GPOS 都合法，不授予字体分发或嵌入权利，不能代替隔离的字体执行组件。

## 候选求值合同

开发入口：Rust `mo_text::cascade::shape_cascade`、WASM `shape_cascade(requestJson, bundle, component)`、CLI `mo-cli shape-cascade request.json fonts.bin`。CLI 仍通过同目录的隔离 worker 执行；worker 的 `--cascade` 模式使用同一有界帧协议。没有系统字体扫描、网络、核心文件路径或 JSON base64 字体。

`CascadeRequest` 包含全文 `text`、字体 bundle 的 `fonts` 绑定，以及有序、不重叠的 `items`。每个 item 显式给出 scalar 范围、方向、script、language、features、上下文标志、glyph 预算和有序候选；每个候选绑定字体索引及自身可变轴坐标。偏移/长度使用 uint64 十进制字符串，先在 uint64 域校验范围，再缩窄为目标地址宽度，避免 32 位 WASM 与 64 位 Native 的拒绝行为不同。

所有输入资源和候选参数先校验，避免未被选中的错误轴或错误资源随着首选字体覆盖变化才偶然暴露。相同 bundle 范围和 face 的多个绑定共用一次已验证资源；不同 face 分别验证。同一批次的不同 item/候选复用该对象，不重复做 Rust 字体检查。当前没有跨请求字体注册表，也没有 C++ face/font 缓存；HarfBuzz 每次仍构造自身对象。

求值按以下顺序进行：

1. 使用固定 Unicode 18 扩展字簇，拒绝切开组合字或 Emoji 字簇的 item 边界。边界合法只是最低检查，调用方仍须提供正确的 script/bidi/样式分段；不能据此任意拆开一个复杂文字片段。
2. 每次对整个 item 塑形，保留同一全文上下文，固定 monotone-graphemes、默认忽略字符行为及 unsafe-to-concat 输出。候选内不按字符拆字形，不拼接来自不同候选的半个连字。
3. 收集实际 `.notdef` 字簇及未解决的 variation sequence。首个没有这两类问题的候选返回 `selected`；此前失败尝试的字体索引与问题仍保留。它表示此 profile 的字形可用性通过，不是视觉质量证明。
4. 没有候选满足时返回 item 的 `unresolved` 和全部尝试；不偷偷选最后一个字体、不丢字符、不用截图冒充成功。组件故障、非法输出、资源冲突、超预算及取消直接失败，整批不返回部分选择。

当前 profile 是 `atomic-item-glyph-availability-ucd18-hb14.5-cmap14-v1`。variation sequence 采用保守的 cmap format 14 证据：支持 default/non-default 映射，缺失映射和孤立 selector 保持未解决。因此通过 GSUB 处理、没有相应 cmap-14 记录的合法变体仍可能未解决；这必须继续由变体/Emoji/复杂文字策略补全，不能宣称一期完整支持。没有 `.notdef` 也不能证明 Emoji ZWJ 已获得期望的组合外观、所选字体审美正确或 Office/WPS 编辑结果一致。

仅凭基础 cmap 不足以预先淘汰候选：例如自有字体没有 `Á` 的 nominal glyph，但固定塑形器可用 `A` 和组合尖音符完成分解塑形。本轮实际用例验证保留该候选。相反，字体名称也不能代替覆盖检查：固定 Noto Sans 样本包含天城文字形及对应布局脚本，按输入顺序可先于专用家族被选中。

上层现已接入[混合字体回退与重塑](font-fallback.md)，独立原子入口仍保持本文件的整项选择语义；[HarfBuzz 的上下文与 unsafe 标志说明](https://harfbuzz.github.io/harfbuzz-hb-buffer.html#hb-glyph-flags-t)不能简化为“单侧没有 unsafe 标志即可任意拼接”。跨字体混排、bidi 重排、断行、度量缩放、基线和视觉质量必须在后续段落管线一并验证。

## 预算与隔离

默认最多 128 MiB bundle、32 个字体绑定、256 个 item、每 item 32 候选、1024 次实际尝试、累计 1048576 个上下文 scalars / 探测 glyphs、262144 个选中字形；全文仍受 65536 scalar 限制。结果同时记录 verified faces、塑形调用、上下文和探测字形数量。范围、预算与取消属于核心；Native 超时回收、WASM 独立模块失效策略继续复用[塑形宿主](text-shaping.md)。生产进程池、浏览器 Worker、完整内存记账尚未完成。

## 验证和局部测量

```sh
node tools/verification/cascade-parity.mjs
node tools/verification/cascade-benchmark.mjs
python3 tools/verification/contracts.py --cascade-report .codex-work/font-cascade/parity.json
```

[本轮证据](../reviews/evidence/2026-09-24-font-cascade-verification.json)绑定 43 组真实 Native/WASM 用例，涵盖候选顺序、Latin/Arabic/Devanagari/CJK/Emoji、竖排、组合字、全局上下文、TTC face、轴量化、UVS、未解决片段及错误边界。23 个选中 item 的 94 个字形既与独立单字体入口一致，也与未经包装的同版上游 `hb-shape` CLI 一致；这是塑形数值核对，没有栅格或目标应用验收。

真实故障注入在首字体正常返回缺字后，让第二字体的 C++ 分配失败，验证整批失败、模块失效、拒绝复用及替换实例可正常工作。既有 467 组跨端用例在新产物上回归，合计 510 组；Rust 共 128 项测试、20 份 Schema/TS 类型，严格 Clippy 和类型检查通过。

一次专门测量在报告所列 arm64 macOS / Node 23.5.0 环境运行：17,772,300 字节的固定 Noto Sans SC 字体，8 个显式中文 item、相同全文上下文，内存中预热后交错执行 9 对样本。8 次独立 `shape_text` 调用中位约 530.04 ms，一次 `shape_cascade` 中位约 67.71 ms，字形逐项相等。前者包含 8 次字体上传与校验，后者仅一次，同时多做候选诊断；C++ 塑形次数相同。原始样本和环境在证据中，不能把约 7.8 倍的这项局部收益外推为整页排版、冷启动、跨设备表现或 Musterwork 性能。

后续已连接自动 script/bidi 与片段内字体切换/重塑，见[自动分段](text-itemization.md)及[混合字体回退](font-fallback.md)。完整 FontManifest/家族样式/主题字体映射、GSUB/Emoji 变体策略、断行/段落/竖排布局、发行字体权利与嵌入、持久资源和字体缓存仍待落地。E0/E1/E2/E3 仍未关闭。
