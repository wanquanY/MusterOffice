# 原生编码、封装与候选输出

2026-10-06 · 开发实现；尚未接入完整工程导出、worker 编码协议或产品界面。

## 职责与接口

`mo-media` 定义无 native 链接的 `EncodeConfig`、`EncodeLimits`、`EncodeVideoFrame`、`EncodeStats` 与 `WriteAt` 能力；`mo-media-ffmpeg::Encoder` 提供线程归属明确的 Rust 所有权。固定 FFmpeg 组件的 `mo_encode.h` 是独立编码 ABI 1，复用媒体错误记录；不暴露 FFmpeg 对象或字符串选项。

核心只接受已准备好的画面/PCM 和宿主借出的随机写入能力。接口不接受文件名、URL、Agent 上下文或业务身份，不启动系统 ffmpeg 命令。宿主必须创建隔离的候选对象，成功后验证、flush 和原子发布；`finish()` 成功只表示字节封装完成，不能代替 Artifact 提交回执。失败后的部分字节不能成为正式成片。

Rust 包装为 `!Send + !Sync`；C 持有 box 稳定地址的回调，所有操作在拥有线程同步进行。输入平面借用只持续一个调用。回调 panic、错误字节数、I/O 错误和取消分别报告，不允许 unwind 穿过 C；任何失败关闭 native 状态，后续调用返回 `State`。成功 finish 和 Drop 也释放状态，但 Drop 不偷偷写 trailer 或发布结果。panic payload 处理沿用[解码边界](video-media-native.md)。

## 明确的编码档案

| 档案 | 编码与容器 | 用途与限制 |
| --- | --- | --- |
| `H264VideoToolboxAacMp4` | macOS VideoToolbox H.264 High + AAC LC；MP4 | 当前已实际验证的常用视频输出。显式关闭 VideoToolbox 的 software fallback；硬件不支持时失败。编译能力枚举不是设备可用性承诺。 |
| `Ffv1PcmMp4` | FFV1 level 3/slice CRC + float32 PCM；MP4 | 精确像素/PCM 审计档案，供验证与计算链路核对。不是浏览器通用播放格式，也不作为 H.264 失败后的替代输出。 |

两者输入均为偶数宽高、平方像素、非透明的 8-bit I420：BT.709 primaries/transfer/matrix、limited range、left-cosited 色度。当前硬件实际码流为 left；不把 centered 输入错误标为 left。FFV1 审计主要核对原始平面，不能假设每个第三方播放器能恢复其全部色度解释。合成工作空间到该档案的转换、HDR tone map、透明背景处理必须由工程渲染驱动明确执行，本层不隐式做这些转换。

音频是有限的 mono 或有序 FL/FR stereo interleaved f32，拒绝非有限值；采样率、声道、码率须显式提供。编码器不隐式重采样、downmix、压限或响度归一化。`audio_samples=0` 不创建音频轨道。输入可以任意正分块，每次至多 65,536 帧；内部按照 codec frame size 聚合，最后的不足整块音频仍保留准确源样本数。

当前为有限 CFR 输出，时基必须精确表达一个完整帧，最后一帧可以短于完整帧。每帧必须从 0 连续输入，满足声明的帧数、总时长和 period；音频样本索引也从 0 连续输入。原素材 VFR 由既有呈现采样层选择，这里不把源平均 FPS 当成采样依据。VFR 成片、HDR/10-bit、多声道和其他硬件档案需要独立接口/质量验证。

## 时间戳与重排

MP4 video track clock 使用指定的 `video_time_scale`，audio track clock 使用真实采样率；movie/edit-list clock 采用二者最小公倍数，必须在有界整数范围内。不能精确表达的配置在打开前拒绝。写头后再核对实际 stream timebase，拒绝封装器悄悄改精度。这样 30000/1001 视频、44.1 kHz 音频和非整帧尾部都有明确的整数时钟。

硬件 B 帧的编码输出顺序与呈现顺序不同。原固定 FFmpeg 的 H.264 VideoToolbox 路径把 `AllowFrameReordering` 的布尔值当成一个固定的一帧延迟，在本机硬件的层级 B 帧上实际出现 `PTS < DTS` 并被 MP4 拒绝。修复位于负责该时间戳的上游组件：

- [固定修改](../../components/ffmpeg/patches/0001-videotoolbox-bounded-reorder-depth.patch)读取实际 extradata 中的 SPS，使用其 `num_reorder_frames` 上界；没有显式限制时，沿用 H.264 parser 的 level/DPB 推导上界，允许保守延迟，不把它宣称为实际硬件延迟。
- 上界必须在 0..16；短于上界的片段在 flush 时以实际输入跨度结束延迟计算。
- 保留 B 帧，不在应用层挪动 PTS 或删除出错包。读取 codec 宏需要 `config_components.h`；配置显式选择 `h264parse`。
- HEVC 未启用该修复档案，不能据此宣称其已完成同样验证。

每个视频输入的 PTS/duration 保存在有限的在途表中，收到 packet 时按 PTS 配对。MP4 decode duration、composition offset 和 edit list 共同描述呈现区间；不能仅凭最后一个解码 packet 的 duration 推断最后画面的可见长度。验证同时核对所有呈现 PTS、声明的 track extent、解码后帧数及精确音频样本数。

AAC encoder 的 priming 和 tail 由正常 codec/mux 元数据表达，不能往用户输入增加伪造的静音作为“对齐修复”。本地 native decoder 的实际输出样本数必须恰好等于声明值。独立旧版 CLI 可能保留 AAC 尾 padding；验证记录它多出的样本，只在比较参考端时使用已知源长度，不裁短 native 结果来获得通过。

## 队列、预算与失败

wrapper 使用自己受控的音频/视频 packet 队列，并以全局 DTS 顺序调用 `av_write_frame`。不委托隐藏的 `av_interleaved_write_frame` 队列无限积累。只有每个活动流已有头部 packet 或已结束时才能确定下一个包；因此调用方应按绝对媒体时钟交错提供输入，不能先塞入整段长视频再提供音频。

边界包括文件最大位置、累计写入字节、单包字节、待写包数/字节、总包数、活动画面字节、像素数、单次计算步数和短写回调次数。随机写入允许覆盖候选头部，最终 byte length 与累计写入量分别统计。零写入、负数、超出本次长度、磁盘满及越界 seek 均失败；取消在行复制、codec 循环和 AVIO 写入中检查。

这些预算不是对 FFmpeg/平台框架全部私有分配或阻塞时长的硬上限。MP4 样本表、codec 内部队列和异步系统硬件实现仍由可终止 worker 的进程限制与总 deadline 承接。当前已有解码 worker，但编码尚未接入该协议，禁止把本 leaf adapter 的测试通过描述成完整产品隔离。

## 构建身份与许可

libraries 阶段从已验证的 FFmpeg archive 提取后，以零 fuzz 应用明确的修改，记录每份修改的路径、字节数和 SHA-256。bridge 阶段、Rust 构建、worker 打包和 native 检查核对相同修改清单与摘要；旧的无修改库不能被新 wrapper 静默接受。C/Rust 编译期 ABI 检查包含编码结构的每个字段。

该上游修改随 FFmpeg 的 LGPL 条件分发，保留其来源和逐文件声明；原创 wrapper/Rust 接口保持仓库原许可。修改记录及相应源码必须进入今后的组件发行包。此轮只有本地构建与源码开发，没有发布新的二进制 SDK。

## 已执行的验证

本机 macOS arm64：

- Rust 默认 4 项编码测试：无损回读、分块/短写、严格输入、回调 panic/I/O/非法返回、取消、预算和失败后复用拒绝。另有 `Encoder` 不能跨线程的 compile-fail。
- 12 组真实导出：两个档案各 1、2、3、8、51、64 帧，共 258 帧；25 fps/48 kHz 和 30000/1001 fps/44.1 kHz、短尾、无音频、音频任意分块与 17 字节短写。
- 独立 ffprobe 验证每个 stream 的时基、起点、精确时长、PTS 集合、DTS 严格顺序、全局交错、颜色与 SAR；51/64 帧硬件案例实际包含呈现顺序重排。
- 独立 CLI 解码视频逐字节等于 native 解码；FFV1 视频与 PCM 逐 bit 等于输入。H.264 本组简单合成画面的 PSNR 为 67.4～79.5 dB，不是复杂实拍场景质量承诺。
- AAC 与独立解码最大绝对差不超过 `5.97e-8`。128 kbit/s 的极短合成音频包含明显 codec 启动瞬态；与同输入/码率的独立 AAC 编码误差对照通过，不把这项测试视为产品音质验收。
- 独立 ASan/UBSan 编译的 wrapper/上游 C，在两个档案的真实输入、9 类预算/非法输入、3 类写入失败及内部取消点上通过。汇编和系统框架不因此被插桩。
- 完整 `media,encode` 9 步回归通过，覆盖原生/Rust 解码、真实 worker、源帧采样、CPU 合成、音频 DSP、标准 PCM 和新导出；Rust workspace strict Clippy 通过。

本地证据：`.codex-work/media-encode-verified/report.json`、`.codex-work/media-encode-sanitized/report.json`、`.codex-work/media-encoding-gate/report.json`。后者 source SHA-256 为 `6f33d307bfcab6f31e301a728f2a729ac7a81658f622db149c45ae8dd327ac8c`，绑定当时的源文件；后续文档变更不倒写旧记录。当前正常库闭包共 7,411,304 bytes，包含五个 FFmpeg 动态库和 wrapper，不包含 Rust worker、GPU、字体及平台框架。

```sh
# 指定由当前源码/修改构建的正常组件
python3 tools/verification/media-encode-check.py \
  --component .codex-work/ffmpeg-encode-final \
  --output .codex-work/new-encode-check --hardware
python3 tools/verification/current.py --groups media,encode \
  --ffmpeg-dir .codex-work/ffmpeg-encode-final \
  --output .codex-work/new-media-encode-gate
# 单独的 sanitizer 组件，libraries/bridge 两阶段都用 --sanitize
python3 tools/verification/media-encode-sanitize.py \
  --component .codex-work/ffmpeg-encode-asan \
  --output .codex-work/new-encode-asan --hardware
```

`encode` 门禁默认实际运行无损档案，因此 Linux CI 不依赖 Apple 硬件，也不把硬件检查标为成功；`--hardware` 明确要求实机 H.264，无法打开即失败。Linux/Windows 硬件、浏览器编码、真实成片播放、工程渲染导出及质量/性能验收仍未完成。

## 下一层

将有限导出时钟与既有工程 FramePlan/AudioMixer 连接，提供带明确颜色/背景/声道策略的输出转换；在 worker 增加有界编码输入与候选写入协议。随后由宿主完成文件校验、持久化、恢复和消息卡片交付。浏览器档案、设备播放、完整效果与字幕、产品 SDK/MCP 和界面仍按总验收矩阵实施，不能以本篇关闭完整视频功能验收。
