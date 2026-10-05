# 解码帧到工程采样

2026-10-06 · `mo-video-source` 已实现有界源帧采样，已用真实 worker 接通工程编译计划；仍未完成画面合成或用户播放器。

## 范围与责任

默认 crate 不加载原生库，只有精确时间、视频模型/编译合同和解码值。`VideoDecoder` 由平台提供，要求新实例从素材起点开始，帧按呈现顺序输出，seek 成功递增 generation。`native` feature 提供薄 `NativeVideoDecoder`，创建全新的 `mo-native-media` 会话；浏览器尚未提供该适配器。

构造参数包括固定 `MediaBinding`、明确的**连续呈现区间**及预算。宿主负责将真实不可变资源和实际流选择绑定到 digest/stream，并提供由容器索引确认的连续区间及区间间隙。普通文件总时长、模型生成的时长或平均帧率不能证明这种连续性。该 sampler 不负责推断任意 MP4 edit list、损坏时间戳修复或跨 discontinuity 补帧；多段编辑列表的生产解析仍待实现。

每次调用消费 `mo-video-compile` 给出的 `MediaBinding + TimeSample`，核对绑定未改变。默认无 native feature 可编译到 WASM；这只是同源算法可编译，不是浏览器媒体验收。

## 时间权威：为什么不能把 duration 直接相加

本机真实 H.264 B-frame/VFR fixture 的帧 PTS 严格递增，但部分原始 `AVFrame.duration` 会覆盖到下一帧之后，例如 PTS 5005、duration 2002、下一帧 PTS 6006（时基 1/30000）。另一些帧的 duration 又短于到下一 PTS 的间隔。把所有这些 duration 直接变成呈现区间，会制造重叠或伪造的黑帧间隙。

固定 FFmpeg 9.0.2 的 `libavcodec/decode.c:1570` 保留 packet duration；原始值仍保存在 `FrameInfo` 中。[AVFrame 文档](https://ffmpeg.org/doxygen/trunk/structAVFrame.html)区分 PTS 与时间元数据；官方 [ffplay 的 vp_duration](https://ffmpeg.org/doxygen/trunk/ffplay_8c_source.html)也在正常连续呈现时使用相邻 PTS 差。这里借鉴这一语义，原创实现使用有界有理数和显式区间证据，不移植 ffplay 实现。

本模块在**已明确连续**的区间内，将当前帧覆盖定义为 `[当前 PTS, 下一 PTS)`；最后一帧截至被证明的连续区间终点。结果记录 `NextPresentationTimestamp` 或 `ContinuousSpanEnd`。原始 PTS/duration/色彩/格式保留不改写。重复、缺失、倒序 PTS 明确报错，不静默排序、猜平均 FPS 或套 epsilon。

同一 fixture 还揭示了另一项区别：native 和独立 ffprobe 的 `AVStream.duration` 均为 89089/30000，而首个轨道单项 1× edit list 的 duration 是 3003/1000。验证脚本直接读取该已知语料的 `mvhd/elst`，确认连续区间为 3.003 秒；没有把错误断言改成凭空延长素材。此脚本的窄格式检查仅为测试 oracle，不作为产品解析器。

## 顺序、seek 与倒放

- 顺序播放复用解码状态，以一帧 lookahead 确定当前覆盖区间。B 帧已由 decoder 排出呈现次序；不会根据输入 packet 的 PTS 停止投喂。
- 结果通过既有 `PresentationIndex` 进行 before/after 选择。源时间为负、时基分子不为一和反向边界都用精确算术；原始帧率不参与选择。
- `Seek` 模式在缓存未命中时跳到目标之前的随机访问点；`Continue` 模式顺序推进，向后且缓存未命中时同样 seek。
- 精确边界的 `Before` 查找使用前一个源 tick 作为 seek 提示，然后仍以原始精确时间选择；这不是将采样时间减 epsilon。非整数 tick 只向下取整提示值。
- seek 返回的首帧晚于目标时，不能把未读过的前缀称为 gap。模块在同一工作预算内回到真实素材起点再判定，单查询最多两次 seek。
- 源起点已经确认时，首帧前的区域或完整空源返回明确 Gap；查询不在指定连续区间内返回 OutsideSpan。已证明覆盖内的空隙与尚未索引区域保持区别。
- 缓存保存有限窗口，倒放可直接使用已拥有的帧；超出窗口才重新正向解码。不是把整份长视频解码进内存。

## 预算与失败

`SourceLimits` 分别约束精确计算位数、单帧 active bytes、内部保留 active bytes/帧数、每次查询 decode 调用数（包含 EOF 尝试）。读取下一帧之前先按单帧上限预留预算并淘汰旧帧；内部保留包括 lookahead。当前内部 active bytes 上限为 1 GiB，不把元数据、allocator、解码进程内存和调用方继续持有的 `Arc` 算成同一指标。宿主仍须进行全任务内存/帧句柄管理。

返回的帧用独立拥有的 `Arc<DecodedFrame>` 共享，不复制旧像素；缓存淘汰、seek、关闭不会改变已交付画面。任何查询错误、取消或回调 panic 都停止 sampler、清空内部缓存并 Drop decoder；native Drop 会回收 worker。最后返回缓存结果或新帧前再次检查取消，不能返回旧成功画面掩盖失败。

此层限制计算步数，不自行建立墙钟/任务调度器。宿主可通过取消回调施加整个 query 的期限；底层 worker 对每次原生操作有自己的总 deadline。同步 callback 仍须及时返回，不能宣称强行中断阻塞的宿主 I/O。

## 验证

```sh
cargo test --offline --locked -p mo-video-source
cargo check --offline --locked -p mo-video-source --target wasm32-unknown-unknown
python3 tools/verification/current.py --groups media \
  --ffmpeg-dir .codex-work/ffmpeg-native-final \
  --output .codex-work/new-video-source-check
```

media 分组现在顺序执行五步：C、Rust 安全 adapter、构建 worker、worker 执行、工程源采样。最后一步显式执行 `native_projection` 的 ignored 真实媒体测试，独立 Python oracle 使用本次 CLI 像素和呈现时间戳逐项核对结果。

本机 `.codex-work/video-source-gate-final/video-source/report.json`：

- 2 项时间运算测试及 8 项行为测试：负时基边界、约分/溢出、VFR 左右侧、倒放、有界淘汰/独立拥有帧、seek 过冲、empty/prefix gap、坏帧/重复时间、工作预算、逐取消检查点、变更绑定和 panic。
- 90 次工程正放、90 次工程倒放和 8 次随机定位，共 188 次真实查询。完整 `VideoDocument → CompiledDocument → TimeMap → VideoSampler → native worker` 链路选择实际帧。
- 所有选中的像素 SHA-256、原始 PTS、实际覆盖区间与独立解码参考一致。源含 77 个 VFR 帧，输出工程为 90 个 30000/1001 采样点，未用平均帧号替代 VFR 规则。
- 内部保留 active bytes 最大 221,184（12 帧 × 128×96 YUV420P）。这是该小语料的缓存观察值，不是完整产品 RSS/4K 性能结果。
- Rust 工作区（含 native feature）strict Clippy 通过；默认 sampler WASM 编译通过。未新增浏览器实际播放或跨平台 native 结论。

尚待完成：通用容器连续区间/编辑列表索引、损坏媒体策略、浏览器 decoder、GPU/色彩/图形与文字合成、音频 DSP、编码导出，以及产品 SDK/Host/时间线接入。
