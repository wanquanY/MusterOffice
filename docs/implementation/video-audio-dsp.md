# 视频音频块处理实施记录

2026-10-06。状态：可执行的内核基础，尚非完整音频引擎或产品验收。

对应[技术合同](../design/video-editing/technical-spec.md)、[验收矩阵](../design/video-editing/acceptance.md)和[总进度](video-editing-progress.md)。档案名 `musteroffice.audio.f32-sinc64-bh4-phase1024-v1-draft`；不同档案不能共用持久化 PCM 缓存。

## 职责与实际链路

`mo-video-compile::audio_block/audio_sequence_block` 按轨道区间索引和时间映射锚点生成音频处理区间。区间起点与结束都是精确有理时间转换为采样格点的向上取整；处理区间为半开区间。不逐样本构造点查询计划。`AudioSamplePlan` 继续作为语义检查和测试参照。

`mo-video-audio` 独立实现 PCM 归一化、命名声道与显式映射矩阵、精确相位时钟、带限重采样、增益/淡入淡出、层级混音。正常依赖不含原生解码器、音频设备、网络、文件路径或业务状态。Native 和 WASM 编译同一份代码；不另写 JavaScript 音频语义。

每一层嵌套序列先在工程采样率上生成自己的 PCM，再由父片段执行速率处理、效果和混音。禁止将不同层的保调/变调政策折叠成净速率。两个 1× 倒放能够恢复原始样本顺序；父组增益在子组混音之后执行。

`AudioResources` 是宿主注入的资源能力：按不可变 resource digest 和 stream 描述、读取精确 PCM 窗口，并在声道布局不同时提供明确矩阵。内核验证返回的采样率、布局、窗口边界及描述符，未知或不完整窗口报错。宿主要完成字节身份验证、decoder 生命周期、缓存和权限；测试中的内存 PCM host 不作为生产适配器。

## 时间与声音约定

- 工程输出样本数遵循既有绝对时钟：只产生起始时刻在工程结束前的样本；不以视频帧数乘近似音频块长度。
- `AudioAsset.sample_zero` 表示素材样本索引零对应的源时间；`available` 是已处理 priming/edit list 的**完整连续** PCM 区间，不是缓存覆盖范围。区间必须与工程资源元数据精确吻合。内部缺失不会补静音；已知真实端点之外采用零延拓。
- 作者时间映射得到 `After` 时，源采样位置为 `(source_time - sample_zero) × source_rate`。得到 `Before` 时，减去一个源采样格，定义离散 sample-cell 倒放：边界 N 的前侧对应 N−1。该规则也作用于分数位置，不使用 epsilon。滤波发生在这个位置上。
- `PhaseClock` 保留精确整数索引和有理余数，逐步相加不积累浮点速率误差。滤波查表相位独立向下量化到 Q0.32，每个样本的分数误差小于 `2^-32` 格；不能据此宣称模拟连续信号完全无误差。
- PCM 使用交错 f32，混音累加使用 f64，每层完成后转换为 f32。有限的超出 `[-1,1]` 的值保留，不悄悄限幅或自动归一化；非有限值和溢出拒绝。最终输出到设备或有损编码前的峰值管理尚未实现。
- u8/s16/s32/s64/flt/dbl 的 packed/planar 输入经过相同归一化，字节序由 decoder adapter 显式给出。声道位置来自明确布局，不按通道数猜测。mono→stereo 为两路 unity；stereo→mono average 为各 0.5，均需明确选择；没有默认丢弃 LFE/环绕声的行为。
- 增益为 `10^(milliDecibels/20000)`。淡入淡出取两条作者包络与 1 的最小值，范围外为 0；保留剪切/裁切前的相位，支持负包络起点、零长度坡道和坡道重叠。每个块按作者绝对相位初始化，不重新淡入。

## 重采样档案

原创标量 Rust FIR，不复制第三方 DSP 源码。数学运算复用仓库已固定的 `libm 0.2.16`（`force-soft-floats`），未新增注册表包或替换既有版本。

- 对称零相位 windowed sinc，Blackman–Harris 四项窗，系数 `0.35875/0.48829/0.14128/0.01168`。
- 基础半宽 64，降采样时半宽扩为 `ceil(64 × |source_step|)`；截止比例为 `0.94/max(1, |source_step|)`，其中速率的滤波系数计算采用 Q0.32 量化绝对值。实际 sample index 推进仍完全使用有理速率。
- 1,024 个相位加端点，各相位权重按 DC 和为 1 归一化；相位之间线性插值。端点外的零样本不参与重新归一化，避免修改边缘包络。
- 精确 ±1 速率且整数相位走逐样本复制，保持输入 PCM bits。分数相位仍经过 FIR。
- 每次处理最多 65,536 输出帧；源步长绝对值不超过 64，实际还受半宽、内存和运算预算约束。不能表达或超预算的查询明确失败。
- 窗口包含前后滤波支持区间；这是离线/预先计算模型，不适合在系统音频回调里现建滤波器或同步读取资源。设备 host 应提前准备并通过环形缓冲喂入，具体设备实现尚未完成。

滤波器缓存有总字节上限和 LRU 淘汰；嵌套调用仍持有的滤波器不能通过淘汰缓存项绕过计量。默认滤波器总额度 32 MiB，活跃核心 PCM/f64 累加器 256 MiB；还限制源/输出块、累计乘加、自动化步骤、计划访问、voice 数、嵌套深度和精确算术位数。统计中的 `peak_pcm_bytes` 排除 plan、宿主/decoder/缓存分配以及滤波器（后者单独记录），不是进程 RSS。

取消检查覆盖构建、资源交接、混音和滤波；错误或取消不会返回部分 PCM 块。已建好的无副作用滤波器可以保留。宿主回调必须及时响应，内核无法抢占阻塞的外部回调。

## 本次实际证据

### 默认测试与跨端

- 音频基础 17 项测试：24 种格式/布局/字节序组合、非有限/截断 PCM、矩阵、精确大/负时间、逐检查点取消、预算、任意分块、分析信号的 DC/通带/阻带、声道映射、重叠混音、包络和嵌套倒放。
- 编译 18 项测试：新增分块查询逐采样对照点查询；原区间索引新增 2,048 组区间查询与独立扫描对照；10 万区间的稀疏窗口查询仍少于 64 个节点访问。
- Native/WASM 实际运行：macOS arm64 与 Node v23.5.0 的 WASM；8 次查询、5 次成功、3 次明确拒绝；所有 PCM f32 bits、区间计划和统计一致。分块结果另外与大块对应切片逐 bit 对照。
- 跨端 fixture SHA-256：`4b1e2a7461c93b7948eba6f1e9176732fe85ba1c0fd7cc4cab89042cc71c568a`；结果 SHA-256：`0dae866e622d4b3264df09d62d0c3d48ec705fb5c4f6fdfff824eb2965183725`。

### 实际 AAC 和独立参考

固定 worker 重新解码既有 3 秒 AAC 素材，逐帧验证真实 PTS 与 PCM 格点连续，得到 144,000 个 48k mono 样本。原生分块器交替用 128/997/4096/257 帧输出。五个完整输出：正放 144,000、倒放 144,000、增益淡入淡出 144,000、44.1k 重采样 132,300、变调半速 288,000。正放 PCM 与本次归一化 decoder 输出完全一致。

独立参考是 CLI FFmpeg，并重新执行解码和滤波；只对旧版 CLI 的已知 AAC 尾 padding 显式裁到原始 144,000，不裁剪本实现输出来凑数。

| 项目 | 最大绝对差 | RMS | 比较口径 |
| --- | ---: | ---: | --- |
| 解码/正放 | 2.2352e-8 | 3.3189e-9 | 全 144,000 样本 |
| 倒放 | 2.2352e-8 | 3.3189e-9 | 独立参考逆序，全样本 |
| −6 dB 与淡入淡出 | 1.2827e-8 | 1.5883e-9 | 独立解码 + 独立数学包络，全样本 |
| 48k→44.1k | 1.4202e-4 | 1.1613e-6 | 全样本，包括不同滤波边界策略 |
| 48k→44.1k 内部 | 4.0978e-7 | 1.2992e-7 | 两端各排除 256 样本；完整误差保留在上一行 |
| 变调半速 | 4.2872e-3 | 8.4104e-6 | 全样本，包括不同滤波边界策略 |
| 变调半速内部 | 2.0713e-6 | 2.3157e-7 | 两端各排除 256 样本；完整误差保留在上一行 |

窗函数与边界处理不同，SRC 不要求与 libswresample bit 相同；分开记录完整结果与内部通带误差。这个单一素材不代表完整音乐/语音质量矩阵，也不代表延迟、CPU、内存 RSS 或听感已经达标。

### 复现

```sh
cargo test -p mo-video-audio -p mo-video-compile --locked --offline
python3 tools/verification/video-audio-check.py \
  --native-evidence <native-media-evidence> \
  --media-evidence <worker-media-evidence> \
  --worker-bundle <verified-worker-bundle> --output <fresh-directory>
node tools/verification/video-audio-parity.mjs \
  target/debug/mo-video-conformance <this-build-wasm.js> <fresh-report.json>
```

`current.py` 的 `media` 和 `wasm` 分组已经分别包含实际声音处理与跨端检查；默认 ignored 的真实媒体测试不算通过，必须由显式门禁执行。

本地未跟踪证据：`.codex-work/video-audio-gate-final/report.json`、`video-audio-parity.json`、`video-audio-workspace-clippy.log`。这些不作为源码包内附件。

## 仍需实现

非原速 `PreservePitch` 当前明确返回 Unsupported；没有以变调重采样冒充保调。后续须接入专门伸缩 DSP，处理输入/输出 latency、预热/尾部、锚点速率切换、seek 随机访问和 Native/WASM 一致性，保留每层政策。

后续已实现[标准 PCM 分块与资源适配](video-canonical-pcm.md)，真实 decoder→本地文件块→Mixer 已有独立测试；生产 Host 窗口准备/存储、通用 edit list/priming/padding 解析、音频设备/AudioWorklet、A/V 主时钟、播放定位、响度/真峰值及限制器、降噪/淡化质量、音频编码/封装、设备变化和用户可操作的工作台仍未完成。CPU/GPU/浏览器实时性能须在这些链路接通后单独测量。本记录不改变完整目标和验收条件。

独立保调顺序流与 Native/WASM DSP 已实现，见[保调记录](video-pitch-stretch.md)。本混音器仍需通过连续派生资源接入；目前非原速分支继续拒绝，不在每个查询块重置 DSP。
