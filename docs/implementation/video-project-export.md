# 有限工程导出会话

状态：计算链路已实现并验证；尚非产品导出。依赖现有不可变工程、CPU 合成、音频 mixer 及[原生编码组件](video-native-encoding.md)。本增量没有修改 FFmpeg wrapper 或固定上游补丁。

## 职责与接口

`mo-video-export` 为纯 Rust crate，不依赖本机编码库、文件路径、业务任务、网络和存储。`ExportPlan` 借用一个不可变 `CompiledDocument`，确定输出时钟、帧数、音频样本数、背景、色彩档案和计算额度；`ExportSession<E: VideoEncoder>` 接收宿主提供的视觉/音频资源接口及编码器。用户手动编辑和 Agent 编辑都仍经过原有事务系统，导出固定的旧快照不随当前 head 改变。

`mo-media::VideoEncoder` 是有限输入候选编码的能力接口，声明配置、提交一帧/一块 PCM、finish。`mo-media-ffmpeg::Encoder` 实现该接口；不因此获得宿主提交权限。构造 session 时检查编码器配置与计划完全一致。正式适配器必须持有同一个宿主取消能力，阻塞 codec/I/O 不能仅靠每步之间的轮询取消。

一次 `step` 只提交一帧或一块音频。两者依绝对整数时钟交错，交叉乘法以 u128 比较，不依赖墙钟。完成全部输入后才 drain/封装，校验完成计数，产生 `ExportCandidate`；它仍是未发布候选，宿主还须检查组件版本、完整资源闭包、内容身份及输出文件，然后原子提交。正常结束、显式 abort、错误和取消都会关闭会话；再次 step 明确报错。

音频 `PendingSource` / `PendingDerived` 返回 `WaitingForAudio`，本步不提交编码输入，进度不变；宿主准备资源后继续，不用静音替代、忙轮询或重开编码器。其他失败关闭会话，部分文件不得发布。视觉资源当前为同步接口，不能把资源缺失解释为作者空白。

## 精确时钟和最后一帧

视频时间基由工程总时长分母与帧周期分母的最小公倍数确定。CFR 采样最后一帧的呈现时长可以短于完整周期，工程终点保持精确。超出编码档案的 timescale/movie clock、奇数画幅、空工程、非法码率/GOP 或额度不足时规划失败，不暗中取整、裁切或变更帧率。

音频为完整样本，末端取上整。`audio_tail` 保留 `ceil(duration * sampleRate) / sampleRate - duration`，使不满一个样本的正残差可审计。例：测试工程为 24,157 / 48,000 秒、30,000 / 1,001 fps，16 帧，视频 clock 为 240,000，末帧 PTS 120,120，duration 665；44.1 kHz 音频额外尾长为 121 / 7,056,000 秒。不是强迫音视频 sample counts 相同。

## I420 输出

`LinearImage::to_i420` 的草案档案为 `musteroffice.video.bt709-bt1886-limited-left-lanczos2-v1-draft`：

1. 将显式 sRGB 背景转为线性 BT.2020，在预乘线性空间合成 alpha。
2. 复用 sRGB 输出的 SDR 色域/tone-map 计算。HDR 缺显式 tone-map 时报错。
3. 将显示线性 BT.709 经理想黑位 BT.1886 逆变换编码；沿用项目既有显示参照语义，不误用 sRGB transfer 或相机 OETF。
4. BT.709 YCbCr limited range：Y 16–235，Cb/Cr 16–240。
5. 色度水平 left-cosited、垂直居中。两倍降采样之前使用按输出格点缩放的 Lanczos2 低通，8 tap，边界延伸；不以隔点取样代替抗混叠。

转换只持有一行全宽 Cb/Cr 和八行半宽过滤结果；scratch 为 `80 * width` 字节，最终 I420 为 `1.5 * width * height` 字节，不分配全幅 f64 中间图。逐行检查取消，分配失败或超额不返回部分图片。报告色域裁切、tone-map 像素、色度 overshoot 裁切计数。

`max_video_output_bytes` 同时核算现有线性最终图、I420 和转换 scratch；合成中间图、资源解码缓存、mixer、codec 与进程额度分别由对应层负责。这不是全进程硬内存上限。当前是 CPU 参考输出，没有宣称 GPU 零拷贝或最终性能达标。

## 验证

本地记录（忽略的 `.codex-work/`）：

| 证据 | 结果与范围 |
|---|---|
| `mo-video-render/tests/yuv_output.rs` | 原色/灰度、显式背景、线性 alpha、色度抗混叠/相位、跨行 ring、HDR 拒绝、额度及逐取消边界 |
| `mo-video-export/tests/session.rs` | 4 项：绝对时钟、任意音频块、作者 gap、编辑及逆操作、等待准备后继续、取消/编码错误、配置绑定及预算 |
| `project-export-verified/report.json` | 实际输出 6 个 MP4：FFV1/float PCM 和硬件 H.264/AAC，各包含 48 kHz、44.1 kHz、无音轨；独立 ffprobe/FFmpeg 检查时钟、长度、全局 DTS、色彩元数据和完整解码 |
| `export-conformance-parity.json` | Native/WASM 6 组配置、96 帧编码输入、185,408 个 PCM 值逐位一致；另有 3 组非均匀 alpha/SDR/HDR 的 I420 输出向量；不同音频块大小结果一致 |
| `export-render-regression.json` | 既有 9 组 sRGB/线性合成跨端向量仍通过，共用 SDR 变换重构未改变其结果 |

原生文件测试使用真实工程编译/合成/mixer，素材为已知图形和固定内容摘要的程序 PCM；未覆盖真实视觉素材解码、字幕/效果或完整产品。FFV1 输入/解码逐字节一致；本组简单纯色 H.264 图像也恰好一致，不能推论复杂场景无损或代表最终视觉质量。AAC 两端独立解码最大误差约 2.24e-8，输入误差另与独立 AAC 编码比较，不以 codec 浮点近似混淆工程时钟错误。

Native/WASM 驱动使用测试专用 capture，仅检查真实计算后的编码输入，不产生假文件，也不注册为产品编码器。真实 MP4 是另一个原生集成测试生成的结果。

可重复入口：

```sh
cargo test --locked --offline -p mo-video-export -p mo-video-render
python3 tools/verification/media-encode-check.py \
  --project --hardware \
  --component .codex-work/ffmpeg-encode-final \
  --output .codex-work/project-export-new-run
```

`--hardware` 要求实际 macOS VideoToolbox，失败不跳过。默认 `current.py --groups encode` 加入无损工程导出检查；`--groups wasm` 加入工程导出跨端检查，所需组件/工具依既有 gate 显式提供。

## 后续边界

下一层要接编码 worker 与宿主文件候选、超时/取消/回收，随后接生产资源、派生 PCM 调度和输出校验发布。浏览器 codec/mux 档案、GPU 导出、字体/效果/转场、最终调色/响度/输出质量档案、断点续作和产品面板仍未完成。不能因本记录通过而关闭整个 W2/W4 或用户本地全功能验收。
