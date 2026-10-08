# 浏览器媒体颜色与 GPU 图片交付

2026-10-08。此增量实现真实解码帧的共享颜色转换与 GPU 图片交付；统一工程预览宿主、完整播放器接线和导出仍在完整目标中。

## 核心负责输入解释

`BrowserFrameInput::compile` 接收实际 VideoFrame 的格式、可见宽高、完整颜色声明，以及显式色度位置和 HLG 显示参数，生成 `RawInputPlan`。输出包含原始平面复制布局、字节预算和 224 字节 GPU 参数。布局解析、颜色标签解释、主色矩阵、色度位置与原生 `mo-video-render` 共用；新增 WGSL 是该原生逐像素计算的 f32 执行后端。TS 不计算颜色系数、EOTF 或色度重建。

`video_input_plan(descriptionJson)` 在独立视觉 WASM 中公开此接口。`color_shaders()` 同时交付输入规范化与 SDR 输出 shader。输出仍为线性 BT.2020/D65、203 nits 相对亮度、预乘 RGBA32F，直接供已有 GPU 合成器使用。输入元数据不完整、RGB/YUV 矩阵冲突、未知格式、缺少必需色度位置、无 HLG 显示解释均明确拒绝；不能根据画面尺寸猜颜色。

浏览器 raw 格式对应 CPU 的 packed RGB、平面 YUV 或 NV12。支持已验证的 I420/I422/I444/NV12、I420A/I444A、I420P10/I422P10/I444P10/I420P12/I444AP10，及 RGBA；BGRA/RGBX/BGRX 和其他已声明 10/12 bit 组合有计算映射，但本批不宣称每个组合均实测。GPU 读取 little-endian 整数码值，按声明位深验证；异常码值只返回失败，不交付候选画面。

## 为什么不用浏览器的默认颜色转换

[WebGPU 规范](https://gpuweb.github.io/gpuweb/) 定义了 external texture 的颜色转换和 alpha 语义，但浏览器实际导入路径仍需像素验证。本机 Chrome 154 的 RGBA VideoFrame 导入出现预乘/低 alpha 精度损失；真实 BT.709 H.264 通过 external texture 转为 sRGB 后，与原生显示光解释存在明显差异。不能通过放宽阈值、额外除 alpha 或改掉原生颜色标准掩盖差异。

当前采用原始平面复制：`VideoFrame.copyTo` 不指定转换格式/色彩空间，按共享内核给出的 layout 直接复制到映射的 GPU storage buffer，再由共享 shader 转换。依据 [WebCodecs 原始平面复制规范](https://www.w3.org/TR/webcodecs/)，这条路径读取 visibleRect，不执行渲染时的显示旋转、翻转和 SAR；这些几何仍由核心合成器处理。实际旋转/翻转/SAR/裁切组合已逐像素验证。没有先把图片变成 Canvas，也没有把整帧搬进 WASM。

这是**一次有界原始平面传输**，不是零拷贝保证。对硬件解码表面，浏览器可能需要 GPU→CPU 再上传；性能必须实测后报告。后续快速导入只能在证明源格式、色度、传递函数和 alpha 与核心解释一致后采用，不允许更改项目画面换取快路径。当前方案优先确保同一工程跨端画面一致。

## 输出和资源所有权

- `importVideo(lease, plan)` 消耗输入 lease，包括参数错误、预取消和 GPU 失败。原媒体预算一直覆盖这份帧，直到传输和计算结束；核心已产出规范化图像后才释放原帧。
- input plan 在第一次 await 前快照，并核验与实际帧的格式、尺寸、颜色身份及平台 copy layout。显式色度解释属于宿主授权的素材解释，后续缓存键必须包含它。
- raw input、输出图像、uniform、4 字节有效性状态及回读状态都纳入同一有界 GPU buffer 账本。GPU 发现超过声明位深的样本时拒绝结果并释放全部候选。
- `readSrgb8` 明确提供 straight-alpha sRGB8 回读；不经 Canvas 预乘往返。
- `toBitmap` 使用 GPU render pass 与 OffscreenCanvas 返回拥有 `close()` 的 ImageBitmap lease，正常呈现没有整帧像素回读。宿主可 transfer 给 UI，并且必须在 UI 消费或丢弃后 ACK/close 原 lease。
- canvas texture 只在当前任务中有效，因此在同一同步调用内编码并提交；私有 canvas 仅在 GPU scopes 与完成检查都通过后才能产生对外图片。失败不能发布半张画面。
- 可见 canvas 缓存一个尺寸；每个未确认 bitmap 单独计额。`retainedSurfaceBytes` 是拥有的逻辑像素面积，`allocatedBytes` 是 GPU buffer 字节；两者都不等于 driver/RSS。close/device loss 释放本执行器账本，不宣称控制浏览器物理回收时机。

HDR 输入保留 PQ 的绝对亮度或明确 HLG 显示光解释，直到最终输出。SDR 输出沿用原生 `SdrTransform` 的 BT.2020→709 和显式 ReinhardLuminanceV1；无 HDR→SDR 策略会失败。显示用编码预乘、文件/诊断用直通 alpha 明确分开。

## 实际证据

环境：macOS arm64、Chrome 154.0.8037.98、实际非 fallback Apple/Metal-3 adapter，headless WebGPU。不是 Firefox/Safari、Windows、物理显示器或实时性能验收。

| 检查 | 结果 |
| --- | --- |
| 原始 RGBA + 8 组方向/翻转 + 可见裁切 + 非方形显示比例 | 最大线性误差 `2.981e-7`；GPU sRGB8 与 CPU 码值完全相同 |
| 44 组 YUV/半平面/alpha/8-12 bit/BT.709/sRGB/PQ/HLG | 原生与 WASM 计划完全相同；GPU 相对缩放误差最大 `4.836e-5`，出现在 PQ。缩放分母为 `max(1,abs(reference))`；不将 HDR 大数误差写成 SDR 阈值 |
| 真实 H.264 B-frame/VFR 视频 | 全部 77 帧 + 3 次回跳，按原始 PTS 对齐已有独立 FFmpeg 验证过的原生样本；最大线性误差 `3.577e-7`，最大 RMS `8.283e-8` |
| GPU 图片呈现 | 实际 ImageBitmap→Canvas 结果核验；预乘颜色误差低于 1 个 8 bit 码值 |
| 失败与结束 | 预取消仍关闭 lease；超位深输入拒绝；GPU/媒体受控账本归零；已有 6 组合成/预算失败回归通过 |

开发证据目录：`.codex-work/video-browser-color-checked-v5`（44 组 raw vectors）、`.codex-work/video-browser-decoded-color-checked-v4`（80 次真实帧）、`.codex-work/video-browser-gpu-color-regression`。证据仅对应具体候选构建，清洁提交生成的不可变交付还需再次验证。

复现工具：`color_conformance`、`raw_color_conformance` 两个原生 oracle，`video-browser-color-check.mjs`、`video-browser-decoded-color-check.mjs`。真实帧检查需要已验证的 native `linear-rgb.f32`，不能拿当前 GPU 自己的输出当参照。测试 fixture 的 left chroma 已由原生 codec 元数据确认；这不代表所有生产浏览器素材已完成自动色度位置探测。

## 后续接入边界

产品必须先锁定保存的 documentDigest/revision，并按完整资源绑定授权读取，再把实际图像和同一采样的元数据提交给核心候选。生产 source interpretation 需要从原始 bitstream 获取色度位置，或持久保存用户明确的解释；WebCodecs VideoColorSpace 自身没有色度位置字段。不能为任意视频写死 left。本批测试明确提供已核实的 fixture 元数据。

后续 [AVC 帧解释](video-browser-avc-metadata.md) 已接通 indexed movie 的逐 sample 色度 metadata 和随机访问参数恢复；旧颜色证据原先使用显式 fixture 解释，不追溯更改其范围。新的颜色 gate 使用共享 metadata。仍需完成颜色来源冲突策略、图像解码/ICC、统一图像 Worker 与已完成的工程音频时钟、最新 seek 的取消与图片 ACK、完整 Web 页面及导出。已发布 v0.1.0 标签未改动。

## 固定开发交付

来自干净源码提交 `c9563efc0b60398ece855f37b45165a2b8307206` 的 `.codex-work/video-visual-color-final`：清单 `ec0806e786807c7a3e64814272f6251eb7b19d129a05770e670a4f2209f1f3f2`，23 个文件、1,345,503 bytes；WASM 1,138,020 bytes，SHA-256 `828bfe1729330267aa10472d07f77aa8c5b3584f515d64622a7efa98d19121bb`。

该最终交付重新通过 `video-browser-color-final-check`、`video-browser-decoded-color-final-check`（80 次真实采样）、`video-browser-gpu-color-final-check` 和公共 WASM/native 6 帧/30 类回复错误检查。44 组 raw case 的输入 oracle 摘要已写入报告；关闭 lease 自身抛错时也不会保留无法交付的 GPU 图像。相关 Rust 检查、严格 Clippy、浏览器执行器严格 TS 检查通过。`releaseCleared: false`；没有发布新公共版本。
