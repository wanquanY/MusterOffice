# 原生编辑页面交互

状态：P1 的页面文字交互计算与 Native/WASM/TS 边界已实现；完整编辑产品继续实施。
本接口针对指定文本框或实际显示的表格单元格。视觉光标移动现已接通，见[文字移动合同](text-caret-navigation.md)。[对象拾取计算](editor-object-picking.md)已接入 Rust 页面 owner；它的公开会话/客户端接口、组选择策略、IME 和产品 UI 继续实施。

## 同一次排版和渲染

`mo-text` 的编辑计算同时保留路径、精确行框和交互 map；字形塑形、换行搜索、度量只执行原有的一次流程。
普通预览不分配交互 map，也不新增字体 GDEF 查询。`SourceFrameEditor` 将段落光标平移到实际文本框坐标，
包含首行缩进、左右对齐、段前后间距、垂直锚点和原生内边距。跨段落选区保留用户 anchor/focus 方向，
以显式 `paragraphBreaks` 表示段落分隔，不虚构其字符位置或字形宽度。

`SourceEditorPage` 通过既有 resource page 编译器接入图片、表格、绘制层级、字体与光栅化。
私有 owner 持有同一次计算的文本绑定、map 与原生 placement；调用方不能用 JSON 拼接另一页的几何。
查询不重新塑形、解码或光栅化，不保留外部可变字节引用。

光标返回实际变换后的线段，选区返回四个有序角点。原生水平/垂直 overflow 先在文本框局部坐标裁剪，
然后应用与文字路径一致的 anchor rebasing、旋转、翻转、组变换和精度证明，不把旋转选区替换成轴对齐外包框。
`visible` 仅表示原生文本框的轴裁剪；画布/视口裁剪仍由宿主显示层负责。
`coordinateErrorBound` 使用与字形 placement 相同的误差上界，单位为 Q32 页面 EMU。

命中点输入为页面 EMU，使用有界大整数表达式精确求逆，最后一次舍入，避免先舍入逆矩阵系数造成大坐标误差。
精确奇异变换返回无 caret 且 `inside=false`；不可表示坐标显式失败。文本框内先按已平移行框距离选行，再复用
段落 advance-cell 命中策略。它不是 glyph 轮廓拾取，也不决定当前应选择哪个对象。

## 会话和合同

`mo_kernel_api::EditorPageSession` 每个 owner 保留一个不可变页面。输入包括：

- `pptx`：检查原始 OPC 内容及来源摘要。
- `author`：直接使用 `AuthorPlan`，不通过导出再导入创建编辑页面。
- `retained`：使用 `SourcePlan` 验证原始包和允许修改的字段覆盖。

`prepare` 必须携带显式 FontManifest。图片与字体摘要由现有资源层校验；仅完整渲染和元数据构建成功后替换 owner。
`view` 是准备请求的领域摘要，包含语义/来源身份、字体清单、视口和采样方式；它不是权限凭据。
`query`、`pick` 和 `clear` 必须匹配当前 view，且不能携带材料或字体字节。失败不返回部分结果，也不替换旧页面。
`clear` 后旧查询失效；重新准备完全相同的不可变输入可以得到相同 view，这不会改变任何文稿状态。

输出提供页面摘要、视口、资源身份、文字工作量、对象/祖先目录、文本框/单元格及段落文字/字素边界，不将完整字形路径和 map 复制到 JS。
Author/Retained 输出包含实际模型 ObjectId，原始 PPTX 输出保留 native part/id；合并表格的 covered cell 不生成重复编辑框。
对象目录及设备像素 `pick` 合同见[对象拾取](editor-object-picking.md#公开会话与对象目录)，不可与页面 EMU 文字命中混用。

### 段落、文本片段与单元格身份

每段文字的 `model` 在已投影时包含真实 `ParagraphId` 和按原有顺序排列的 `runs`。每项携带 `RunId`、
`scalarStart` 与 `scalarEnd`，区间半开，来自本次排版保留的 `SourceScalarRange`，不会按 UTF-16 或 XML 发现序号推算。
元数据构建同时校验模型文字、片段种类、次序及 scalar 长度与实际计算输入一致；不一致使整个准备失败并保留旧页。
空片段保留零长区间；样式边界可能位于组合字素内部，因此片段边界不是合法光标边界，输入仍须使用 `boundaries`。

原生 `a:br` 在计算输入中使用共享定义 `NATIVE_SOFT_BREAK`（U+2028），占一个 scalar 和一个 UTF-16 单位。
普通 `a:t` 内容保持原样；不能全局替换换行符。排版 `text` 的 UTF-8 字节位置不等于模型编辑偏移，
产品先按返回的字素表转换到 scalar，再使用模型段落身份形成编辑命令。

自建形状按当前文本框段落绑定。自建表格按实际物理行/列找到模型单元格，返回稳定 `cellId`；
已有文字仍用段落/片段身份。没有文字模型的自建空单元格返回真实 `cellId`、`model=null`，
不为排版合成的空段落制造 ID，初始化文字使用既有单元格事务能力。
Retained 使用渲染过程中已确定的 `native_paragraph` 范围映射到对象的原生物理段落序列，
不会将各单元格的局部第零段都映射到对象第一段；Retained 不具有自建 `cellId`。

原始 PPTX 没有模型身份；V1/V2 保留导入 profile 的表格文字仍是未投影内容。这两类段落保持 `model=null`，
查询不升级 profile、不创建可写模型。身份绑定只说明文字属于哪个对象，不授予编辑权限；
原生文字保护约束和高层文字命令支持范围仍由编辑内核决定。

身份验证见[执行记录](../reviews/evidence/2026-10-03-editor-text-identity.json)：27 份真实输入、752 条 Native/WASM 消息，
包括 30 个模型段落、29 个片段身份、6 个自建单元格和 29 个明确未投影段落。缩放、清理和重新准备保留正确身份。
旧文字导航的 18 份样本、679 条消息及 3,120 次移动查询继续通过。上述检查不替代从产品输入到正式保存的验收；
公开文稿页映射/准备身份发现、高层表格与保留文字命令、共享编辑器和固定 SDK 接入仍须完成。

Native Worker 使用 `--editor-page-session`，请求头为三个 LE u32（JSON、材料、字体长度），随后三个字节通道；
回复头为两个 LE u32（JSON、RGBA 长度），随后两段内容。请求 32 MiB、材料/字体各 128 MiB；已有包、图片和渲染预算继续适用。
WASM `EditorPageSession` 使用同一个 Rust owner；`PresentationEditorPage` 是同步薄接口，应在宿主 Worker 内调用。
关闭可以幂等调用；回调内关闭延后到外层 Rust 借用返回，拒绝发布晚到结果。桥接异常释放 owner；类型化计算失败保留旧 owner。

每页最多保留 262,144 个交互 cell、16,384 行；每批最多 64 查询、1,048,576 工作项和 65,536 选区片段（含段落分隔）。
预算跨本页所有文本框累计。字体组件已有坏回复隔离和取消流程继续生效。公开合同由 Rust → JSON Schema → TS 生成。

## 验证和限制

可执行证据：`crates/mo-harfbuzz-sys/tests/{text_frame_interaction,source_editor_page}.rs`、
`tools/verification/editor-page-parity.mjs`、`tools/verification/editor-page-client-tests.mjs`。
本机 Native 测试比较普通渲染与编辑渲染的每个 RGBA 字节，覆盖图片、旋转/翻转/嵌套组、原生轴裁剪、
九种对齐/垂直锚点组合、GDEF/GPOS、跨段落反向选区、合并表格、空单元格、Unicode EGC/bidi、取消和跨框预算。
表格测试保留原始边框冲突拒绝，并用显式一致边框的样本验证单元格交互。

Native/WASM 验证使用真实 HarfBuzz、Skia 和原创固定测试字体，同时核对 TypeScript facade。
合成 Hebrew/emoji 字形只证明坐标和偏移，不证明语言字形质量。环境、计数、构建和输入输出摘要见
[本批验证记录](../reviews/evidence/2026-10-03-editor-page-interaction.json)。
这些结果不替代真实浏览器/Electron、Windows/Linux、IME、性能/内存、Office/WPS 或最终产品验收。
本批尚未升级 Musterwork 固定开发 SDK。

## 复现

先配置已验证的 HarfBuzz caret 与 Skia Native 组件绝对路径，再执行：

```sh
MO_EDITOR_PAGE_FIXTURES="$PWD/.codex-work/page-interaction/fixtures" cargo test --locked -p mo-harfbuzz-sys --test source_editor_page --test text_frame_interaction -- --test-threads=1
cargo test --locked -p mo-text -p mo-presentation-compile -p mo-kernel-api --lib
cargo build --locked --release -p mo-raster-worker
cargo build --locked --release -p mo-wasm --target wasm32-unknown-unknown
.codex-work/toolchain/bin/wasm-bindgen --target nodejs --out-dir .codex-work/page-interaction/wasm-node target/wasm32-unknown-unknown/release/mo_wasm.wasm
# Bindings directory needs a CommonJS package.json, as in the paragraph probes.
pnpm build:editor-client
node tools/verification/editor-page-parity.mjs
pnpm test:editor-client
```

对比脚本沿用 `.codex-work/caret/{harfbuzz,text-component}`、`.codex-work/raster-component` 和
`.codex-work/font-fallback-20261003/playback/runtime` 中已验证的组件。脚本不下载字体或资源。
