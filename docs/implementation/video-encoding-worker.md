# 编码 worker 与候选输出能力

状态：已实现原生编码进程隔离和实际工程导出验证。计算内核不持有输出路径；`mo-native-media::NativeEncoder` 仍为未发布候选的执行能力，不是产品任务/文件存储系统。

## 结构

复用既有 `mo-media-worker` 固定组件包、`mo-native-worker::Session` 传输/回收和 `mo-native-limits`，不新增另一套进程或任务平台。每个进程只能进入解码或编码模式；两者分别持有自己的原生状态，不能在同一实例混用。

编码进程通过安全 Rust adapter 加载固定 FFmpeg，宿主 `NativeEncoder` 不链接 FFmpeg。它实现 `mo-media::VideoEncoder`，因此[工程导出会话](video-project-export.md)可直接用该受控能力替代进程内 encoder，合成和混音语义不变。

协议升级为私有 `musteroffice.native-media-worker/2`，旧 `/1` 包会在绑定时拒绝，必须构建/安装与宿主一致的新包。包 manifest 格式本身仍为 `/1`；协议版本与包结构版本是不同维度。早期解码证据保留原包身份，不改写为新包验证。

## 输入与写回

```text
OpenEncoder(config, codecLimits, processLimits)
  → EncoderOpened(config, appliedLimits)
EncodeVideo(pts, duration) + packed I420 → VideoEncoded
EncodeAudio(firstSample, frames) + float32 LE PCM → AudioEncoded
FinishEncoder → Encoded(stats)
Close → Closed → stdin EOF → checked process exit
```

Open、编码及 Finish 期间，worker 可以发出 `Write(writeId, offset, byteLength)` 和最多 32 KiB payload。宿主校验单调 ID、输出范围、累计写入额度及每次调用次数，调用显式 `WriteAt`，返回实际写入字节数。合法短写继续，0、超长回报、I/O 错误、panic 或取消终止整个会话。关闭阶段不允许再写文件。

I420 根据已绑定的画幅确定平面大小，不信任请求指定任意长度。宿主将有 stride 的输入打包为 active rows，不传输 padding；每个传输块最多 64 KiB，不再复制整帧作为 IPC 消息。worker 在读取 payload 前检查画幅和聚合额度，只分配受限的 packed frame。PCM 是明确的 little-endian f32，最多 65,536 帧、1 或 2 声道，拒绝非有限值和错位数据。worker PCM 解码暂有字节缓冲和 float 缓冲两份，并非零拷贝。

核心 frame 额度、协议 staging 额度、codec 私有内存、进程地址增长及宿主数据分别核算；不把单一 frame limit 当成全进程峰值。

## 完成、错误与取消

宿主记录真实提交的帧/样本数、确认写入字节和最高输出位置。Finish 结果必须与配置及这些观测值一致；然后 Close、关闭 stdin、检查没有多余输出并等待进程正常退出，才返回 `EncodeStats`。完成后的文件仍须由产品检查并原子发布，字节长度吻合不等于文件内容质量已经验收。

每次操作只有一个 deadline，覆盖数据生产、pipe 传输、所有 Write 往返、codec drain 与结束回收。往返不会续期。等待 I/O 或 codec 时依同一个取消能力 kill/reap；最后响应与取消竞争不能返回成功。状态一旦失败不可重用。未完成对象 Drop 直接回收进程，不自动 finalize。

`WriteAt` 与取消函数在宿主调用线程执行，必须及时返回；回调 panic 捕获后回收 worker。不能抢占一个永不返回的同步宿主回调，宿主仍须提供有界/可取消的文件 I/O。地址/CPU/文件描述符等额度在 codec 初始化前安装并由宿主复查；资源隔离不等于完整 OS 系统调用沙箱。

## 验证入口和范围

```sh
python3 tools/verification/current.py --groups media,encode \
  --ffmpeg-dir .codex-work/ffmpeg-encode-final \
  --output .codex-work/encoding-worker-new-gate

python3 tools/verification/media-encode-check.py --project --hardware \
  --component .codex-work/ffmpeg-encode-final \
  --worker-bundle .codex-work/encoding-worker-new-gate/media-worker-bundle \
  --output .codex-work/encoding-worker-new-hardware
```

本机最终证据：`.codex-work/encoding-worker-gate-final/report.json` 的 11 步 media/encode 回归通过；`.codex-work/encoding-worker-hardware-final/report.json` 的 6 个真实工程导出通过；全 workspace Clippy 通过。二者使用同一新包，manifest SHA-256 为 `b5179604499144da82abe46751818e6068333de1215321b3f4bf2dc6de6a278b`。该 debug 开发包记录的 executable 与动态库共 9,470,600 字节，不代表最终 release 体积或完整安装体积。

独立验证器要求 explicit bundle，核对组件身份，调用方提供的测试包是本地构建的固定输入。产品中不能从不可信包计算一个摘要后把它当作可信 pin；安装来源仍须由宿主掌握。

- 默认协议/宿主测试覆盖分配预检、stride padding、错响应/超长写入、伪造范围与序列、回调错误和异常、输入背压/输出停顿后的 deadline、末响应取消和子进程回收。
- 实际工程采用图形、alpha、gap、带增益/重采样的程序 PCM，跨进程编码并跨进程解码；无损像素/PCM 和任意分块一致。不能把该测试说成复杂素材工程已验收。
- 真实 encoder 10 个分支覆盖取消、sink/取消 panic、I/O 错误、0/超长写入回报、完成时取消、错末帧时长、输入未齐就 finish、未完成 Drop；每项核对 OS 子进程已回收。
- 独立 ffprobe/FFmpeg 另检查 6 个实际文件：无损与硬件 H.264/AAC，各含 48 kHz、44.1 kHz 和无音轨。强制硬件模式失败不跳过。
- 新协议继续回归原有解码、资源采样、CPU 合成、音频 DSP 与标准 PCM 路径。Windows/Linux 编码和完整浏览器导出不由本机证据覆盖。

产品 Host 的候选持久化/恢复、资源 pin、固定 codec/profile 闭包、文件内容校验、原子发布、输出卡片和用户导出 UI 仍待实施。此处没有发布正式 worker 二进制或改变稳定 v0.1.0。
