# 浏览器共享影片时间索引

2026-10-08 · W2 增量。实现了浏览器实际调用原生共用容器索引与精确时间映射；完整 Web 剪辑播放/导出仍未完成。

## 架构与边界

原生私有 wire 中的分页索引查询移入 `mo-media-isobmff::query`；原 wire 原样重导出，序列化协议不变。新 `IndexedMovie` 持有只读索引及按轨道缓存的 `VideoTiming`。原生 worker 和新的 `mo-media-index-wasm` 都调用同一实现，不在 TypeScript 重写 edit list、CTS/DTS、before/after 或定格映射。

独立 WASM 只依赖媒体值类型、ISO BMFF、精确时间与公共序列化，不链接 codec、GPU、PPT、文件系统或网络。它只接收宿主明确提供的同步 `ReadAt` 字节能力；解析完成后不保留该回调。宿主负责素材身份、授权、任务、Worker 生命周期与硬截止。

浏览器在专用 Worker 内使用 `FileReaderSync.readAsArrayBuffer(blob.slice(...))`，Rust 单次读取不超过 64 KiB，跳过 `mdat` 媒体负载，只读取有界元数据。没有把完整素材读入 WASM。同步文件读取用于 Worker 的边界依据 [File API](https://w3c.github.io/FileAPI/#readAsArrayBufferSyncSection)。

同步 Rust/ReadAt 执行期间，Worker 无法处理取消消息。因此不放置一个实际无法更新的“取消标记”来宣称支持抢占；产品 Host 必须在取消/截止时 terminate 对应 Worker。释放 index 可重用实例内存，但 WASM 线性内存高水位不保证缩回；释放整个 Worker 是宿主的退休边界。

## 草案接口

SDK 入口 `createMediaIndexRuntime(module)` 只接受宿主校验并编译的 `WebAssembly.Module`，每个运行时初始化一次。无默认 URL、fetch 或跨仓库私有源码依赖。当前入口是薄 ABI，JSON 返回值的 64-bit 字段一律为十进制字符串；轨道 ordinal、u32 ID/计数、u16 页长保持安全整数。有理数继续使用 Rust `ExactRatio` 的字符串分子/分母。

| 方法 | 语义 |
| --- | --- |
| `MediaIndex.open(byteLength: bigint, readAt, limitsJson)` | 解析不可变源；readAt 返回 Uint8Array，可短读，不能超过请求长度；空 options 使用默认有限额度 |
| `summary()` | movie 时基、声明时长、轨道/mdat 数、实际读取字节量及 profile |
| `query(json)` | 轨道、sample/edit/mdat 页及 sample description/auxiliary 字节页；最多 128 项或 4096 bytes |
| `videoTiming(track)` | 原始 composition 范围和影片可播放终点，延迟创建共享时间索引 |
| `selectVideo(json)` | 在影片时间上选择 outside/gap/sample；保留原始 sample ordinal、DTS、精确原始区间、候选 access point 和 edit 身份 |
| `movie(json)` | 音视频共用的影片映射分页，保留显式空 edit、重复引用和零速 dwell；不自行选择音频 dwell 策略 |
| `free()` | 释放此 Rust 索引；宿主停止使用该 handle，必要时 terminate Worker |

`track` 参数是索引内的 **zero-based ordinal**，不是 tkhd ID。输出 `trackId` 保留原始 ID，包括大于 `i32::MAX` 的合法值。sample 的 `syncSample` 仍只是容器声明的候选恢复点，不能取代 codec 验证与实际解码覆盖检查。

`movie` 请求包含 `track`、`uneditedMediaEnd`、`start`、`count`。fallback endpoint 只用于没有 edit 且没有声明影片时长的情况，由实际源覆盖提供；不能把压缩包时长推测成连续 PCM。

输入 JSON 限制 16 KiB，拒绝重复/未知键；64-bit 时钟不通过 JS Number 往返。响应分页有界，并设置 2 MiB 最终编码上限。损坏容器、外部 data reference、超预算、无效回调不产出部分索引。fragmented/compressed movie 等原解析器未覆盖的格式仍明确拒绝。

## 实际验证

| 项目 | 证据 |
| --- | --- |
| Rust 专项 | ISO BMFF / wire 共 25 项通过；另有一个需要显式素材的旧测试默认 ignored，未伪装为通过 |
| 精度与错误 | 新用例覆盖大于 `2^54` 的 edit 时钟、负 CTS、单侧边界/空 edit/dwell、分页、重复键、非法小数 track、已缓存查询的取消 |
| 真实浏览器 | Chrome 154.0.8037.98 / macOS arm64，实际 Worker + FileReaderSync + Rust WASM；3 个真实压缩素材，各 39 查询，合计 657 个压缩包记录 |
| 双重参照 | 浏览器完整返回值与原生 Rust 一致；压缩包 PTS/DTS/字节位置独立对比 ffprobe；edit 选择由 Python Fraction 扫描 oracle 对比 |
| 读取 | 每个素材仅读取 4,403 / 4,403 / 4,463 bytes；强制 13-byte 短读仍成功；未读取完整媒体负载 |
| 回调/截止 | 空结果、过长结果、错误类型、抛异常均拒绝；确认进入同步阻塞回调后，由 Host 终止该 Worker，替代 Worker 正常工作 |
| 原生回归 | 重新构建当前原生 worker，3 个素材各 77 帧真实解码；1,050 个影片查询及 32 次 seek 与独立像素/时间参照通过 |
| 构建 | 独立 WASM 构建、相关 Rust strict Clippy、组件 pin/额外文件/嵌套 manifest/符号链接检查通过 |

原生回归 `.codex-work/media-index-native-regression/report.json`，worker manifest `2638596baa07691a1a36ae11d5bea63e3a02f6f2e8a9746fb4c811fc7d351db1`。浏览器开发证据 `.codex-work/media-index-browser-03/report.json`；最终干净提交与交付 pin 会单独登记。

最终干净源码 `e0416656ba07c1dea9ddd45d56fc0f15a909f5fc` 已构建并实际复验：SDK `.codex-work/media-index-sdk-final/`，manifest `c857b483e0e285fc08c9cbca4977dde7e2f0b7f1f89ef51357466e3df59595f9`；报告 `.codex-work/media-index-browser-final/report.json`。WASM 504,013 bytes、gzip 170,568 bytes；完整 SDK 541,797 bytes、11 个 inventory 文件。此体积只包括索引/时间组件，不包括媒体解码、渲染、整个 Web 工作台或运行内存。

```sh
python3 tools/media-index-sdk/test_build.py
cargo test --offline --locked -p mo-media-isobmff -p mo-media-wire
python3 tools/media-index-sdk/build.py --output NEW_SDK --bindgen EXPLICIT_BINDGEN
cargo build --offline --locked -p mo-media-isobmff --example portable_index
node tools/verification/media-index-browser-check.mjs BROWSER NEW_SDK PIN target/debug/examples/portable_index NEW_EVIDENCE FIXTURE...
```

## 不能由此推断的完成项

此组件完成 **movie→原始时间/包位置** 的计算，不等于浏览器 codec 已按该映射呈现画面。当前第三方 MP4 demuxer 只采用首个有效 edit，原 `BrowserMediaInput` 的 packet clock 已带该 offset；接线时必须使用原始包身份/时间桥接并验证实际输出，不能把新 movie 映射再叠加到已偏移的时间上。

后续浏览器媒体层已消费原始包与精确选帧结果，并通过实际解码，见[影片解码接线](video-browser-movie.md)。共享 Rust 合成/音频、正式工作台与 OPFS 导出继续实施。多 codec/容器、完整音频 priming/padding、HDR、长时性能、四类浏览器及产品整体验收仍独立推进。此开发组件未公开发行，不更改 v0.1.0。
