# Rust 路径编译与实际像素输出

后续[渐变画笔](gradient-raster.md)已升级为类型化 brush 和 ABI 4；本节原测量保留为历史证据。
2026-09-24 · 已实现 Q32 路径到固定 CPU profile 的精确编译、Native 隔离进程、WASM 绑定及 CLI 原子文件发布。真实段落轮廓已通过此入口生成像素。初始阶段是底层填充路径操作；后续已增加[纯色描边](stroke-raster.md)和[作者页面编译](page-render.md)，完整 Draw IR 与目标应用视觉验收仍未完成。已接入[仿射场景](scene-raster.md)的共享变换和实例。

## 职责和输入

`mo-raster` 持有类型、坐标计算、语法/预算、取消检查与结果核对，不访问文件、网络、隐式字体或平台绘图库。`mo-skia-sys` 只封装固定 [Skia 组件](skia-component.md)的私有 C ABI，unsafe 集中在 `ffi.rs`；`mo-raster-worker` 是可回收原生进程。WASM Rust 通过显式 `RasterComponent` 实例调用同一 C++ WASM，不把 Skia 链入主 Rust WASM。

输入 [PathRasterRequest](../../contracts/generated/path-raster-request.schema.json) 包含：

- viewport：像素宽高、Q32 EMU 原点、正整数分子/分母的 pixels-per-EMU、允许的 Q32 **像素**坐标误差及未预乘 sRGB RGBA8 背景。
- paths：共享的局部 Q32 EMU 路径，move/line/quadratic/cubic/close 和 nonzero/evenodd 填充规则。
- draws：路径索引、Q32 EMU 位移、未预乘 RGBA8，以及可选 stroke；省略 stroke 表示填充，显式 stroke 指定世界线宽、端点和连接。按数组次序 source-over 合成。

所有 Q32 值使用规范 i128 十进制字符串。输入文件最多 32 MiB，拒绝重复键、未知属性和不规范数值。宽高各 1–8192，总计最多 16777216 像素；最多 4096 路径、65536 draw、262144 源指令、1048576 条被 draw 引用的指令。路径预算先于组件调用检查。空路径可以复用但不产生 ink；开放 contour 按填充规则隐式闭合，close 后需要新的 move。

这些是当前操作 profile 的明确上限，不是整个产品接受大文稿的最终限制。生产调度还需分块、裁剪、缓存及完整资源管理，不能将超限内容静默丢弃。

## 坐标与精度

每条路径以第一个 move 为局部锚点，先在整数域减去锚点；每次绘制的位移在整数域计算 `draw.origin + anchor - viewport.origin`，再缩放。相同路径仍只编译一份。共同原点达到 `2^110`，或运算中抵消 i128 极值，均不会因为提前浮点化丢失位置。不能表达的最终差值或设备范围明确失败。

正有理数 scale 先约分，随后直接从整数分子/分母计算 IEEE float32 的 nearest-ties-even；没有 `i128 → f64 → f32` 的双重舍入。语料包含恰好略高于单精度中点的反例，普通双精度中转确实给出不同的最后一位。

编译器逐个检查控制点实际 `float32(local + translation)` 与精确变换值的误差。比较在整数向外区间中进行，报告上界最多比真实最大误差多 2 个原始 Q32 像素单位。请求容差范围为 256–16777216，即约 `2^-24` 至 `1/256` 像素；超过用户容差返回 `PRECISION_EXCEEDED`，不降级为较差结果。局部值、位移和变换后控制点都受 ±32768 设备像素边界限制。

此误差只描述**量化后的控制坐标**，不表示像素覆盖率、曲线到真实字体的全部误差或 Office/WPS 视觉误差。仿射矩阵由[场景编译器](scene-raster.md)求值，纯色描边的独立宽度/尖角量化预算见[描边说明](stroke-raster.md)。渐变、图片、clip、group/effect 继续在同一数值与资源合同下扩展，不能把全部全局坐标直接交给 float32。

## 输出与失效

成功元数据遵循 [PathRasterResponse](../../contracts/generated/path-raster-response.schema.json)：固定 profile、宽高、字节数、像素 SHA-256、设备批次 SHA-256 和工作量/坐标误差上界。像素走单独的二进制通道，为 sRGB、RGBA8、premultiplied alpha，行距固定 `width × 4`；不使用 JSON 数字数组或 base64。

Rust 核对组件状态、精确像素长度及所有 RGB ≤ alpha，再分块计算摘要。WASM 在复制前检查 `Uint8Array` 类型和长度，并用有界、可失败的 Rust 分配接收像素；不会先按未核对的宿主长度分配 Vec。故障、取消或不完整输出不返回图片；组件状态 2/4、协议损坏、宿主异常及后置取消都会使实例失效。已失效 Native 库实例不能通过创建新 Rust 包装器恢复；WASM 模块同样不可重新认领。

设备批次摘要标识字节，不单独承诺跨编译环境的永久像素缓存兼容。未来缓存键还需固定 profile、后端构建及其验证范围；请求的坐标误差/工作量元数据不能只按相同设备批次复用。

`mo-raster-worker` 从 stdin 接收 `[jsonBytes:u32LE][JSON]`，stdout 返回 `[metadataBytes:u32LE][pixelBytes:u32LE][JSON][RGBA]`；不接受路径或其他参数。正常输入错误后可以继续下一份完整请求；组件失效后退出。当前 CLI 每次启动一个进程，清空环境、限制响应长度、要求无尾随字节与成功退出。30 秒期限涵盖写入、读取和**进程退出**；这一执行器也供文字工作进程使用。关闭 stdout 后继续挂起不能绕过期限。失败时杀死、回收并等待传输线程结束。

CLI 对成功像素采用和 PPTX 共用的同目录暂存/读回验证/无覆盖发布：文件权限 0600，校验实际文件的长度与摘要，已有目标不会被覆盖。它仍是显式路径的开发宿主，不是 Musterwork Artifact/CAS 提交或生产沙箱。当前 worker 不创建子进程；生产进程组回收、RSS 限制、池化、异步取消与浏览器 Worker 仍待实现。

## 初始填充阶段的验证与大小

[本轮证据](../reviews/evidence/2026-09-24-path-raster-verification.json)绑定源码、构建、合同、语料和产物。新增 116 批真实 Native/WASM 请求：89 批成功，27 批明确拒绝；包括 29 个实际段落、填充规则/透明叠加、32 组有理数曲线、共享大原点、精度拒绝和资源边界。元数据与像素逐字节相同；同一已编译帧还通过独立 C++ 可执行程序核对，15 个基础形状和已有组件输出相同。

独立 Python Fraction 参考通过相邻 float32 候选的精确距离选择舍入结果，核对 535264 个局部坐标、133546 个绘制位移坐标、2108978 个变换后控制坐标及其误差界，算法不复用 Rust 的位构造。9 种 WASM 桥接故障、原生连续请求、CLI 发布/拒绝覆盖、实际耗尽组件内存后的无部分输出、拒绝复用与新实例恢复均有记录。

旧 1883 批主内核在新产物上回归且语义结果不变；主内核合计 1999 批。另行回归独立 Skia 148 批，包括 Native ASan/UBSan、61444 个解析参考像素及实际 Skia abort，不重复加入主内核批数。Rust 215 项测试、严格 Clippy、42 份 Schema 和 TS 检查通过，40 份既有 Schema 不变。7 份 PNG 是本入口实际像素的编码；已查看中文、双向和原创三次曲线样本，未将这种观察记作目标应用视觉验收。

当前未压缩开发产物：CLI 3929616 字节、文字 worker 2375008 字节、绘制 worker 2724816 字节、Rust WASM 3551475 字节；独立 Skia WASM 1803122 字节，另有各自 glue/TS 薄层。原生绘制 worker 已静态包含所用 Skia 代码，不应再把静态库归档当作额外运行下载相加。Native 在 macOS arm64 验证，动态链接 libc++/libSystem；尚无 Windows/Linux/浏览器证据。这些数字不包含完整字体/图片/媒体/宿主，也不是完整内核或 Musterwork 安装包估算；本轮没有性能/峰值内存门禁结论。

## 复现

先完成 [Skia 构建](skia-component.md)及[段落路径语料](paragraph-paths.md)。Native 构建现在额外产生 `libmo_skia_adapter.a`，Rust build 同时核对它和 `libskia.a`，核对组件源码、锁文件和无 sanitizer 配置；默认只接受本机 macOS target。没有隐式下载或链接系统 Skia。

```sh
python3 tools/components/build-skia.py --target native
cargo build -p mo-cli -p mo-text-worker -p mo-raster-worker --release --locked --offline
cargo build -p mo-wasm --target wasm32-unknown-unknown --release --locked --offline
node tools/contracts/build-wasm-node.mjs
pnpm build:raster-component
node tools/verification/path-raster-parity.mjs
python3 tools/verification/path-raster-reference.py
python3 tools/verification/contracts.py --path-raster-report .codex-work/path-raster/parity.json
python3 tools/verification/skia-previews.py --directory .codex-work/path-raster
```

开发使用：`mo-cli render-paths <request.json> <new-output.rgba>`。WASM 使用 `render_paths(requestJson, component)`，先读取 `.metadata`，再通过 `.take_pixels()` 消费返回对象。生成 PNG 的脚本只做像素编码，不参与绘制。

后续继续统一 Draw IR、PPTX/文档模型到页面编译、图片/笔刷/效果和可编辑高级对象，再连接播放、Agent 接口与 Musterwork。当前成果不关闭 E0、E1 或 E3。
