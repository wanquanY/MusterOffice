# 原生媒体 worker 与宿主

2026-10-06 · 已实现并在 macOS arm64 实际验证。属于媒体执行基础，还未成为用户可操作的时间线预览/导出。

## 职责与依赖

- `mo-media`：无原生库链接的资源读取能力、流信息和独立拥有的帧值；不包含路径、URL、存储或工程状态。
- `mo-media-wire`：私有 IPC framing、输入和输出预算校验。它不是公共 SDK 或持久工程格式。
- `mo-media-worker`：唯一加载固定 FFmpeg 的进程；启动后先安装 `mo-native-limits`，再调用安全 Rust adapter。没有命令行媒体路径或继承业务凭证。
- `mo-native-media`：校验宿主指定的 worker 包，启动及持有进程，以宿主 `ReadAt` 提供资源；不依赖或链接 FFmpeg。复用现有 `mo-native-worker::Session`，没有建立另一套任务平台。

宿主拥有文件和网络权限及不可变资源；worker 只持有有界字节能力。环境变量在通用 Session 启动时清空。这里的资源能力约束与进程故障回收不等于 OS 系统调用沙箱：当前没有禁止进程内任意系统调用的完整平台隔离。

## 协议和生命周期

最初的解码协议为 `musteroffice.native-media-worker/1`；当前已随[编码能力](video-encoding-worker.md)升级为 `/2`，旧包不会被新版宿主接受。两版均使用 4 字节 little-endian 长度和最多 64 KiB 的严格 JSON 头，随后按布局传输原始帧平面或资源读回复。没有 base64，也不要求整份素材进内存。聚合平面长度、乘法溢出、帧 generation、调用编号和音视频类别在读取帧 payload 前检查。

单会话 `Open → Select → Next/Seek → Close`，一个会话只有一个流和一个拥有线程。Open 返回流和实际安装的进程额度；Select 消耗容器。Seek 成功后递增 generation；返回帧必须匹配当前 generation。错误使会话永久停止，不能以正常 EOF 或上次成功帧代替失败。

解码过程中 worker 发出带 call/read ID 的 `Read`，宿主核对单调编号、offset、资源长度、总读取预算，以最多 32 KiB 回复。原生 AVIO 有时会直接请求超过其缓冲大小的包，所以 worker 明确采用合法短读；不会把大于 32 KiB 的真实媒体包误拒绝。每次解码响应携带独立拥有的 active planes，后续解码、seek、关闭不会修改已交付帧。

Close 先销毁原生状态并回复 Closed，再等待宿主关闭 stdin 后正常退出。宿主检查无尾随输出且退出成功。错误、取消、超时、panic 或 Drop 都 kill/reap，不遗留后台解码进程。

## 预算、取消和故障

`WorkerOptions` 分开声明 `open_timeout`（包复验、spawn、probe、输入往返）和 `call_timeout`（每次后续操作）。每项操作只有一个总 deadline，多个资源读往返不会重置它。帧最后一字节到达时仍复查取消与 deadline，取消竞争不交付成功帧。

`ReadAt` 和取消回调在宿主拥有线程执行，必须及时返回；包装在边界捕获 unwind，停止并回收 worker。不能抢占一个不返回的宿主回调，因此不把该 deadline 描述成任意同步宿主 I/O 的强制中断。宿主应自行提供有界或可取消的 I/O 能力。

进程累计 CPU、地址增长、文件描述符、core/file 增长额度由[资源限制模块](native-worker-limits.md)在接收素材前安装，宿主复查实际值不高于请求值。安装失败立即退出。地址增长不是 RSS 硬限额；帧/资源预算也不是所有 vendor 私有分配的严格上限。

## 本地包与发行边界

`tools/media-worker/build.py` 校验固定 C 组件及来源后构建 worker，将 executable 与 6 个动态库、10 个 loader alias 记录在 manifest 中。macOS 使用 `@executable_path/../lib`，Linux 使用 `$ORIGIN/../lib`，不能带个人开发目录 rpath。宿主包校验覆盖 manifest pin、平台、长度、SHA-256、真实目录、文件类别、符号链接目标及多余 loader 文件，并在每次 spawn 前复验。

`WorkerBundle::open(root, trusted_manifest_sha256)` 要求调用方传入可信的摘要，不能从待校验包自身读取摘要后将它当成信任。产品安装方负责签名/发行来源及不可变存储权限。复验本身不能防止有权限的并发写入者在校验和加载间改包。当前 builder 只产出开发包；签名发行、组件频道及完整 LGPL 对应源码/notice/库替换材料仍需正式发行流程。

## 实际证据和复现

```sh
python3 tools/verification/current.py --groups media \
  --ffmpeg-dir .codex-work/ffmpeg-encode-final \
  --output .codex-work/new-media-worker-check
```

早期 gate 的四步顺序为：C callbacks、Rust 安全 adapter、构建新包、真实 worker。最后一步明确执行默认 ignored 的 6 项生命周期、3 项包校验及 1 项真实媒体测试；缺环境报错，不把跳过算通过。

本机 `.codex-work/media-worker-gate-final/report.json` 与 `media-worker/report.json`：

- 8 项默认协议/宿主测试：长度与重复字段、聚合平面预算、错 call ID/旧 generation 的 payload 前拒绝、取消与末帧竞争、无输出和 pipe 背压超时后的回收。
- 6 项真实进程测试：PCM 短读逐字节往返、seek/EOF/独立拥有帧、取消、资源/取消回调 panic、跨多次读取的总 deadline、SIGKILL 崩溃、Drop 和 clean Close 的回收。
- 3 项包测试：可信 manifest pin 与库/worker 字节、loader alias/目录/多余文件等变更会被拒绝，spawn 时再检查。
- 77 帧实际 H.264 B-frame/VFR 的像素 SHA-256、PTS、seek 后缀与 BT.709 标签和独立 CLI 一致。
- AAC 144,000 样本连续，最大绝对误差 `2.235e-8`、RMS `3.319e-9`；沿用已知源长度的参考尾 padding 规则。
- 1024×1024 rawvideo AVI 的单包超过 32 KiB，经跨进程短读成功产出 1,572,864 字节帧，与独立 CLI 摘要一致；把帧预算降到 1 MiB 会失败并回收 worker。
- 对实际测试宿主二进制执行 loader 检查，未链接 FFmpeg 或 C bridge。

本次包 manifest SHA-256：`47ff13846cb679db51a8226f417843042dc492ccbe0eb9f8e862e2b0311b9edd`。它绑定这次本地开发包，不是已发布版本或平台性能结论。

后续[连续区间源采样](video-source-presentation.md)已将实际帧接到工程编译计划。后续 CPU/Metal 合成、音频 DSP、标准 PCM、工程保调和[编码导出 worker](video-encoding-worker.md)已有独立实施记录。通用容器索引、浏览器及其它原生平台、正式 SDK/组件发行与产品接入仍待完成。不得从这里的进程测试推断完整媒体/产品验收通过。
