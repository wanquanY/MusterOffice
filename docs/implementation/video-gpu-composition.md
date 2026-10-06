# 视频 GPU 合成实施记录

2026-10-06。已在 Apple M4 Max / macOS arm64 / Metal 执行实际计算与读回比对。尚未接通产品显示窗口、浏览器 WebGPU、硬件解码纹理和编码器；没有用计算测试替代实时预览或成片验收。

## 同一份剪辑语义

`mo-video-render` 的 `render_with_backend` 统一负责计划预检、媒体 digest/流/时间区间/尺寸校验、SAR/显示矩阵、作者变换顺序、组表面、透明度和预算。`CompositionBackend` 仅分配线性表面并执行单次 paint；CPU/GPU 不各写一套剪辑规则。

原 `render_frame` 保持 CPU 使用入口。新 `mo-video-gpu::GpuCompositor` 执行同一计划，依赖固定 wgpu 和原创 WGSL。媒体平面→线性 BT.2020 的 normalization 目前仍由 CPU 完成；GPU 不是另一个隐式色彩转换器。色彩合同见[色彩与参考合成](video-color-composition.md)。

`GpuCompositor::new` 接收宿主选择的 Device，不自行选择软件适配器或接管全局错误处理。`encode` 返回未提交的 command buffer、线性输出 buffer 和统计，**不表示 GPU 已完成**。宿主持有 queue submission、完成通知、播放时钟、设备丢失、取消结果隔离和发布输出的责任。编码失败/取消不返回可提交的命令；callback panic 展开也按 LIFO 释放 wgpu error scopes。

原生采用 wgpu 的 Metal/Vulkan/DX12 后端。浏览器按既有方案通过薄 WebGPU Host 消费公共语义和同源 shader；当前没有启用额外的 Rust wgpu WebGPU 绑定，不能把 WGSL 可移植性当成浏览器已经验收。

## 确定的空间采样

实际对照首先发现：直接把 f64 逆矩阵转成 f32 会在最近邻边界选择不同源像素。最终实现统一了覆盖/选择的整数算法，没有放宽误差门限掩盖选错像素。

1. 由共享 executor 求逆矩阵，再以 nearest-ties-even 量化为 signed Q32.32 系数；这是渲染边界合同，不改写作者工程或精确时间。
2. 对输出像素中心使用 `(2*x+1, 2*y+1)`，以有界 signed 64-bit 运算得到 Q30.33 源位置。预检证明所有乘积与求和都不会溢出；极端不可表达或量化后奇异的变换明确拒绝。
3. CPU 用 i64；WGSL 用两个 u32 的加法及 16-bit 部分积模拟相同的二补码运算，不要求可选的 shader i64 功能。
4. 覆盖范围和最近邻索引完全由整数决定。双线性先精确减去像素中心半格并 clamp，再只把局部小数变成 float，避免大图绝对坐标损失小数精度。

在最大 16384 边长处，系数量化造成的每轴位置误差上界小于 `3.82e-6` 像素。它是声明的空间量化，与禁止时间 epsilon 选帧是两种不同合同。CPU 档案升级为 `musteroffice.video.linear2020-d65-203nits-premul-cpu-v2-draft`；已有三组 Native/WASM 语料输出摘要保持不变。

GPU 档案为 `musteroffice.video.linear2020-premul-gpu-f32-v1-draft`。颜色/alpha 插值和 source-over 在 f32 中完成，CPU 参考使用更高精度中间值，所以色值允许明确的数值误差；不承诺任意 GPU 完全相同的 float bits。GPU 原始输入 RGB 绝对值最大 `1e12`，超出返回 capability 错误，结合节点上界保证混合不会浮点溢出，不静默截断。

## 资源与执行

当前使用 RGBA32F storage buffer 和 8×8 compute workgroup。每个 paint 是一个 pass；嵌套组保持独立表面，完成后整体混合。输出仍在 GPU buffer 中，后续 GPU 消费不要求 CPU 读回。测试专门 copy/map 读回，是核验通道。

当前明确存在的开销：

- CPU normalization 和线性帧上传；同一帧多次出现尚未建立 GPU 内容缓存。
- 每 paint 独立参数 buffer，每帧表面/命令重新分配，尚无表面池或持久纹理互操作。
- `allocated_bytes` 是单帧编码过程中全部 buffer 分配之和，包括 command buffer 保留的资源；不是仅按递归栈计算的逻辑表面峰值，也不是驱动内存/RSS。
- `upload_bytes` 包括原始线性源及 uniform；`dispatches` 是实际编码的 compute pass 数。pipeline 和 16-byte 常驻 solid 占位 buffer 不计入逐帧统计。

默认单帧分配预算 512 MiB、上传 256 MiB、4096 dispatch；此外受共享图/像素访问预算和实际 Device buffer/workgroup 限制。不存在隐式降分辨率、取消画面效果或退回 CPU 来伪装满足 GPU 档案的行为。

一次成功返回的是 **encoded candidate**。取消无法抢占已经提交的硬件工作，Device Host 仍需隔离旧 epoch/任务输出；崩溃恢复、设备丢失和在途 buffer 淘汰还需执行层完成。

## 实际证据

独立硬件门禁 `tools/verification/video-gpu-check.py` 已执行，报告为 `.codex-work/video-gpu-gate-final/report.json`。它验证独立媒体检查报告和线性像素摘要后启动真实硬件测试，拒绝 CPU/no-op adapter，不读取预录 GPU 结果。

| 项目 | 结果 |
| --- | --- |
| 设备 | Apple M4 Max，Metal，IntegratedGpu，macOS aarch64；驱动名/版本 API 返回空值，未自行填入版本 |
| 用例 | 29：最近邻/双线性、空帧、透明图片、组透明度、镜像/旋转/多层变换、16384 最大宽度、1920×1080、负 RGB/HDR 线性值，以及四帧真实视频素材 |
| 最大线性分量误差 | `7.152557373046875e-7`（包含 HDR 线性用例）；门限 max `2e-5`、RMS `2e-6` |
| RGBA8 | 最大差 1 code value；四个真实 SDR 帧均为 0，1080p 双线性用例有 1 code value 的舍入差 |
| 组透明度独立预期 | 不透明蓝覆盖红后，组 0.5 混到白色，最终每像素 `[188,188,255,255]` |
| 失败与取消 | 26 个实际取消检查点全部拒绝候选；分配/上传/dispatch 预算拒绝；宿主 callback panic 后设备与下次编码仍可用 |
| 1080p 语料内存 | 一次分配 `66,364,512` bytes，上传 `9,312` bytes，3 dispatch；这是小源图的合成样本，不代表真实 1080p 解码链或最终预览峰值 |
| CPU 媒体回归 | 77 帧 normalization 独立参考最大误差仍 `4.768e-7`、RMS `6.967e-8`；真实采样/合成用例通过 |
| Native/WASM 回归 | 像素 9 查询、编辑 25 步和编译 15 查询均通过，输出摘要与上批一致 |

当前没有提供实时 FPS、端到端拖动延迟、最终应用体积或零拷贝声明。Windows/DX12、Linux/Vulkan、浏览器 WebGPU、device-loss 恢复和长时资源驻留必须分别实测。

## 固定依赖

使用 `wgpu 29.0.3`，由锁文件固定 core/hal/naga 等为 `29.0.4`。上游 [29.0.3 修复说明](https://github.com/gfx-rs/wgpu/releases/tag/v29.0.3)已核对。选择这一兼容版本以保留当前固定的 wasm-bindgen 0.2.126 工具链；30.0.1 要求至少 0.2.127，不能只升级 Cargo 依赖而遗漏既有 WASM 产物与绑定工具。

[组件记录](../../components/video-gpu/component.json)记录 wgpu/bytemuck/pollster 根依赖的 104 项 resolved closure，包含目标平台分支。registry archive SHA 与 Cargo.lock 逐个核对；原许可、版权和缺失于 archive 的固定上游通知另有来源/hash。此计数不是单个平台最终链接 104 个库；pollster 仅用于测试驱动。

原创 WGSL/坐标算法按仓库许可证提供，没有复制编辑器源码。原始依赖许可仍保留。Apple 平台绑定的上游许可证另有 SDK 说明；分发时应保留该说明并核对实际平台材料，不能把开发记录当成所有平台二进制发行已获验收。

## 复现

先执行[真实色彩门禁](video-color-composition.md#复现与证据)，再在有硬件 GPU 的机器执行：

```sh
python3 tools/verification/video-gpu-check.py \
  --linear-evidence .codex-work/new-video-media-check/video-render \
  --output .codex-work/new-video-gpu-check
```

常规 `cargo test --workspace` 保留显式 ignored 标签，不假装通用 CI 拥有 GPU。此脚本会显式执行该用例，硬件缺失直接失败。依赖记录可用 `python3 tools/components/record-video-gpu.py` 离线复核。

本次补充证据：`.codex-work/video-gpu-color-regression/`、`video-gpu-render-parity.json`、`video-gpu-domain-parity.json`、`video-gpu-projection-parity.json`、`video-gpu-workspace-clippy-final.log`。硬件门禁记录测量时的源码摘要；后续本说明和 CI 记录检查入口不冒充硬件代码重测。
