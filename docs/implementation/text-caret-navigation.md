# 原生文字光标移动

状态：P1 文字移动计算已实现；产品键盘、选区扩展和 IME 宿主继续实施。
移动查询沿用不可变段落、文本框和编辑页面，不产生文稿事务，不重排文字或调用字体/图片/光栅组件。

## 移动合同

`TextQuery::Move`、`FrameTextQuery::Move`、`PageTextAction::Move` 共用 `CaretMove`：

| movement | 语义 |
|---|---|
| `previousGrapheme` / `nextGrapheme` | 按逻辑文本顺序移动一个 Unicode 18 扩展字素；显式控制字符也有逻辑位置 |
| `left` / `right` | 沿渲染器 L1/L2 的视觉字素顺序向文本局部左/右移动；行边界按段落基方向继续 |
| `up` / `down` | 按文本行顺序移动，使用持续的目标横坐标寻找实际光标边；不按字形高度或 baseline 选行 |
| `lineStart` / `lineEnd` | 当前行基方向的起点/终点；RTL 行首位于物理右侧 |
| `textStart` / `textEnd` | 段落 API 的逻辑首尾；文本框和页面 API 的整个当前文本框逻辑首尾 |

每个输入位置必须是有效字素边界。返回 `caret` 包含规范化后的 scalar/UTF-8/UTF-16 边界、
upstream/downstream affinity、所在行及实际几何。页面返回同一变换后的光标线段、原生裁剪和精度证书。
`exhausted=true` 仅表示方向移动已到当前容器边缘；它不是失败，也不授权切换对象、表格单元格或幻灯片。
行首/行尾和文本首尾是定位命令，返回 `exhausted=false`，即使原本已在目标位置。

左右移动复用渲染时已计算的双向文本顺序，并将 scalar 折叠为字素；不按 x 排序，避免负字距和重叠字形改变字符遍历顺序。
软换行保留不同 affinity 的两处位置。移出视觉行边界时进入相邻行的反向物理边缘。
X9 移除控制和无字形 cell 不生成视觉槽；从其逻辑位置开始移动时，先以真实 caret x 锚定最近的可见边，
距离相同优先段落基方向，再按视觉槽稳定选择。控制字符仍可通过逻辑字素移动访问。
空行/空段落使用现有 strut 光标。跨段落移动经过结构分隔但不虚构 scalar 字符。
进入不同基方向的段落时，使用目标段落自己的视觉行首/行尾。

## 连续上下移动

输入 `preferredX` 为可选 Q32 坐标，只接受 `up` / `down`。第一次省略时使用当前光标 x；
输出保留该值，包括移动到短行、空行、段落边缘或容器边缘时。宿主将它传给下一次连续上下移动，
在鼠标定位、非垂直移动或改变文稿后清空。这样经过短行后仍能回到原来的列。

- 段落查询：`preferredX` 位于段落局部坐标。
- 文本框和页面查询：`preferredX` 位于文本框局部坐标，包含实际行缩进与对齐平移。
- 页面旋转、翻转和组变换不改变这个坐标系；移动沿文本轴，返回的可见线段仍应用完整页面变换。

文本框在目标行扣除该行自己的偏移后查找最近 caret，再应用该行平移。不能把上一行的段落局部 x 直接传给下一段。
距离相同优先段落基方向，再优先 downstream 和较小逻辑偏移。`preferredX` 用于非垂直移动时显式拒绝。

## 所有权、预算和验证

段落 map 只保留内部的视觉字素索引，不增加对外可上传的几何或另一套排版来源。
帧/页查询仍由不可变 owner 绑定原始排版、段落、原生 clip 和变换。逻辑查询不修改历史、草稿或文稿。
查询上限、跨框累计工作预算、取消和整批失败语义沿用[编辑页面接口](editor-page-interaction.md)。

验证入口：

- `crates/mo-text/src/interaction/navigation_tests.rs`：双向 affinity、扩展字素、CRLF、软换行、负字距、空行、取消、输入预检及预算。
- `crates/mo-harfbuzz-sys/tests/text_frame_interaction.rs`：真实字体、不同对齐段落之间的 sticky x、空段落、段落方向和结构边界。
- `crates/mo-harfbuzz-sys/tests/source_editor_page.rs`：移动后的页面光标与同位置直接查询相同，包含嵌套旋转/翻转及取消。
- `tools/verification/text-interaction-parity.mjs`：原创字体与已锁定的拉丁、阿拉伯、天城文、中文、emoji 字体，Native/WASM/TS 对等。
- `tools/verification/editor-page-parity.mjs`：原始、自建、保留来源及表格页面，所有有效字素位置与两种 affinity 的十种移动；查询不调用组件。

本接口不负责 DOM 键盘映射、Shift 选区锚点保留、composition、对象拾取或持久保存。
Windows/Linux、浏览器/Electron 的实际输入与性能资格须由完整产品测试单独证明。

本批在 macOS arm64 Native / Node WASM 上通过 239 项核心库、12 项 Native 页面/文本框和 9 项客户端检查；
77 批段落及 18 个页面样本的 679 条 Native/WASM 消息一致。精确输入/输出、源码和构建摘要见
[本批验证记录](../reviews/evidence/2026-10-03-text-caret-navigation.json)。

复现使用既有 HarfBuzz/Skia 固定组件及字体目录；先构建 `mo-raster-worker`、`mo-text-worker` 和 `mo-wasm`，
再将 nodejs bindings 放入 `.codex-work/text-navigation/wasm-node`（CommonJS package scope），
使 `.codex-work/text-navigation/fixtures` 指向原页面样本目录。执行：

```sh
pnpm build:editor-client
node tools/verification/text-interaction-parity.mjs .codex-work/text-navigation
node tools/verification/editor-page-parity.mjs .codex-work/text-navigation
MUSTEROFFICE_EDITOR_WASM=.codex-work/text-navigation/wasm-node/mo_wasm.js pnpm test:editor-client
```
