# 浏览器连续解码与帧借用

2026-10-08 · W2 增量。影片精确选帧已复用有界解码 session；正式工作台播放、声音和导出继续实施。

## 所有权与调度

`BrowserVideoCursor` 持有一个当前帧和已有的 `BudgetedVideoStream`。Rust 仍是影片时间、edit、sample ordinal、before/after 与 dwell 的唯一权威。游标只根据 Rust 已选中的原始 PTS 推进解码，不引入另一套时间线或名义帧率估算。

- 选中原始帧相同时复用当前帧，适用于连续采样、边界两侧仍落于同一帧、重复显示和定格。
- 向前请求继续消费既有解码队列，解码状态、参考帧和已有读取不重新创建。
- 请求更早的原始帧时先完整释放旧 session，再通过实际 IDR 验证恢复点。重复 edit 的回跳遵循同一规则。
- `maxSequentialVideoSamples` 默认 256，是请求间的原始 sample-ordinal 距离调度上限。较大跳转重新定位，避免从旧游标一直解码；这不改变 Rust 的选帧结果，也不是声称整个恢复过程最多解码 256 个包。open-GOP 依赖可能要求从更早的 IDR 重建。
- gap/outside、轨道切换以及转入旧 packet-clock 视频 API 都释放影片游标。`releaseMovieVideo()` 让 Host 在素材不可见或内存压力下显式逐出缓存，保留不可变源和 Rust 索引供后续使用。

当前帧由游标拥有，解码流仍对内部待输出、重排、就绪队列及消费中的帧保留额度。停止消费会反压整个流，不会因保留一帧就偷偷继续把全片解码进内存。这里的“一个当前帧”不等于只有一个平台帧，codec 的有限队列和参考存储仍单独计入。

## 借用与关闭

缓存会跨请求保留同一个底层帧，因而不能只用 `Set<sample>` 判断调用方是否仍有借用权。否则第二次借用同一帧时，第一次回调留存的对象会意外重新有效。

现在每次借用创建独立的活动标记。`copyRgba/cloneFrame` 同时检查该标记、输入生命周期及当前解码状态；回调退出后该借用永久失效，即使同一底层帧正在被下一回调使用。视频/音频旧入口也使用同一借用规则。只有明确调用 `cloneFrame()` 得到的独立 lease 可超出回调存活，继续占用共享预算，直到调用方关闭它。

普通回退、逐出或失败会立即关闭 codec/帧，然后等待已经发出的有界读取实际退出。操作取消/截止仍立即终止借用和 codec，不提前返还尚未结束的读取额度。不能返回的宿主读取仍需终止整个 Worker；没有用假取消状态冒充底层 I/O 已结束。

保留的 cursor 需要 Host 的缓存管理：空闲不等于已经逐出，另一素材可能因共享额度不足而被拒绝。该拒绝在 codec configure 前发生；逐出前一个 cursor 后可重试。库不在背后扩容，也不让产品以额外服务器渲染承担此状态。

## 真实证据

Chrome 154.0.8037.98 / macOS arm64，实际 Worker、Rust WASM 索引与 WebCodecs，沿用[影片解码](video-browser-movie.md)的 5 个真实素材和独立 Fraction/FFmpeg 像素参照。

| 素材 | 时间查询 | 画面核验 | decoder 创建 | 解码包数 |
| --- | ---: | ---: | ---: | ---: |
| B-frame/VFR | 314 | 308 | 1 | 77 |
| signed CTS | 314 | 308 | 1 | 77 |
| 多 edit / 重复 / rate / dwell | 146 | 128 | 2 | 53 |
| false sync | 314 | 308 | 1 | 77 |
| 实际 open-GOP | 314 | 308 | 1 | 77 |
| 合计 | 1,402 | 1,360 | 6 | 361 |

上一版每次画面请求创建 decoder；同一正向语料的 1,360 次创建现在为 6 次。所有完整 Rust 选择、独立像素误差、最近参照帧以及同一 raw PTS 的像素 SHA-256 检查仍通过。这个结果证明该语料的状态复用，并不证明 4K 长时、多轨、倒放缓存或整个工作台帧率。

新增实际 SDK/平台检查：

- 在第二次借用同一帧的回调中，第一次留下的 `copyRgba/cloneFrame` 都返回 `closed`；当前借用正常。
- 嵌套逐出返回 `busy`，不破坏正在使用的帧。
- 暂停消费后解码包与帧计数停止增长，保留 decoder 和独立 clone 时实际跟踪帧数为 4；逐出 cursor 后独立 clone 继续存活并占用 131,072 bytes，关闭 clone 后额度归零。
- 逐出后能重新定位；回调异常保留原错误并释放 cursor；同一 input 能再次使用；较大 sample 跳转和回退分别验证 decoder 重启。
- 切换 packet-clock 入口会释放原游标；另一个 input 的额度拒绝发生在 configure 前，释放后重试成功。
- 真实压缩负载的第 13 个包被破坏后，前面的有效画面仍可读取；向前推进遇到损坏时报告 `unsupported_media` 并释放全部游标额度。随后独立定位回未损坏的前部可重新成功，不复用已失败的 decoder。
- 专用 `Blob` 故障语料在首个实际压缩包读取处停住。观察到该读取进入后关闭 input，调用立即得到 `closed` 且没有 configure；读取尚未退出时仍占用 9,043,968 bytes，放行该实际读取后归零。

测试中的 fault Blob、平台计数和延时只存在于验证 Worker。产品 SDK 没有更改浏览器全局对象，也不依赖测试端口。跟踪帧现在同时包含实际 decoder 输出、显式构造和 clone，避免漏算独立 lease。

开发证据 `.codex-work/video-browser-cursor-check-03/report.json`；旧 packet-clock 视频/AAC 回归 `.codex-work/video-browser-cursor-regression-01/report.json`。最终清洁提交/组件 pin 会另行登记。单独原生/PPT 并未因此重新验收。

## 复现与后续

```sh
pnpm run test:video-browser-media
node tools/video-browser-sdk/build.mjs build NEW_MEDIA_SDK
node tools/verification/video-browser-movie-check.mjs BROWSER MEDIA_SDK MEDIA_PIN INDEX_SDK INDEX_PIN NATIVE_INDEX NEW_OUTPUT ORIGINAL SIGNED_CTS MULTI_EDIT
node tools/verification/video-browser-media-check.mjs BROWSER ORIGINAL NEW_REGRESSION MEDIA_SDK MEDIA_PIN
```

前者同时验证完整像素/时间语料和 `video-browser-cursor-cases.mjs` 的生命周期用例。组件、浏览器和输入必须显式提供；不允许缺少真实解码器时以模拟结果当作通过。

下一步是影片音频原始 clock 与 priming/padding 接线、共享 Rust 合成/DSP 和 Product media Worker；同时继续完整 codec/容器与平台矩阵、OPFS 成片导出和正式 UI。此增量不代表完整浏览器播放器已可由用户验收。
