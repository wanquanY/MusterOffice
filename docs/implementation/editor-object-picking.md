# 原生编辑页面对象拾取

状态：Rust 页面对象拾取、Native/WASM 几何组件、公开会话协议、模型身份与组关系目录及 TypeScript 客户端已实现。产品输入接入继续实施；尚未升级 Musterwork 固定 SDK，不代表完整编辑产品验收。

## 同一份渲染几何

`mo-raster::CompiledRaster` 在原有编码过程记录路径、描边、裁剪和绘制区间。编辑页面从已经通过精度检查的设备批次提取 `CompiledPicking`；不重复生成路径，不重新解码图片或排版文字。普通渲染只多保留区间，不复制拾取批次。

`SourceEditorPage` 持有这份私有不可变几何和编译器生成的对象索引。绘制实例对应原始 `paint_sources`，对象以 `(part, nativeId)` 去重；背景不成为对象。结果按页面原有对象绘制顺序从上向下返回，表格的填充、边框和单元格文字归于同一个对象。命中带有实际绘制实例、绑定及可选的文本框索引，供后续文字定位使用。相同对象同时有精确与邻近命中时保留精确命中。

Skia 拾取扩展共用渲染器的路径构造、描边参数、线端和 miter-clip 实现。`exact` 对填充区域或描边轮廓执行包含查询；填充的开放轮廓显式补上原生隐式闭合边，描边保留开放线端。零宽描边按一设备像素的几何轮廓处理。它不声明与抗锯齿像素 alpha 完全等价。

`nearby` 将填充轮廓或描边轮廓按指定半径扩张，再要求指针处于原有裁剪交集内。扩张采用 round cap/join；不会扩张页面、图片或文字裁剪。该操作是轮廓容差查询，包括孔洞、内部轮廓和退化轮廓，不是到最终可见像素集合的距离计算。透明填充仍有几何身份；画笔颜色、图片 alpha、组透明度和合成遮挡不参与几何拾取。

## 空文本框与裁剪身份

空文本框或文字留白使用同一次排版的原生 `region.outer`，包含原生文字区域的内边距。页面指针使用实际送入组件的同一个 float32 采样值，以 Q32 表达后按精确有理式逆映射到页面，再应用既有原生 placement 的精确逆变换。旋转、反射和嵌套组沿用文字交互变换；奇异变换不生成语义区域命中。语义区域只产生精确命中，不额外扩张矩形。

文字编译器记录每个交互框实际使用的裁剪引用。场景编译器仍会删除无使用者的节点，因此编辑准备通过 `compile_images_retaining_clips` 显式保留这些交互使用者，并取得原场景到设备裁剪的索引映射。空框即使没有任何字形绘制也保留其裁剪。普通场景沿用原来的精简行为，不靠插入透明绘制或伪造字形保留资源。

每个设备查询同时返回裁剪交集位图，包括视口位。语义文本区域复用这些实际设备裁剪结果；页面取整产生的最后一个部分像素也受原文档边界限制。渲染和查询不会各自重建另一套裁剪算法。

## Rust 页面合同

`SourceEditorPage::pick` 接受最多 64 个 `PagePickQuery`：

- `device.point`：当前准备视口中的 Q32 设备像素，不是 CSS 像素或页面 EMU。
- `device.radius`：Q32 设备像素，范围为 0 至 32；不接受负数。
- `maxHits`：1 至 256 个唯一对象；超出时返回 `truncated=true`。

页面文本命中 `PageTextAction::Hit` 仍使用页面 EMU，两者坐标合同不能混用。宿主需根据实际显示缩放转换指针。对象拾取不修改文稿，不决定组选择策略、编辑权限、当前页或保存操作。Native part/id 与模型 ObjectId 的映射由下述公开会话提供。

设备坐标范围为 ±32,768，float32 采样误差不超过 1/256 设备像素。`exact` 表示此设备几何在无半径扩张时命中，不声明无限精度或抗锯齿覆盖等价；页面原有几何误差证书仍适用。

输入先经过数量、范围、精度和工作量检查。结果整批成功或整批失败，不返回已完成前缀。组件调用前的输入错误和取消不影响实例；组件异常、坏回复、分配失败或调用后验证阶段取消会使组件失效。TS 返回独立的 `Uint32Array`，拒绝共享输入、组件内存别名、重入和异常输出所有权；失效后由宿主销毁 Worker/进程。

## 公开会话与对象目录

`EditorPageSession` 增加 `pick {view, queries}`，成功返回 `picked {view, results}`，无像素或材料字节。
它要求当前不可变 `view` 和几何组件；旧视图、超限查询及附带材料的非准备请求均拒绝。准备失败不替换旧页，
`clear` 使旧页失效；重新准备完全相同输入可以得到相同内容身份。WASM `pick(request, raster)` 只接收拾取组件，
文字 `command` 不调用组件；Native worker 复用同一 Rust dispatcher。

`prepare.info.objects` 是当前页的绘制目标及其组祖先目录，最多 8,192 项。它来自经过验证的原生 `SourceIndex`，
按实际图层及原生树的绘制前序排列。只保留本页使用的对象和祖先，不是完整文稿树，也不加入隐藏对象、空组或背景。
每项包含 `object {part,nativeId}`、`objectId`、`name`、`kind`、`surface` 和 `parent`。
`parent` 是同目录中更早的 group 索引；顶层为 null，原生 shape-tree 根不是用户组。
Author 输入通过 `AuthorPlan` 绑定模型 ID，Retained 输入通过验证后的来源绑定获取模型 ID；映射缺失会使准备失败，
不会返回看似可编辑但没有身份的对象。原始 PPTX 输入没有模型，`objectId=null`。

每个结果含 `hits` 与 `truncated`。命中只公开 `object`（本目录索引）、`kind`（exact/nearby）和可选 `textFrame`
（同一准备结果的文本框索引）。私有绘制序号与编译器绑定不进入公开回复。对象索引只在对应 view 内有效；
宿主依据稳定模型 ID 提交编辑，依据 `surface` 和模型能力决定允许的操作，目录本身不授予写权限。
文本框索引可用于后续页面 EMU 光标/选区查询；表格多个单元格保留各自文本框，拾取结果仍按原生表格对象去重。

`PresentationEditorPage.pick(queries, raster)` 保留同步薄计算边界，适合在产品 Worker 中调用。
客户端验证回复视图、批次数、截断数、对象范围与去重、命中类型及文本框所属对象。类型化拒绝保留页，
桥接异常或坏回复关闭 owner；组件回调中关闭会延后释放 Rust 借用并拒绝返回晚到结果。重入调用直接拒绝。

## 几何扩展 ABI v1

`mo_skia_pick_abi()` 返回 1。`mo_skia_pick` 接受 u32 批次，成功返回独立分配的 u32 结果，以 `mo_skia_free` 释放。它共用原光栅实例的故障隔离域。

输入依次为：

1. 10 字头：`0x4d4f504b, 1, width, height, paths, commands, strokes, clips, draws, queries`。
2. 路径：`fillRule, commandCount` 与每条 7 字设备路径指令，与原光栅路径格式一致。
3. 每项 4 字的描边；每项 4 字的裁剪（parent、path、x、y）；每项 5 字的绘制（path、x、y、stroke、clip）。路径引用为零起始，描边与裁剪引用为一开始，0 表示填充或视口。
4. 每项 3 字的查询（x、y、radius），数值为 float32 位模式，由 Rust 验证 Q32 到设备值的转换。

成功输出头为 `magic, 1, queries, draws, drawStride, clips, clipStride`。每个查询依次携带精确绘制位图、邻近绘制位图和裁剪位图；`clipStride=ceil((clips+1)/32)`，bit 0 表示视口，后续位表示设备裁剪交集。两种绘制命中位互斥，最后一字的未使用位必须为零。Rust 拒绝长度、头部、尾位、零半径邻近位和视口外非零位等坏回复。

失败状态沿用 1 输入错误、2 分配失败、3 工作/范围限制、4 实例失效，均必须返回空输出。批次最多 4,096 路径、262,144 指令、4,096 描边、8,192 裁剪、65,536 绘制和 64 查询。查询工作限制为 8,388,608；生成描边/容差轮廓累计上限为 1,048,576 verbs，路径/描边组合上限为 8,192。结果最多 `7 + 64 × (4096 + 257)` 字。

这些是输入、结果和累计工作限制，**不是最坏延迟保证**。C++ 同步调用中不能中途执行宿主取消；生成轮廓的上限在该次 Skia stroker 返回后检查。每批仍会重建组件侧路径与描边，当前没有跨批 C++ 几何缓存。真实浏览器/Electron 的交互延迟、峰值内存和平台表现须单独测试。

## 验证

- Rust 几何单测验证批次投影不受透明度/合成影响、位图顺序、空结果、坏回复、输入预检及取消；场景单测验证裁剪保留和索引重排。
- Native 组件测试验证孔洞、线端、各类尖角、曲线、隐式闭合、透明绘制、超大坐标重基、多字位图和工作放大拒绝。
- 原生页面测试验证曲线外包框内的空白不选中曲线、重叠对象顺序、截断、旋转文本、表格去重和单元格身份、空框与分数页面裁剪。
- `raster-picking-parity.mjs` 比较真实 Native/WASM 几何结果，并与重构前 WASM 组件及 Native 对比像素；可传入带 ASan/UBSan 的 probe 执行相同有效和无效输入。
- `raster-picking-client-tests.mjs` 检查 TS 分配/释放、内存增长、缺失能力、输入/输出别名、异常状态、重入和失效隔离。

几何基础执行记录见[首批证据](../reviews/evidence/2026-10-03-editor-object-picking.json)。后续公开会话对等、模型/组目录及客户端验证见
[会话证据](../reviews/evidence/2026-10-03-editor-page-picking.json)。真实 UI 组选择/输入、性能、Office/WPS 和最终产品验收尚未通过。

```sh
# 使用含 picking 扩展的固定 Skia 组件及既有 HarfBuzz caret 组件。
cargo test --locked -p mo-raster -p mo-render -p mo-presentation-compile --lib
MO_PICK_FIXTURES="$PWD/.codex-work/object-picking/fixtures" cargo test --locked -p mo-skia-sys --test picking
cargo test --locked -p mo-harfbuzz-sys --test source_editor_page
pnpm build:raster-component
node --test tools/verification/raster-picking-client-tests.mjs
node tools/verification/raster-picking-parity.mjs .codex-work/object-picking BASELINE_COMPONENT_DIRECTORY OPTIONAL_ADAPTER_ASAN_PROBE
```

完整可复现组件构建继续使用 `tools/components/build-skia.py`。本机开发验证复用了哈希已核对且源代码未变的 Skia/codec 核心静态库，只重编译适配层，记录了复用来源和本次真实命令；未宣称本批重跑完整核心构建。ASan/UBSan 证据仅覆盖此次插桩的适配层与 probe，复用的核心库没有插桩。

公开页面验证使用新建 fixture 目录、当前 Rust Native worker/WASM 和同一固定字体。配置上述组件路径后：

```sh
MO_EDITOR_PAGE_FIXTURES="$PWD/.codex-work/page-picking/fixtures" cargo test --locked -p mo-harfbuzz-sys --test source_editor_page
cargo build --locked --release -p mo-raster-worker
cargo build --locked --release -p mo-wasm --target wasm32-unknown-unknown
.codex-work/toolchain/bin/wasm-bindgen --target nodejs --out-dir .codex-work/page-picking/wasm-node target/wasm32-unknown-unknown/release/mo_wasm.wasm
# WASM bindings 目录须包含 {"type":"commonjs"} package.json。
pnpm build:editor-client
node tools/verification/editor-picking-parity.mjs .codex-work/page-picking
```

脚本默认使用 `.codex-work/object-picking/skia`，可通过 `MO_EDITOR_PAGE_SKIA` 指定组件目录；
HarfBuzz 默认沿用 `.codex-work/caret/harfbuzz`，可通过 `MO_EDITOR_PAGE_HB` 指定。
24 份样本的 1,577 个初始拾取点、重复/截断/失败/替换/清理路径共比较 668 条 Native/WASM 消息，含所有像素回复；
覆盖 shape/picture/group/connector/graphicFrame、slide/master/layout、6 个父组关系、386 个文字命中、50 个邻近命中和 113 个截断结果。
这些计数仅描述测试输入，不表示产品支持范围或性能指标。
