# 共享 GPU 帧编译与浏览器执行

2026-10-08。已实现并完成以下专项验证，完整视频目标仍执行中。此增量不代表 Browser 工程播放器、媒体转换、字幕或导出已经完成。

## 实现与职责

`mo-video-render` 的 `LinearSource` 将源图元数据与像素载体分开。CPU 和原生 wgpu 保持原有 `Arc<LinearImage>` 入口；浏览器编译采用只有 id、宽高、HDR 标记的 `GpuImage`。共享合成器仍负责全部几何、源绑定、精确时间区间、source contain、项目坐标到预览像素的映射、定点采样、组裁切和组透明度。未新增 JS 时间线/几何计算，也未要求所有浏览器帧先复制进 WASM。

`mo-video-gpu-plan` 不依赖 wgpu、解码器、浏览器或平台 IO。它调用原合成器生成分配与 paint 指令。原生 wgpu 与浏览器现在使用同一个原创 `compose.wgsl`，同一个 Rust `PaintParameters` 生成 112 字节 ABI：7 个 vec4、28 个 u32 字段。Q32.32 拆分、颜色和透明度值全部由 Rust 生成；JS 按小端写入指令，不重新推导算法。

`SourceGeometry::from_display_metadata` 解释明确的像素宽高比与容器行向量矩阵 `[a,b,u,c,d,v,x,y,w]`。原生固定点矩阵先在 Rust 解码，再进入这个相同入口。拒绝非有限、奇异和未支持的投影矩阵，不丢弃旋转/镜像。输入像素必须已裁到 visible rect，且没有预先应用容器显示变换。

`PreparedGpuFrame` 持有不可变计划和完整请求集合。请求键是完整 `MediaBinding` 与精确 `TimeSample` 的域隔离内容摘要；`before/after` 不合并。重复引用同一资源采样只分配一个图像 ID。此键仅表示内容身份，不能授权读取文件。宿主异步取帧后提交完整的 `VisualReply[]`；未知、缺失、重复请求和改变的绑定均拒绝。帧/空隙区间、素材可用范围、媒体种类和尺寸仍由原合成器校验。失败不消耗候选，可以干净重试。

`mo-video-visual-wasm` 是独立正式开发接口：

- `ProjectVisual.open(documentJson, expectedDigest)` 编译并固定不可变工程。
- `frame(index: bigint, width, height)` 创建拥有自身计划的 `VisualCandidate`。
- `candidate.requests()` 给出精确素材请求；`identity()` 固定工程摘要与帧序号。
- `candidate.compile(repliesJson)` 返回完整 GPU 程序。项目对象释放后，已拥有的候选仍有效。
- `composition_shader()` 返回与本二进制一起交付的共享 shader；不得混配其他版本。

无设备、网络、授权、像素缓存和文件存储进入 WASM 编译器。文档最多 32 MiB，回复最多 1 MiB，默认最多 64 个不同采样请求、8192 条指令。总 GPU 程序预算包含所有保留的合成面、每个规范化输入面一次、uniform 和 solid placeholder，默认 512 MiB；并非仅计算递归深度峰值，也不表示浏览器总内存上限。

## 浏览器执行与所有权

`packages/video-browser-gpu` 是薄平台执行器。`BrowserGpuExecutor.create(adapter, shader)` 消耗调用方选择的新 adapter，创建专属 device；不与外部代码交错 error scopes。每次操作只允许一个活动调用，WASM 计划在首次 await 前完成快照与设备边界验证。

- `upload` 明确接收已经规范化的线性 BT.2020/D65、203 nits 相对亮度、预乘 RGBA32F。它是显式 CPU 输入入口，拒绝共享数组、非有限值、越界 alpha、透明像素隐藏颜色；没有暗中做颜色转换。
- `render` 接收当前执行器拥有的图像能力，验证实际尺寸/HDR/设备归属，随后创建全部临时资源并完成 GPU error scope 检查，才提交队列。
- 活动操作持有输入的引用。调用方在操作进行时 close 输入，不会提前销毁正在使用的 buffer。完成后释放所有临时 buffer，只交付结果图像。
- `readback` 是明确的诊断/CPU 回读。返回数据归调用方管理；已有 GPU 图像还可直接作为下一次合成输入。
- GPU buffer 账本独立于解码器、driver、JS/WASM 堆和调用方持有的回读数据。分配失败、输入错误和中途预算失败均释放已创建候选。
- 提交后的取消退役专属 device，阻止后续复用完成状态不确定的队列；宿主负责新建执行器。device loss/close 是幂等清理，物理资源回收时机仍由浏览器/driver 决定。尚未声明设备丢失后自动恢复或抢占 GPU。

当前没有浏览器 GPU 颜色规范化、Canvas 输出或 GPU 视频编码适配器。不能把 `upload + readback` 测试称为零拷贝播放。下一步应在此执行器内连接媒体图像导入和明确的输出颜色档案，保持源图像能力与预算共用。

## 浏览器媒体接入依据

核对 [WebCodecs 规范](https://www.w3.org/TR/webcodecs/) 与 [WebGPU 规范](https://gpuweb.github.io/gpuweb/)：VideoFrame 渲染会应用裁切、方向和颜色转换；external texture 还带有对应坐标/颜色元数据，采样颜色空间由导入参数确定。复制 VideoFrame 时，传入 `rotation: 0` 并不自动清除原方向，构造过程组合原方向。接入时必须实测移除方向的像素视图，或明确调整核心几何，避免双重变换；不能仅依赖一个默认值。external texture 不保证无拷贝，视频过滤与颜色转换的顺序也存在实现差异。

这部分是已核实的接入约束，不是本批已经实现的 WebCodecs→GPU 颜色通路。当前真实浏览器验证只使用明确的规范化线性输入。

## 实际验证

| 范围 | 证据与结果 | 限制 |
| --- | --- | --- |
| 纯编译器 | 5 项测试：精确前后边界/重复引用/空隙，9 类失配重试、每个取消点、精确预算、未接文本拒绝及 SAR/矩阵 | 不是媒体解码 |
| CPU 渲染回归 | 23 项通过 | 显式真实素材 ignored 用例不计入 |
| 原生 GPU | Apple M4 Max/Metal，29 用例；最大线性差 `9.5367431640625e-7`，RGBA8 最大差 1 | 原硬件门禁；不是浏览器显示 |
| 公共 WASM | 6 个真实工程帧，完整 GPU 指令与 native 完全一致；30 次回复故障和干净重试；项目/候选所有权验证 | 二进制编译验证，无解码 |
| Chrome WebGPU | Chrome 154.0.8037.98、macOS arm64，adapter vendor `apple`、architecture `metal-3`、非 fallback；6 帧最大线性差 `5.960464477539063e-8`、最高 RMS `7.283559444060873e-9` | 正确性语料不是实时性能基准 |
| 浏览器失败与回收 | 2 类非法像素、5 类非法程序、关闭后的能力、分配与部分帧预算拒绝、事前取消、成功重试均通过；每轮自有 GPU buffer 账本归零 | 未验收运行中抢占/设备恢复 |
| 编译与规范 | 新增 Rust crate 与相关渲染/GPU crate all-targets strict Clippy、WASM 编译、TS strict 编译通过 | 未重跑所有历史工作区门禁 |

门禁记录：`.codex-work/video-gpu-shared-program-verified/report.json`、`video-visual-sdk-final-report.json`、`video-browser-gpu-final-check/report.json`。原生 GPU 组件记录重新核对全部 104 项依赖与 181 个文件；本次只更新既有 workspace 累积的 js-sys feature 激活记录，不改变第三方代码。

## 不可变开发交付

`tools/video-visual-sdk/build.py` 离线生成开发组件。根入口提供实际 WASM runtime，`./gpu` 提供浏览器执行器；19 个交付文件含完整 SHA-256 清单、源码输入、共享 shader 的源码摘要、声明与第三方通知。WASM 约 1.09 MB，本地完整候选约 1.28 MB，均不是整个编辑器或应用体积。

最终本地交付 `.codex-work/video-visual-sdk-final` 来自干净提交 `977c65cdd5f2d8133fdeed34a9f41ccec1fc9c27`，清单 pin `d572b37cfbfec28234135d88d536a99c8eebd54ce566b221a58bf3a06106bd94`，19 个文件、总计 1,278,154 bytes。WASM 1,094,519 bytes，SHA-256 `8d347aa46cdd52d30f7712465c3f4c2186ee0d7f33697b08f2f13937c73891d4`。最终交付已重新完成公共 ABI 和实际 Chrome GPU 检查，Product 固定消费该交付。

新增仅开发声明依赖 `@webgpu/types@0.1.74`，BSD-3-Clause，锁定包完整性；只交付声明及原许可证，没有复制第三方视频引擎实现。安装者仍须固定清单与文件摘要。本交付 `releaseCleared: false`，不公开发布，也不更改 v0.1.0 稳定基线。

```sh
cargo test --offline --locked -p mo-video-render -p mo-video-gpu-plan --lib --tests
cargo build --offline --locked -p mo-video-gpu-plan --example visual_conformance
python3 tools/video-visual-sdk/build.py --output .codex-work/new-visual-sdk --bindgen .codex-work/toolchain/bin/wasm-bindgen
node tools/verification/video-visual-sdk-check.mjs target/debug/examples/visual_conformance .codex-work/new-visual-sdk .codex-work/new-visual-sdk-report.json .codex-work/new-gpu-vectors.json
node tools/verification/video-browser-gpu-check.mjs .codex-work/new-gpu-vectors.json .codex-work/new-browser-gpu-check .codex-work/new-visual-sdk
```

Product 后续必须消费此公共不可变交付，将精确源请求绑定至已授权资源与固定工程版本，再连接原有音频主时钟和播放器。文字/字幕、HDR 档案、长素材预算、正式浏览器导出、正常页面/Agent 和真实 provider 计费仍属于完整目标，不能用当前合成专项代替验收。
