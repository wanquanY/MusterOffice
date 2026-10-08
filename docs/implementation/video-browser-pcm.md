# 浏览器共享 PCM 与影片音频

2026-10-08 · W2 增量。共享计算组件及真实解码验证已实现，产品 Browser Host 的解码调度、OPFS 原子保存、播放与导出接线仍在进行。

## 计算归属

`MovieAudioPlan` / `MovieAudioResource` 从 runtime 门面下移至 `mo-video-pcm::audio_movie`；`mo-video-runtime::audio_movie` 原接口继续重导出。Native 与 WASM 使用同一套 edit 映射、精确端点、显式空 edit、带限变速和裁切边界滤波。TypeScript 不维护另一份影片音频算法。

新增 `mo-video-pcm-wasm` 独立组件，只包含公共值、精确时间、媒体索引值、视频模型/编译值及 PCM/DSP；不链接原生 codec、PPT、GPU、runtime 全包、文件系统或网络。宿主持有实际 WebCodecs 解码器、素材权限、存储能力、原子发布和 Worker 的终止权限。此组件不是新增云端转码服务。

## 原始时钟与覆盖

输入包必须来自同一不可变素材的 **原始** sample table。以首个原始包 PTS 为传输原点，由 Rust `MicrosecondPcmClock::packet_timestamp` 输出微秒整数；不能再使用已经应用首个 edit 的第三方 demux 时间。

浏览器的实际 AudioData 时间不向第一包强行平移。AAC 规范允许解码器按码流信息丢弃 priming，WebCodecs 会相应推进输出时间；保留这个变化才能避免重复裁切。依据：[AAC 注册说明](https://www.w3.org/TR/webcodecs-aac-codec-registration/)与 [WebCodecs AudioDecoder](https://www.w3.org/TR/webcodecs/#audiodecoder-interface)。具体浏览器行为仍以实际输出为准。

Rust 从每块输出的绝对微秒时间恢复唯一采样位置，不累加浮点时长。传输误差最多接受 1 μs，覆盖输入时间量化和 codec 裁切的两次舍入；内部时钟限定 8–384 kHz，此误差范围仍不会同时包含两个采样位置。该容差只适用于此明确的传输适配，不能修复真实缺样、重叠或任意粗粒度容器时钟。

`MicrosecondPcmStream` 复用原有 `PcmInspection` 和 `PcmStreamBuilder`：实际样本数量决定原始覆盖，格式/代次/时间断裂/非法浮点/取消均使整次计算失效。每次仅保留一个 4096 帧 carry；输出 tile 在同一个 decoder 成功 EOF 前都属于未发布 attempt。记录覆盖可能包括编码尾部填充，也可能由 codec 提前去掉一部分；影片映射独立验证所需来源是否真实存在，不靠固定丢弃 1024 样本猜测。

## Worker 草案接口

`createPcmRuntime(module)` 接受已验证、已编译的显式 `WebAssembly.Module`，同一 Worker 初始化一次；没有隐式 URL 或下载。

| 接口 | 责任与边界 |
| --- | --- |
| `PcmDecode.open(optionsJson)` | 绑定 resource/content/stream/decoderProfile/trackId/rawOrigin/sampleRate/channels；当前平台适配接受单、双声道和 8–192 kHz |
| `packetTimestamp(rawPts: bigint, timescale)` | Rust 校验相对于首包的原始采样格，并返回微秒 bigint；宿主在传给浏览器 Number 前仍须检查可表示性 |
| `push(timestamp, frames, rate, channels, bytes, write)` | 接受实际 AudioData，bytes 为 interleaved f32le，单块不超过 8192 帧；同步 write 接收 provisional ref JSON 和独立 Uint8Array，必须返回 true |
| `finish(write)` | 只在同一 decoder 全部包及成功 EOF/flush、所有输出已处理后调用；写入最终 tail，返回完成记录；本方法本身不能证明 codec EOF 或存储提交 |
| `seal(referenceJson)` | 完成后绑定最终 source 身份及 tile 几何；宿主将全部 sealed refs、payload、record 原子发布 |
| `MoviePcm.open(options, record, clock, lookup, read)` | 仅接受宿主授权并已提交的完成记录；movie clock 必须由宿主绑定原始索引，不能凭任意客户端 JSON 宣称它来自某素材 |
| `info()` / `read(first: bigint, frames)` | 精确影片 descriptor 与帧数；随机读取 owned f32le，单次最多 8192 帧；缺块为 pending_source、坏块失败，只有显式空 edit 产生静音 |
| `retainedBytes()` / `clear()` / `free()` | 报告已保留 carry 或 cache/filter；清缓存与释放实例；不表示整个 WASM 线性内存或浏览器/codec 的实际占用 |

options/record 不超过 16 KiB、tile ref 不超过 4 KiB、clock 不超过 1 MiB/4096 edits。64-bit 值为字符串或 bigint；重复/未知 JSON 字段拒绝。write 返回 Promise、false 或抛异常均使该 decode 失效，不能继续完成前缀。宿主落盘失败必须废弃 attempt；完成记录不是 blob 存在性或授权证明。

PCM stream 工作额度 4 MiB；movie DSP 8 MiB；tile reader 工作额度 4 MiB、cache 1 MiB，另计输入、宿主存储、输出回调、调用者持有数据和 WASM 线性内存。尚未将这些额度与 BrowserVideoBudget 及产品调度统一，不能声称整条浏览器媒体路径已具备总内存准入。

同步 Rust/存储回调执行期间 Worker 不能接收取消消息。硬截止必须由产品 Host terminate Worker；每个 attempt 的持久化发布必须另有当前代次/授权检查，不能把仅释放 JS handle 当成已终止物理工作。

## 实际验证与发现

Chrome 154.0.8037.98 / macOS arm64，实际 Worker、WebCodecs AAC、独立 index WASM 和本 PCM WASM。浏览器读取原始压缩包，连续解码、写 4096 帧 canonical tiles，EOF 后密封，再从共享 Rust 影片映射读取；独立 FFmpeg 7.1.1 提供 raw PCM 与应用首 edit 后按声明时长裁切的参照。

| 真实素材 | 原始 PCM 值 | 影片 PCM 值 | 结果 |
| --- | ---: | ---: | --- |
| 3 秒 / 48 kHz / 单声道 | 145,408 | 144,000 | RMS 约 3.32×10⁻⁹，峰值差 2.24×10⁻⁸ |
| 3 秒 / 44.1 kHz / 双声道 | 268,288 | 264,600 | RMS 约 3.84×10⁻⁹，峰值差 2.98×10⁻⁸ |

两种素材每块输出即复制并 close，观测 AudioData 峰值 1、结束为 0。随机窗与整片顺序读取逐位相同。另用实际已解码数据模拟 decoder 丢弃首个 priming 块，保持原时间后影片结果仍相同；缺少可播放数据则拒绝。分块缺失/损坏、写回调抛异常/Promise/false、尾部写入失败、非法 PCM/时钟/长度、重复键、错误 track 身份、越界读取均有实际 WASM 检查。

验证中发现 **中间 flush 不是安全的反压手段**：每 8 包 flush 的真实 AAC 波形相对连续解码 RMS 误差约 0.0217、峰值约 0.165。测试宿主已改为按实际后续输出推进有界包队列，只在真正 EOF flush；不得在产品调度中复用中间 flush 做法。测试宿主的 8 包门槛尚不是正式完整 AAC 码流容量模型。

原生共享模块回归通过：51 项 PCM/runtime 测试通过；需要显式 codec fixture 的旧默认 ignored 测试不计入。另显式执行真实原生影片音频检查：一次 raw decode、36 tiles、保存完成记录再恢复、144,000 影片样本及 5 个随机窗口与独立新建的 pinned native decoder 逐 f32 位一致。Native FFmpeg 9 适配的 raw 端点为 145,024 帧，Browser/独立 FFmpeg 7 raw 为 145,408 帧；分别保留实际覆盖，最终影片都是 144,000 帧，没有强迫不同 decoder 具有同一尾部填充。

原生及 wasm32 strict Clippy 通过。交付构建共用 `tools/wasm-media-sdk/delivery.py`，两个独立 wrapper 分别限定允许的 crate 依赖；仍验证外部 manifest pin、全文件 inventory、禁止符号链接，并记录全部编译输入。此开发组件不公开发布，不更改稳定版。

```sh
cargo test --offline --locked -p mo-video-pcm -p mo-video-runtime --tests
cargo clippy --offline --locked -p mo-video-pcm-wasm --target wasm32-unknown-unknown -- -D warnings
python3 tools/media-index-sdk/test_build.py
python3 tools/video-pcm-sdk/test_build.py
python3 tools/video-pcm-sdk/build.py --output NEW_PCM_SDK --bindgen EXPLICIT_BINDGEN
node tools/verification/video-pcm-browser-check.mjs BROWSER PCM_SDK PCM_PIN INDEX_SDK INDEX_PIN NEW_EVIDENCE THREE_SECOND_AAC_FIXTURE
```

WASM clock 向量还覆盖 empty/repeated edits；这是明确的合成输入向量，不声称修改了源文件或完成产品编辑权限校验。44.1 kHz 双声道素材由两个不同频率的正弦生成，用于发现声道交换和时钟误差。

## 下一步接线

此轮没有替换原 `BrowserMediaInput.audioRange`：它仍采用旧 packet clock 和第三方音频游标。下一步为正式浏览器媒体层建立 raw AAC 配置/码流容量验证、共享字节准入、输入/输出/取消/等待退出边界，使用本组件物化 OPFS tiles，再接产品素材资源和播放/混音。多声道、其他 codec、audio dwell 策略、长时性能、浏览器矩阵、Web 合成/导出与正常登录整体验收仍独立待完成。不能以两份短素材证明整个功能已可验收。
