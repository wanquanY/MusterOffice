# 原始 PCM 与容器音频时间映射

2026-10-07 · 内核增量已实现、真实 AAC 验证通过；尚未接入产品音频导入、声音设备或最终成片。

## 本次解决的问题

原先的 PCM 分块和工程混音需要宿主预先提供准确的连续音频范围，产品因此将音轨标记为 `EXACT_AUDIO_TIMING_UNAVAILABLE`。不能把 `duration_ticks`、压缩包数量或视频帧数当作音频的实际样本数，也不能把原始 AAC 的前置填充当作用户应该听到的内容。

本次新增真实解码检查和容器时间映射，使这层语义属于内核。产品仍负责授权读取、不可变内容验证、后台任务、派生 PCM 存储、完成后发布以及设备输出；没有加入路径、网络、权限或业务数据库。

## 数据与计算链

```text
授权原始内容
  ├─ 固定 worker / raw ISO BMFF 解码
  │    → PcmInspection：真实 PTS、声道、样本数、平面及有限数值
  │    → 成功 EOF 后的 InspectedPcm（原始覆盖范围）
  │    → 原有 PcmSource / PcmTileBuilder / PcmTileReader
  └─ 固定 index worker 的 track / edits 有界分页
       → MovieTimeline：精确 movie → raw media 映射

两者 + 内容/流绑定
  → MovieAudioPlan / MovieAudioResource
  → 原有 AudioResources / AudioMixer / 导出音频计算
```

`MovieTimeline` 从原 `VideoTiming` 中提取；视频仍使用它，音频不再复制另一套 edit-list 算法。保留 movie clock、media clock、16.16 rate、空编辑、重复片段、零时长编辑的原索引和精确左右边界。二分查询复杂度为 O(log edits)。共同限制和错误移到独立 `timing` 模块，原 `presentation` 重新导出相同类型以保持调用兼容。

`MovieClock` 允许只传时钟和编辑表。`NativeIndex::movie_timeline` 复用现有 Track/Edits 消息，每页最多 128 项，不把全部压缩样本表传回宿主，也不要求变更 worker 协议或重新构建 codec。

### 实际解码覆盖

`PcmInspection` 一次处理一个独立拥有的解码帧，只保留时钟和计数。首个真实 PTS 是样本零点，允许负值和非整数采样相位；后续帧必须落在该相位的精确样本格点且与上一帧连续。时间单位可以改变，只要表示的是同一真实时刻。忽略不具有 PCM 覆盖意义的 frame duration。

检查实际 sample rate、命名声道、generation、平面布局和每个样本的有限值。缺 PTS、缺段、重叠、seek、错误格式、取消、预算超限或回调 panic 都使检查器失效；不能把此前的前缀随后完成为成功。只有宿主收到同一个新解码器的成功 EOF 后才可调用消耗检查器的 `finish_at_eof`。这个合同不是 EOF 的密码学证明，宿主必须保留物理执行与错误处理责任。

检查结果明确是**原始解码范围**，不自动声明为最终可听范围。`continuous_asset` 仅验证显式选择的连续子范围，不推测 priming/padding/edit-list。`PcmSource::normalize` 与检查器复用同一个内部 `DecodedClock`，避免两套 PTS 归一化逻辑分歧。

### 映射与采样

`MovieAudioPlan` 将每个精确 movie 边界乘以源采样率后取绝对上界 `ceil`，不累加各段取整时长。输出仍保留精确的 movie end；即使最终端点位于两个音频采样时刻之间，也不修改文档时间。

- 空编辑是容器明确声明的静音；缺失、损坏或未物化的源 PCM 返回错误，不能变成静音。
- 重复片段读取同一个原始 PCM 区间，不重新启动有状态解码器。
- 正 rate 使用已有带限 `SincResampler`，按精确相位计算每个请求；容器 rate 是变速变调，不能冒充工程的保调效果。
- 各编辑区间以外的滤波支撑零延伸，不把已排除的前置填充、尾部填充或相邻语音混入该段。
- 非空的零 rate 音频编辑明确返回 unsupported。它不等于普通连续音频，产品不得自行当作某种已确认的静音/循环政策。视频定格仍按原语义支持。
- 默认每次读取最多 8,192 个输出采样帧，最多保留一个 rate 滤波器；输出、源窗口、计算结果和滤波器合计默认上限 64 MiB，另受 DSP 工作量和 PCM 限制。宿主自身缓存与调用方已拥有的内存须另计，不把此值宣传为进程 RSS。

建立源时核对原始 descriptor、实际解码范围、内容 digest 和流绑定；读取时再次核对原始能力的 metadata 与返回窗口。MovieTimeline 必须来自同一内容和同一 track，且 PCM 必须是 raw 时间轴，不能把已经应用 edits 的 demuxer 输出再次映射。原始数据的存储身份还应包含固定 decoder profile，沿用既有 canonical PCM 合同。

映射档案为 `musteroffice.movie-audio.raw-pcm-sinc-edits/1-draft`。宿主若缓存映射后音频，应将此档案、原始 decoder/PCM profile、内容和流一起纳入处理身份；不能只拿原文件摘要当作派生结果身份。

## 验证

常规四 crate 回归共 **66 项通过、0 失败**，该命令中另有 21 项需要专用素材或 worker 的 ignored 检查；其中本次新原生用例另行显式执行通过，其余不计作通过。四 crate `--all-targets` strict Clippy 与格式检查通过。默认 runtime 通过 `wasm32-unknown-unknown` 编译检查；尚未将本次新增映射加入真实浏览器的 Native/WASM 数值对照，不扩大此结论。

新用例覆盖精确负相位、跨 JS 整数精度的 PTS、实际样本计数、不同时间单位、缺段/重叠、格式变化、非法 PCM、全局预算、失败后禁止完成、取消/panic；还覆盖空白/重复/变速编辑、亚采样编辑边界、不同分块与倒序随机读取、每个读取阶段取消和能力变化。原 VideoTiming 的 VFR、B 帧和编辑边界回归保持通过。

真实媒体检查使用既有 VFR H.264 / AAC 素材，内容摘要 `ca7211a9b5e1bbdb66c7bd98e515484d7b6bc5ee8bbeeadb6cabb06d90bd73a6`，固定 worker manifest `4a4b194f09fdf2893b749dd5d3c4ba8ad54a1891a3648945341192522ab66fb3`。实际现象：

| 项目 | 实测 |
| --- | --- |
| 原始 AAC 解码 | 142 个解码帧，共 145,024 个 48 kHz 单声道采样帧 |
| 容器 edit | raw media 从第 1,024 个采样帧开始，movie 长度精确 3 秒 |
| 映射后 | 144,000 个采样帧 |
| 对照 | 与同一固定 worker 的新 demuxer 解码结果逐个 f32 bits 相同 |
| 连续结果 SHA-256 | `40b3b2bf3dd2790d5cb63752401e1b7a1559f1ce5280d6df1ff90bce65e46def` |
| 随机读 | 5 个首/中/尾区间与连续结果一致 |

索引和解码均通过实际隔离 worker，明确使用未修改的 profile 5。测试中的 PCM 存储是内存 fixture，不是产品派生存储。未把它称为声音设备播放、生产编辑器或全部 AAC 容器兼容性验证。

复现：

```sh
cargo test -p mo-media-isobmff -p mo-video-pcm -p mo-native-media \
  -p mo-video-runtime --features mo-video-runtime/native --offline --locked
cargo clippy -p mo-media-isobmff -p mo-video-pcm -p mo-native-media \
  -p mo-video-runtime --features mo-video-runtime/native --offline --locked \
  --all-targets -- -D warnings
cargo check -p mo-video-runtime --target wasm32-unknown-unknown --offline --locked
```

显式原生用例通过环境变量 `MO_MEDIA_TEST_BUNDLE`、`MO_MEDIA_TEST_BUNDLE_SHA256`、`MO_AUDIO_MOVIE_FIXTURE` 和不存在的 `MO_AUDIO_MOVIE_REPORT` 路径绑定实际文件，再运行：

```sh
cargo test -p mo-video-runtime --features native --test native_audio_movie \
  --offline --locked -- --ignored --nocapture
```

## 接入状态与下一步

干净源码 `636ad72376d16ddb1594e6f87d564379eb844ba8` 已构建 20 crate 的本地不可变开发包 `0.1.0-video.20261007.4`，manifest `5c6e086daa6b6684597993345abc8f300ec439afe53804f4412c707206472628`，`sourceDirty=false`、`releaseCleared=false`。这是本地开发产物，未对外发布或提升产品频道。

独立消费者仅复制 registry archives 和索引，使用固定版本、checksum 及完全一致的外部 Cargo 依赖图执行新公开 API；没有导入本仓库源码。再次经过真实 raw 解码、分页索引和 movie 映射，144,000 个结果采样帧与对照逐位一致，32 个随机窗口一致。`native` / `gpu` features 用于完整固定包闭包；本检查没有执行 GPU 或设备播放。复现工具为 `tools/video-sdk/check-audio-movie.py`，参数为 `--bundle`、`--sha256`、`--worker`、`--worker-sha256`、`--fixture`、`--output`；消费者程序也已保存为独立 Rust 文件。

Musterwork 当前仍固定 `.20261007.3`，保留画面导入和无声预览的准确提示。下一步需要消费新的固定 SDK，在产品授权媒体任务中接通快速 movie 描述、后台 raw PCM 物化/验证/发布与恢复，再将原声导入和 `MovieAudioResource` 接入统一源集合。音频设备时钟、持续视频调度、图片、最终候选导出及 Browser 媒体执行继续按[完整验收矩阵](../design/video-editing/acceptance.md)推进。

本地日志/摘要报告位于忽略的 `.codex-work/video-audio-movie-*`；完整本地用户验收仍未完成。
