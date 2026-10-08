# 浏览器影片时间与真实解码接线

2026-10-08 · W2 增量。共享 Rust 影片时间索引已连接浏览器实际 AVC 解码；完整 Web 工作台仍在实现中。

## 实现与职责

`BrowserMediaInput.open` 可接收宿主已校验、已初始化的独立 `mediaIndexRuntime`。媒体组件在专用 Worker 中，从同一个不可变 Blob 建立 Rust 索引，再按该索引的原始字节范围读取压缩包。宿主仍负责素材身份、授权、组件 pin、Worker 硬截止和最终文件提交。组件不接受网络地址、不上传素材，也不引入服务端剪辑资源。

新的 `movieVideoInfo(id)` 返回影片终点及媒体/影片时基。`movieVideoAt(id, TimeSample, consume)` 调用 Rust `selectVideo`，返回 `outside / gap / sample`；只有 sample 会真实解码并调用借用帧回调，回调区间属于 **影片时钟**。现有 `videoAt/videoRange` 的第三方 packet-clock API 保留原语义，两者不能混用。

流程为：

1. Rust 按精确有理数和 before/after 选择影片片段，处理非零起点、空 edit、重复引用、rate 与 dwell。
2. TS 薄层验证公共 JSON ABI，按最多 128 项的页读取原始 sample ordinal、PTS/DTS、字节范围及 sample description 身份。
3. 原始包源在读取压缩数据前检查字节额度，再验证 AVC 参数和 NAL 边界。原始 PTS 直接进入平台解码器，不叠加第三方 demuxer 的首 edit 偏移。
4. 解码输出必须覆盖 Rust 选中的原始 PTS；缺帧、越过选中帧或不能精确通过平台时钟时报告错误，不假装为空白，也不沿用上一帧。
5. 借用帧、独立克隆和解码过程继续使用既有共享额度。借用回调结束后关闭帧；所有权显式转移的 clone lease 保持原合同。

时间规则没有在 TypeScript 重新实现。第三方固定源码仅提供 codec 配置、显示信息和经审计的平台差异处理。其封装的秒数接口需要安全整数和 microsecond 往返检查；不能表示的原始时钟会拒绝，不能据此宣称浏览器 WebCodecs 支持任意宽时钟。

## 恢复点与资源生命周期

`stss` 以及第三方 key 标记均只作为候选。当前 AVC 原始包源检查实际 IDR；遇到非 IDR 的 sync/open-GOP 候选，会向前寻找更早的 IDR，从那里恢复依赖并顺序解码。Recovery Point SEI 不被当成已经证明可独立呈现的起点。缺少可证明恢复点时明确拒绝。

单一 sample description 是当前已验证范围。description 切换、HEVC/AV1/VP9、交错及 alpha 等仍需各自的配置、恢复点和容量模型，不能落回无额度路径。

真实连续定位暴露了生命周期缺陷：上一定位虽然已关闭 decoder，其未完成的 Blob 读取仍占用基础额度；下一定位提前申请额度会误报资源不足。`BudgetedVideoStream` 现在在普通 iterator 结束时等待已发出的有界读取真正退出，再让串行调用结束。取消立即关闭解码器/帧；不能返回的读取仍由 Host 终止该 Worker，不能提前返还仍在使用的额度。

视频预算是应用准入权重，不代表完整浏览器进程或硬件驱动内存测量。Rust 索引另有有限元数据、轨道、sample/edit 数量限制；WASM 内存高水位和同步执行抢占仍由 Worker 生命周期管理。

## 验证

真实 Chrome 154.0.8037.98 / macOS arm64 Worker，同进程加载两份按 manifest 校验的独立 SDK。每次选帧与原生 Rust 完整返回值一致；独立 Python Fraction 扫描检查片段映射，FFmpeg 忽略 edit list 后提供原始 PTS/像素参照。

| 语料 | 影片查询 | 实际呈现核验 | 内容 |
| --- | ---: | ---: | --- |
| B-frame/VFR 原素材 | 314 | 308 | 每个呈现边界的 before/after 及区间中点 |
| signed CTS | 314 | 308 | 负 CTS，保留 unsigned track ID 2147483649 |
| 多 edit | 146 | 128 | 前后空白、重复引用、1.5 倍速、定格、源末尾以外 |
| false sync | 314 | 308 | 只改 sync 表，将依赖帧假标记为候选；压缩负载不变 |
| 实际 open-GOP | 314 | 308 | 独立 x264 重编码，确认存在 3 个非 IDR sync 候选 |
| 合计 | 1,402 | 1,360 | 不把 outside/gap 计为解码成功 |

所有实际图像与 FFmpeg 对比的最大单帧 RGBA 平均误差约 0.659/255。除误差阈值外，还检查所选参照帧是全片中最近的像素匹配；同一原始帧跨定位/edit 的浏览器像素 SHA-256 必须相同，避免仅凭宽松阈值放过邻帧错误。空白/片外查询不启动 decoder。

错误与资源检查：压缩包上限 1 byte 在 codec configure 前拒绝；把首个 IDR 改成依赖 slice 的损坏素材也在 configure 前拒绝；借用回调永不返回触发 deadline；回调中取消立即生效。全部结束后 decoder/frame/额度为 0，暂存平台帧峰值 3，16 MiB 共享池峰值 9,060,174 bytes。这不是全应用内存或长时播放性能结论。

原有入口回归：77 帧视频、16 次精确定位、144,384 个 AAC 采样值、100 个连续音频窗口、独立 clone 存活及并发额度拒绝均通过。音频仍是原先的 packet-clock 入口，144,384 包含参考尾 padding；没有冒充影片有效 144,000 样本的音频接线已经完成。

开发报告：`.codex-work/video-browser-movie-check-05/report.json`、`.codex-work/video-browser-movie-regression-01/report.json`。最终干净提交的构建与复验记录另行登记。

最终干净源码 `fd2bcc60830e6868a6b813ca6b663de0fbb6aef3` 已构建并复验：媒体 SDK `.codex-work/video-browser-movie-final/`，manifest `eb604759e2e6d33560f7ea475c6a565ac999788761669c122a7f9593fd042ab4`；仍配合索引 SDK `c857b483e0e285fc08c9cbca4977dde7e2f0b7f1f89ef51357466e3df59595f9`。报告 `.codex-work/video-browser-movie-final-check/report.json`、`.codex-work/video-browser-movie-final-regression/report.json`。媒体 ESM 324,720 bytes，gzip 86,267 bytes；含对应源码的完整媒体 SDK 2,272,751 bytes、90 个文件。索引 WASM 另算，不将这一 ESM 体积当成整个剪辑器体积。

```sh
pnpm run test:video-browser-media
node tools/video-browser-sdk/build.mjs build NEW_MEDIA_SDK
node tools/verification/video-browser-movie-check.mjs BROWSER MEDIA_SDK MEDIA_PIN INDEX_SDK INDEX_PIN NATIVE_INDEX NEW_OUTPUT ORIGINAL SIGNED_CTS MULTI_EDIT
node tools/verification/video-browser-media-check.mjs BROWSER ORIGINAL NEW_REGRESSION MEDIA_SDK MEDIA_PIN
```

验证脚本从第一份原素材派生 false-sync、missing-IDR 和真实 open-GOP 语料，并保存于输出目录；依赖独立 FFmpeg/ffprobe 和已构建的 `portable_index` 原生验证宿主。输入路径和组件 pin 必须明确传入。

## 后续工作

这次完成了共享影片时间到实际画面的单点定位。后续已实现[有界连续解码与帧借用](video-browser-cursor.md)，同一实际语料的画面解码 session 创建数从 1,360 降至 6；完整播放性能仍需正式 Host、合成与音频共同验证。音频的原始 clock、priming/padding、完整 codec/容器、色彩、浏览器平台矩阵、导出、长时性能及用户正式页面验收仍分别推进。组件没有公开发行，稳定 v0.1.0 未改变。
