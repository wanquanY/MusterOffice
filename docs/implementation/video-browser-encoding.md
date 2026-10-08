# 浏览器本地视频编码与 MP4 候选

2026-10-08 · 开发实现与真实浏览器专项验证已完成；产品导出交付和整体验收尚未完成。

## 实施选择与依据

浏览器视频使用 WebCodecs VideoEncoder，音频编码与 MP4 封装复用桌面端的固定 FFmpeg 9.0.2 及同一原创 `mo_encode` 核心。浏览器只构建 AAC encoder、H.264 parser、MP4 mux 及其必要依赖，全部在本地 Worker 运行。没有增加服务端渲染，没有引入完整 FFmpeg 命令行或软件视频编码器。

这项实现选择处于用户已授权的组件开发/选型范围，不代表公开二进制发行批准，也没有改变四类目标浏览器及完整剪辑目标。

- 固定 Mediabunny 1.61.3 的容器实现使用推导视频时钟和 57600 的电影时钟，尚无已验证的完整精确整数时钟入口。没有通过伪造 frameRate、修改已安装组件或浮点补偿规避约束。
- 实际 Chrome 154.0.8037.98 的 AudioEncoder 在三组 48/44.1 kHz、单/双声道输入上，首个 AAC 包 timestamp 为 0，但解码声音相对输入均延后 2112 个样本。这个观察只适用于该设备/实现，不能写成通用补偿常数。[WebCodecs AAC 注册规范](https://www.w3.org/TR/webcodecs-aac-codec-registration/)也明确区分编码包与解码 priming，通常的 AAC 码流不携带完整 priming 信息；该接口不能据此提供本档案所需的明确延迟合同。
- 共享 AAC 编码器提供真实 initial padding，原有 MP4 封装处理负起始包、edit list、完整音频样本和最后短帧。浏览器与 Native 使用同一实现，不各建延迟修正系统。WebCodecs 只承担本档案的视频编码。

这是对特定接口与已测档案的工程裁定，不代表所有浏览器 AAC 都有同一问题，或所有其他容器组件无法改进。参考：[WebCodecs](https://www.w3.org/TR/webcodecs/)、[Mediabunny 写文件合同](https://mediabunny.dev/guide/writing-media-files)。

## 责任与调用

`ProjectExport` 继续持有不可变工程摘要、有限范围、精确整数工作与 ACK。`encodeProject` 是薄执行器：向宿主索取共享视觉/音频组件产生的 I420 与 PCM，绑定微秒传输时间，调用 VideoEncoder，再把输出匹配回原始精确 PTS/duration。文件 track/movie 时钟始终使用核心计划值，不使用 codec 微秒反算。

`createEncodeRuntime` 从调用方明确提供的 WebAssembly.Module 创建独立实例。C 编译器公开实际结构偏移；薄 JS 只搬运整数、字节与错误，不重写容器和音频算法。公开声明随交付提供。

宿主提供同步 `writeAt(bigint, borrowedBytes)` 与 cancellation，允许短写。真实门禁使用 OPFS SyncAccessHandle；核心不持有文件路径、网络、权限、数据库、任务或发布 API。finish 返回字节候选统计，宿主仍必须 flush、独立回读验证、持久提交并生成正式文件引用。

新增 `MO_ENCODE_EXTERNAL_AVC_AAC_MP4=4` 必须通过独立 packet 入口打开。它不冒充 Native 硬件 encoder，不进入 Native 的 `mo_encode_profiles` 能力位；原生 Rust 对浏览器编码档案的拒绝保持不变。公共结构 ABI 未改变，新增入口和 opaque 内部状态单独记录。

## 当前明确档案与资源边界

- H.264 High、8-bit I420、BT.709 limited、left chroma、逐行、AVCC 四字节 NAL length；要求实际 decoderConfig 的尺寸和颜色与声明一致。硬件设置为 `prefer-hardware`，不是对浏览器内部实际硬件路径的保证。
- 本次外部 packet 入口支持按显示/解码顺序输出的 I/P access unit。B picture、重排时间戳、配置变化、错误时长和不符的关键帧明确失败，不能猜测 DTS，也不静默切换 realtime、降质或云端。这个限制仍须在后续平台档案中解决或提供另一明确能力，不算四类浏览器全部覆盖。
- C parser 校验尺寸、I/P、逐行结构和关键帧；AVC framing 单独有界检查，in-band SPS/PPS 变化拒绝。parser 的尺寸状态每包清空，坏包不能沿用上一包。它不承诺完整恶意 H.264 验证；SPS 色彩并非该 C parser 的输出，适配器检查 codec 元数据，最终文件还需核心回读校验。
- 默认最多 16 个尚未收到输出的视频工作、256 MiB 在途 I420 有效样本、4 个 VideoEncoder 请求、1 MiB 首包前 PCM；所有原始帧在提交后 close。codec 私有表面/对齐不冒充 I420 有效样本统计。PCM 入参按块核对并有界复制；encoded packet、容器待写队列、文件大小、写入次数另有显式限额。末尾只调用一次 flush，不能反复 flush 绕过反压。
- Worker WASM 初始 16 MiB、增长上限 256 MiB，无 MEMFS、网络、protocol、动态代码执行或线程。codec/驱动私有内存、GPU、输入缓存、浏览器进程内存不包含在此上限中；整个宿主仍需预约并持有可终止 Worker。
- 等待 codec/渲染/PCM 均有停滞期限与取消信号。关闭会取消正在准备的资源；取消后才返回的 VideoFrame 也会被关闭。失败不产生候选完成统计。
- 共享 C 的有限帧数检查改为除法，消除了恶意大 frame-count 与 period 相乘溢出的风险；没有改变正常工程的尾帧规则。

## 构建与来源

固定源码仍为 `components/ffmpeg/lock.json` 的 FFmpeg 9.0.2；浏览器构建不应用仅用于 Native VideoToolbox 的补丁。库构建和 wrapper 链接分离，后者逐一验证原始库、头文件、配置与已记录编译器摘要，写入新目录。编码/解析/容器闭包精确检查为 `AAC_ENCODER / AC3_PARSER / H264_PARSER / MOV_MUXER / MP4_MUXER`，其中 AC3 parser 和 MOV mux 是上游 MP4 的构建依赖；没有 AC3 编码能力或额外公开输出档案。

当前复用已登记的 Emscripten 6.0.10 编译器坐标，不依赖 Skia 库。完整工具链安装归档摘要仍缺失，沿用其明确的开发限制。组件带有库/头/源摘要、配置、命令、relink 对象/静态库、FFmpeg LGPL 与许可记录，`releaseCleared: false`；正式二进制分发前仍须完整对应源码、工具链与重新链接材料审计。

观察到核心 WASM 为 746730 bytes，gzip 325479 bytes。此值只含本编码/封装组件，不含共享 Rust 规划、GPU、输入 decoder、字体、产品和总下载。未做长片吞吐、低端设备或全平台性能承诺。

## 实际证据

| 检查 | 观察 | 范围 |
| --- | --- | --- |
| 实际 WebCodecs AAC 调查 | 三组首包 timestamp=0，输入/解码相关定位延迟 2112 样本 | 调查原始 PCM 留存，不将平台观察写死到实现 |
| packet leaf | 4 个真实 AVC 输入、短写 OPFS 成片；14 类坏输入/预算/取消/IO 拒绝 | 候选，不是产品发布 |
| 实时浏览器执行 | 4 个共享 Rust 计划 → 实际 VideoEncoder → 本地 AAC/MP4 → OPFS | 16/72/91/1 帧，239 个共享调度步骤；测试画面/PCM 为确定性合成输入 |
| 队列 | 多次运行峰值 9–10 个在途帧；最终交付有效 I420 峰值 184320 bytes，PCM 峰值 122880 bytes | 这批短输入，非长视频资源承诺 |
| 失败回收 | 中途取消、渲染在途取消、真实写入异常、错 timestamp/颜色、视频/PCM 字节预算，共 7 类通过 | 未取得成功统计，所有创建帧关闭，OPFS 候选清理 |
| 独立 FFprobe/FFmpeg | 完整视频解码、精确 track/movie 展示终点、BT.709/limited/left、I/P、无丢帧 | 分数帧率、48/44.1 kHz、单/双声道、无音轨与极短片 |
| 音频 | 三组相关检查最佳 lag=0；AAC RMS 不差于同输入/码率的独立参考编码门槛 | AAC 为有损，不等同任意内容的主观音质证明 |
| 浏览器播放器 | 4 个生成文件加载、seek 到 60%、播放到 ended | 独立 HTMLVideoElement 检查，不冒充 Musterwork 正常页面 |
| Native 回归 | 同一 C 源码重新构建固定组件；6 个真实工程 MP4 独立回读通过，5 项编码行为通过 | 已有硬件 H.264/AAC 与 FFV1/PCM 档案 |

参考 CLI 对三组 AAC 仍暴露 419/687/128 个尾部 codec padding 样本；容器展示范围分别严格为 24157/132433/1920 样本。检查明确区分 decoder 私有填充与工程范围，不把参考解码器的尾填充补到成片时长。

本地证据：`video-browser-encoder-observed/report.json`、`browser-aac-delay-observed/report.json`、`browser-mux-check-06/report.json`、`browser-live-export-06/report.json`、`browser-live-independent-06/report.json`、`browser-live-player-06/report.json`、`video-native-external-final/report.json`（均位于忽略的 `.codex-work/`）。失败尝试原样留存，没有修改旧组件内容或放宽时间断言。干净源码的最终组件与门禁另见本页交付记录。

```sh
python3 tools/components/build-browser-encode.py --archive PINNED_ARCHIVE --output NEW_LIBRARIES
python3 tools/components/build-browser-encode-bridge.py --libraries VERIFIED_LIBRARIES --output NEW_COMPONENT
node tools/verification/video-browser-encoder-probe.mjs PLAN_COMPONENT NATIVE_VECTORS NEW_PROBE
node tools/verification/video-browser-mux-check.mjs ENCODE_COMPONENT PROBE_REPORT NATIVE_VECTORS NEW_LEAF_CHECK
node tools/verification/video-browser-live-export-check.mjs ENCODE_COMPONENT PLAN_COMPONENT NATIVE_VECTORS NEW_LIVE_CHECK
python3 tools/verification/video-browser-mux-decode.py --input LIVE_CHECK --output NEW_DECODE_CHECK
python3 tools/verification/media-encode-check.py --component VERIFIED_NATIVE_COMPONENT --project --hardware --output NEW_NATIVE_CHECK
```

## 下一项产品接入

必须把共享视觉/PCM 执行器连接到这里的 createFrame/createPcm，使用同一固定版本和资源授权闭包；真实复杂工程渲染与本次 codec 门禁分开验收。Musterwork 接入不可变组件 pin、持久 export intent/job、唯一执行 owner、OPFS 配额/候选恢复、独立核心回读与正式文件提交，再展示简洁的进度、播放和下载。

浏览器 Agent、生成素材闭环、图片/字幕/效果/完整属性、长片性能、跨端与真实 provider 计费等仍按总方案继续。不能因本专项可生成/播放 MP4，就标记全部功能或产品浏览器导出完成。稳定 v0.1.0 不变。

## 干净源码交付

代码提交 `59e61b7725d4951d3f0efa79dd6469cabb0ed63d` 的不可变开发组件为 `.codex-work/video-encode-sdk-59e61b7`，manifest SHA-256 为 `cab89db8f578b09343039d6881687c8e1a50ff429a26a750fa7b43e9f827aa42`。20 个记录文件，`sourceDirty: false`、`releaseCleared: false`。WASM/gzip 大小与上文一致。

该清单实际重跑结果：`video-encode-sdk-59e61b7-check/report.json`（4 个实时成片、7 类失败、4 个播放器检查）、`video-encode-sdk-59e61b7-leaf-check/report.json`（14 类 leaf 拒绝）以及 `video-encode-sdk-59e61b7-independent/report.json`（独立完整解码、音频质量/零偏移和原始 mvhd 整数 time scale/duration 检查），均在 `.codex-work/` 下。公开 `.d.mts` 声明经 strict TypeScript 检查。

规划组件继续固定 `6141990` 对应清单，见前批记录。Musterwork 组件 pin 本批未提升，182 个原有受保护改动摘要仍一致；正常页面导出验收不因新增组件而自动成立。
