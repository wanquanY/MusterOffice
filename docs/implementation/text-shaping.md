# Rust 文本塑形与隔离宿主

2026-09-24 · 已把固定 HarfBuzz 组件接入 Rust 文本计算。`mo-text` 使用真实字体资源，形成有上下文的塑形批次，并检查返回的每个字形。Native 工作进程与 Rust WASM＋独立 C++ WASM 已完成同一输入的实际对比。此处仍是 run 塑形，不是字体选择、段落/页面排版或演示渲染。

## 计算与协议

`ShapeRequest` 由字体 SHA-256、face index、全文逻辑字符串和 run 数组组成；二进制字体独立传递。每个 run 显式给出全文 Unicode scalar 的半开范围、方向、script、语言、cluster level、flags、features、variations 和字形预算。UTF-16 与字节下标不能用作 run 或 feature 坐标。请求和响应从 Rust 生成 [Schema](../../contracts/generated/shape-request.schema.json) 与 [TS](../../packages/contracts/src/generated/shape-request.ts)，没有第二套手写文档模型。

一批请求先检查大小，再由 `mo-font` 核对一次字体身份、目录和所消费元数据。`mo-text` 校验所有 run/feature/axis 后才调用后端；C++ 不负责用户文稿语义。私有批次一次传递一份字体与所有 run 的计算帧，C++ 仍逐 run 执行固定 ABI。WASM 薄层只负责复制、调用与生命周期，不实现第二套排版规则。

可变轴输入 `value1616` 是精确的有符号 16.16 整数。先按字体原始轴范围校验，再舍入为 HarfBuzz 接收的 binary32；响应同时返回 `requested1616` 与 `effectiveF32Bits`。例如 `400 × 65536 + 1` 可能舍入到恰好 400，调用者可以看到该差异。重复、未知和越界轴均拒绝，不静默 clamp。未指定轴使用摘要绑定字体自身的默认值；输出列表记录显式设置的轴。

成功响应保留字形 ID、全文 scalar 字簇、三个 glyph flags 和四个 i32 位置；固定精度为 1/64 字体设计单位，`positionUnitsPerEm = unitsPerEm × 64`。Rust 校对每个输出头、长度、UPEM/scale、glyph/cluster 范围、flags，并按所选 level/方向校对单调字簇。缺字 ID 0 保留，同时给出 `missingGlyphClusters`；计算成功不等于字体覆盖或视觉质量已经通过。任何 run 失败都不返回此前 run 的部分结果。

默认总文本最多 65536 scalars、256 runs、累计上下文 1048576 scalars、累计输出 262144 glyphs；另有私有帧总量限制。C++ 每个 run 当前仍带完整上下文，复制和重复字体对象解析尚需优化。Rust 不保留资源/字体缓存；后续宿主资源租约与字体上下文缓存必须另计成本。

## 原生边界

`mo-harfbuzz-sys` 是唯一新增的原生 unsafe 边界，unsafe 只在私有 `ffi.rs` 中允许；其余核心继续禁止 unsafe。构建检查固定组件 lock、构建 profile 和实际静态库 SHA-256，拒绝故障注入库、sanitizer 库或未经配置的交叉目标。不自动下载或选择系统 HarfBuzz。

`mo-text-worker` 独立进程持有 C++ 实例。输入为两项 little-endian u32 长度，再跟请求 JSON 和字体 bytes；响应为 u32 长度和 JSON。Worker 可连续处理多批请求，拒绝畸形/超限帧；组件失败、非法输出或失效会使实例不可复用。失效不仅记录在 Rust wrapper，还写入 C++ 全局失效标记；重建 Rust wrapper 不能清除它。工作进程不接收输入文件路径。

`mo-cli shape-text` 从宿主显式路径读取资源，启动同目录的 worker，清空子进程环境，完成一批任务后关闭输入并回收进程。当前开发宿主有 30 秒硬超时；阻塞输入/输出时仍由父进程 kill/wait，已验证不会留下该子进程。CLI 每批启动进程；生产池、用户取消接口、跨平台 OS 权限/RSS 限制仍待实现。清空环境和独立进程不等于 OS 沙箱。当前实际验证平台为 macOS arm64；Windows 组件构建尚未实现。

## WASM 边界

Rust 导出 `shape_text(request, fontBytes, component)`，组件实现同步 `shapeBatch`/`invalidate`。`packages/text-component` 通过宿主提供的已校验模块与 factory 创建独立实例；没有隐式下载、路径读取或系统字体。导入表只允许当前内存增长函数和三个必须拒绝的 fd 函数，后者执行即抛错。所有 heap 视图都在使用时重新取得，避免 memory growth 后访问失效视图。

字体每批向 C++ 内存复制一次，结果按批返回 Rust 做语义核对。分配失败、trap、异常或非法输出废弃实例，之后必须明确新建。实例释放不会销毁独立的 Rust WASM；已验证换用新组件后同一 Rust 内核可以继续计算。此处仍在 Node 测试宿主运行，浏览器 Worker 的调度、硬取消与模块下载完整性策略尚未完成。没有把同步 WASM 消息取消冒充为已实现。

## 开发与实际验证

先按[组件说明](harfbuzz-component.md)构建 `.codex-work/harfbuzz/release` 的无注入 Native/WASM 组件；Rust build 可用 `MO_HARFBUZZ_LIB_DIR` 显式指定其他已核对目录。然后执行：

```sh
cargo build -p mo-cli -p mo-text-worker --release --locked
cargo build -p mo-wasm --target wasm32-unknown-unknown --release --locked
node tools/contracts/build-wasm-node.mjs
pnpm exec tsc --project packages/text-component/tsconfig.json
node tools/verification/text-parity.mjs
target/release/mo-cli shape-text .codex-work/text-shaping/request.json '.codex-work/font-corpus/notosans/NotoSans[wdth,wght].ttf'
```

验证脚本还需要上一轮的固定多语种字体、组件语料和故障注入测试构建，见同一组件说明；注入接口不进入普通 worker 或常规模块。

[本轮证据](../reviews/evidence/2026-09-24-text-shaping-verification.json)记录 57 组实际 Native/WASM 完全一致响应（38 成功、19 拒绝）；33 组成功输入的 303 个 glyph 与独立组件逐字段核对。覆盖阿拉伯文、天城文、中文竖排、单色 Emoji、UVS、可变轴、TTC/CFF、空内容、上下文批次、量化、未知字段和预算。额外验证原生同进程连续 3 批与 2 个畸形帧、CLI 实际进程入口，以及四个真实 C++ 分配失败位置经 WASM 薄层与 Rust 接口返回错误和拒绝复用。

新增边界和超时测试后共 115 项 Rust 测试通过，严格 Clippy、格式化、16 份 Schema/TS 和薄层/生成 WASM 声明类型检查通过。此前 389 组 Native/WASM 回归在新主内核上重跑通过，合计 446 组；旧的独立组件 45 组不再重复计入主内核总数。测试没有证明字体布局程序净化、全部复杂字体、浏览器并发、Office/WPS 或完整性能/体积预算。

下一步进入字体资源/主题别名选择、script/bidi 分段、grapheme 与合法断行、字号/行度量和页面布局，同时落实生产 Worker/进程池与完整资源回收。动画、图表、SmartArt、数学、媒体、公开 Agent 协议和 Musterwork Artifact 接入仍按完整目标推进；本轮没有关闭 E0/E1/E2/E3。
