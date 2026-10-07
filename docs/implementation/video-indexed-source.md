# 容器索引驱动的真实视频采样

2026-10-07 · 源码 `e639172ee21049d9af0eda39b1a8b39ea9887981`。工程正放、倒放和随机定位已连接真实原生解码；尚未成为产品播放器、Browser 媒体 Host 或完整导出链。

## 取样合同

`mo-video-source::indexed::IndexedVideoSampler` 消费工程编译出的 `MediaBinding + TimeSample`，通过 `VideoTimeline` 查询素材容器的 movie 时间，再通过 `IndexedVideoDecoder` 取得对应的原始画面。时间提供方可以是同源 `VideoTiming`，也可以是保留索引的 `NativeVideoTimeline`。宿主负责把二者绑定到同一份已授权的不可变字节；内核没有增加文件权限、存储或任务平台。

构造时检查完整 movie 范围、尺寸、真实 tkhd ID、原始时钟和新 decoder 的 generation；端点按精确时间比较，允许等值但不同表示的时间分数。逐次调用检查相同资源绑定、不可变时间摘要、轨道时间基及映射结果。公共摘要/结果校验移到 `mo-media-isobmff`，原生传输和同进程取样共用一套规则。

结果保留精确 `VideoTimeSelection`，包括不能表示为普通 `RationalTime` 的边界；只有 `Sample` 才附带独立拥有的 `Arc<DecodedFrame>`。原始 PTS、duration、色彩、格式和像素均不改写。明确的 `Gap` / `Outside` 不调用解码器。缺帧、PTS 跳过或不匹配、尺寸/时基改变、重复 PTS、坏布局或超预算均使会话失败，不延长前一帧掩盖错误。

既有 `VideoSampler` 仍负责宿主明确证明的连续区间，二者共用帧字节校验和预算规则。不能把文件总时长当连续区间，或用平均帧号代替容器映射。

## B 帧与开放 GOP 的访问起点

真实全边界拖动检查发现：直接按画面 PTS seek，会跨过仍未解码的 B 帧；仅改成目标样本 DTS，在开放 GOP 的 leading B 帧处仍可能从过晚的 intra picture 开始。原有少量随机点未覆盖这些位置，本次增加独立生成的开放 GOP H.264 语料，并逐项核对像素。

索引现在同时保留目标样本 DTS 和可选 `VideoAccess`。后者是在**解码序号不超过目标、显示 PTS 也不超过目标**的 sync 候选中，选择解码序号最大的一个。构建按解码顺序扫描，在按 PTS 排序的坐标上用 Fenwick 前缀最大值查询，成本为 `O(n log n)`、辅助空间 `O(n)`，不在每次 seek 时线性回扫全文件。计数、分配、排序、候选计算均受原预算和取消边界约束。

固定 FFmpeg 9.0.2 的 `mov_seek_stream` 在关闭 edit list 时，`min_corrected_pts` 为 0，并在查 DTS 索引前减去 `dts_shift`。原生适配器传入候选的原始 DTS，由 demuxer 处理其负 CTTS 时钟平移；随后仍要求实际画面的 PTS 精确等于目标。没有增加失败后猜时间或无条件从文件开头解码的路径。

`VideoAccess` 是容器级候选，不是所有 codec 的闭合 GOP 证明。没有候选时，时间映射仍返回应显示的样本，当前采样档案返回明确 unsupported。需要额外 roll/recovery 的格式、field/multiple-picture sample 等还需 codec admission；不能把本次 H.264 语料扩大成全部编码格式承诺。`seek_sample(DecodeTarget)` 保留目标和候选的样本 ID、DTS/PTS，供平台实现 codec 特定的预读要求。

## 缓存、定位与生命周期

- 缓存按原始 PTS 查找，同时维护访问次序索引；查找、更新和淘汰均为 `O(log retained frames)`，没有不断增长的过期队列。
- 同素材的定格、重复片段和 seek 前后复用原始像素。缓存画面保持原 generation；新 decoder 输出必须匹配当前 generation。资源绑定改变不能复用缓存。
- 每次缓存命中仍查询时间 capability 并检查取消，不能掩盖索引 worker 已停止或映射改变。
- 每次读取前按最大单帧字节数预留空间；索引采样不需要 lookahead，允许单帧预算。首次取样、向后定位、拖动或超出顺序工作预算的大跨越使用索引访问点，其余复用顺序解码。
- 错误、取消或回调 panic 同时释放 decoder、timeline 与缓存。Native 下两者各持有受限进程，继续共用原 Owner 的 deadline、I/O 能力和物理回收规则。

活跃帧缓存字节不包含调用方保留的 Arc、容器元数据、allocator、传输缓冲或 decoder 私有分配，不是应用 RSS。同步宿主回调仍须及时返回；总内存、句柄数量、并发任务与恢复由 Host 管理。

## 实际验证

五组素材经过 `VideoDocument → CompiledDocument → TimeMap → IndexedVideoSampler → 原始时钟 decoder worker`。拖动另外覆盖各样本/编辑边界两侧、空白、素材结束和范围之外。独立 FFmpeg 7.1.1 提供实际像素与 PTS，Python `Fraction` 和线性扫描独立验证编辑映射与访问候选。

| 实际素材 | 源画面 | 边界拖动 | 工程正放/倒放 | seek 次数 |
|---|---:|---:|---:|---:|
| B 帧/VFR | 77 | 244 | 180 | 89 |
| 既有 SDK 输出 | 75 | 240 | 180 | 61 |
| 开放 GOP H.264 | 120 | 330 | 240 | 133 |
| signed CTS 元数据变体 | 77 | 244 | 180 | 89 |
| 空白/分数速率/重复/定格变体 | 77 | 562 | 102 | 64 |

共 **426 个源画面、2,502 次取样、436 次 seek、1,314 次缓存命中**，选中画面的 PTS 与像素 SHA-256 同参考一致。每份实际素材测试均执行，0 ignored；正常结束和缓存命中前取消均检查两个 worker 已回收。四帧缓存峰值为 **73,728 active bytes**，只对应 128×96 YUV420P 语料，不是 4K/RSS/FPS 基准。

开放 GOP 由独立 CLI 的 `libx264` 以 `open-gop=1:bframes=3:keyint=30` 参数生成；后两份保持原压缩 payload 不变，不冒充独立相机来源。报告位于 `.codex-work/video-indexed-access-verified/`，`timing/` 保存包、画面摘要及参考命令。

8 项新增采样测试覆盖边界、缓存/独立拥有帧、单帧预算、初始裁切/大跨度定位、错误时钟/缺帧/坏响应、所有观察到的取消点和无访问候选。时间映射的 38,656 个生成查询同时与两种顺序的独立候选扫描比较。相关 Rust 默认检查共 42 passed、20 ignored；外部条件用例另由实际脚本执行，不把 ignored 计作通过。

原 worker 回归核验 77 帧、144,000 AAC 样本及大 packet/回收故障路径；原连续区间 sampler 的 188 次工程查询和三种工程导出通过。相关 `native,gpu` strict Clippy、完整默认 runtime WASM 编译、2 项 SDK 闭包和 6 项验证驱动检查通过。WASM 此处只有编译证据，没有 WebCodecs/浏览器实际解码结论。回归目录为 `.codex-work/video-indexed-{worker,continuous,encode}-verified-02/`，使用包与下面的最终 bundle 字节一致。

## 发行和复现

私有 worker 协议升级为 **`musteroffice.native-media-worker/5`**，增加样本解码位置和访问候选。C 解码 ABI 仍为 2，编码 ABI 仍为 1；本次没有修改固定 FFmpeg 库或 C bridge。`mo-video-runtime::isobmff` 公开同源纯计算索引，runtime 审核闭包仍为 20 包，authoring 仍为 6 包。

干净源码 release bundle `.codex-work/video-indexed-worker-final/` 的 manifest SHA-256：

`56a70d657fb7f60a71622cf9af696e70b6d0cd0f4d48334a1f6c029a1e88fc0c`

7 个二进制共 **8,424,968 bytes**，不含产品、GPU、字体、模型或安装包；源码/clean 状态见 `.codex-work/video-indexed-worker-final-source.json`。产品仍安装旧 19 包 SDK/profile 2 worker，本次未替换 pin 或公开发布新软件包。

```sh
cargo test --offline --locked -p mo-video-source --features native
python3 tools/verification/video-indexed-check.py \
  --input /absolute/path/to/nonfragmented-bframes.mp4 --variants \
  --worker-bundle /absolute/path/to/verified-profile-5-bundle \
  --output .codex-work/new-indexed-source-evidence
```

该次 `current.py` media 分组为 10 步，最后一步改为 `video-indexed-source`：先执行完整实际时间/像素 oracle，再执行索引采样，不重复列为两个顶层步骤。本次未宣称跑过整个 media group 或远程 CI。

## 后续连接

上述采样提交之后，合成器已接入共用精确区间和明确空白结果，sampler 已通过统一资源提供方连通 CPU 预览和真实导出，见[索引画面合成记录](video-indexed-composition.md)。后续仍需升级产品授权素材 Host、预览/导出 RPC 与播放器。Browser I/O/decoder、完整 codec/损坏媒体策略、图片/字体、音频播放、UI 和全部验收仍按[实施记录](video-editing-progress.md)与[验收矩阵](../design/video-editing/acceptance.md)推进。
