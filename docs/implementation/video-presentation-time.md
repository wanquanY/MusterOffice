# 视频容器呈现时间与解码时钟

2026-10-07 · 源码提交 `3c0cf1ae120d4840db2f31b27e5933842cf4577b`。原生实际媒体已验证；产品导入、通用素材采样与 Browser 媒体 Host 仍待接入。本增量不关闭完整视频验收目标。

## 为什么需要独立的时间映射

容器有解码顺序、画面显示顺序和容器编辑后的播放顺序。B 帧使前两者不同；elst 又可裁掉开头、插入空白、重复素材或改变速率。把 packet duration 当画面时长、把文件总时长当连续画面终点，或在内核与解码器两次应用 elst，都会导致定位错误。

`mo-media-isobmff::presentation::VideoTiming` 在原始索引上计算第三种时间。原始索引保持不变，源素材容器的编辑列表与用户工程的剪辑操作仍是两层不同语义。计算只依赖 `mo-time` 和有界元数据，不依赖宿主权限、存储、路径或模型。

## 精确合同

- 按 PTS 排序建立显示索引，保留原解码顺序的 sample ID。普通画面的终点是下一个显示 PTS；最后一个显示样本使用自身 decode delta。`mdhd`、`tkhd` 和样本表总和不被互相替换。
- movie time 与 media time 均用受位宽预算约束的精确分数。elst 支持 empty、重复片段、正分数速率和 rate 0 定格；不转成平均帧率或浮点秒数。
- 查询显式选择 `Before` 或 `After`：分别采用 `(start,end]` 与 `[start,end)`。边界前后可落在不同素材片段；定格的来源点固定按 `After` 选择。
- 结果为 `Outside`、带精确范围的 `Gap`，或带 movie 范围、原始 sample ID/PTS 范围、精确来源点及来源侧的 `Sample`。有显式 elst 时以其编辑区间为准；没有 elst 时使用已声明 track 区间，缺失声明才由原始显示终点求范围，保留前后空白。
- 相同 PTS 的多个压缩样本、负的容器 edit rate 等未实现语义明确返回 unsupported。原始 parser 仍保留声明；工程自身已有的倒放语义不受此限制。fragmented/cslg 等原索引未支持的特征也不被猜测展开。

算法采用可取消的归并排序，构建为 `O(samples log samples + edits)`，查询为 `O(log samples + log edits)`；排序 scratch 和最终记录均受数量与分配预算控制。默认最多 2,000,000 样本、4096 edits、256 位精确算术；worker 还继承原索引的跨轨总预算与物理进程限制。这里是算法复杂度和逻辑预算，未声称完成 RSS/FPS 基准。

参考依据为 [Apple composition offset](https://developer.apple.com/documentation/quicktime-file-format/composition_offset_atom) 对最后一个显示样本的说明，以及 [composition shift](https://developer.apple.com/documentation/quicktime-file-format/using_composition_offset_and_composition_shift_least_greatest_atoms) 对组合时间的区分。[Apple edit list](https://developer.apple.com/documentation/quicktime-file-format/edit_list_atom/edit_list_table) 文档只说明正速率 QuickTime profile，不能作为本实现 rate 0 定格的依据；这里把定格作为独立支持并验证的 ISO BMFF 语义。[W3C MSE ISO BMFF](https://www.w3.org/TR/mse-byte-stream-format-isobmff/) 的零时长无限 edit 规则属于 fragmented MSE profile，不套用到本非分片文件路径。

## 解码器与受限进程

`DecodeTimeline::Demuxer` 保留原解码行为。新的 `RawIsoBmff` 显式选定 MOV demuxer，并要求 `ignore_editlist=1` 实际被消费；格式不符或选项未应用即失败，不退回自动格式猜测。宿主检查 worker 回传的时钟模式。

`StreamInfo.container_track_id` 保存完整 32 位 tkhd ID，与 decoder stream index 分开。测试包含 `2147483649`，避免在带符号的上游 ID 表示或多轨排序中绑定错轨。其他容器不会伪造 ISO BMFF ID。

FFmpeg 即使忽略 edit list，遇到负 CTTS 仍会平移 DTS：`mov_update_dts_shift` 取最大负偏移量，DTS 减去它，随后重建 PTS 时再补回，原始显示 PTS 保持一致。因此验证脚本独立从 CTTS 求参考 DTS shift，比较原始 PTS、平移后的参考 DTS 和未变的包字节范围；不把参考 demuxer 的解码时钟写回原始索引。依据为固定 FFmpeg 9.0.2 的 `libavformat/mov.c`，独立对照工具为 7.1.1。

`NativeIndex.describe_video_time` / `select_video_time` 在已有索引进程内惰性构建并保留映射。宿主只保留有界摘要，检查结果的 sample/edit ID、movie 范围、raw 范围及精确来源点。非法响应使 owner 永久停止；已缓存摘要也不会使后续调用绕过原取消、deadline 或 stopped 状态。索引进程和 decoder 进程仍各有独立额度与同一套生命周期实现。

本次窄 C 解码 ABI 升为 **2**；编码 ABI 保持 **1**。私有 worker 协议升为 `musteroffice.native-media-worker/4`，旧 bundle 明确不兼容。runtime 源码打包审核闭包仍是 20 个自有 crate，authoring 仍是 6 个。产品现有 19 包 SDK/profile 2 worker 尚未升级，不能混用新旧 SDK 和二进制。

## 实际证据

从上述干净源码构建的 macOS arm64 release bundle：`.codex-work/video-time-worker-final/`；manifest SHA-256：

`ae2f0a7f9c88efb53a5ab67c94a54a7d4151be89cee611a9d14678903c4f55c9`

7 个二进制共 **8,424,776 bytes**，其中新 C bridge 为 56,272 bytes，其余 5 个固定 FFmpeg 库与既有核验组件字节一致。不是应用安装包、GPU、字体或模型总大小。构建源码和 clean 状态记录于 `.codex-work/video-time-worker-final-source.json`。

| 实际媒体 | 解码帧 | 时间查询 | 随机定位 | 特征 |
|---|---:|---:|---:|---|
| B 帧/VFR MP4 | 77 | 244 | 11 | 正 CTS、非单位时间基、容器开头裁切 |
| 原 SDK 消费者输出 MP4 | 75 | 240 | 10 | 无重排的工程导出素材 |
| signed-timing MP4 | 77 | 244 | 11 | CTTS v1 负偏移、32 位大 track ID，参考 DTS shift 1001 |
| edits-timing MP4 | 77 | 562 | 10 | empty、3/2 速率、重复、定格、素材结束后 gap |

后两份是对实际压缩素材的受控容器元数据变体；脚本验证 mdat payload 摘要不变。它们不是独立相机/编辑器来源。所有 306 帧实际像素 SHA-256 与原始 PTS 同独立 FFmpeg 解码一致；1,290 次映射同 Python `Fraction` 的独立线性 oracle 一致，42 次 seek 的 PTS/像素与顺序解码一致。实际测试使用短 `ReadAt`，并在关闭后验证两个子进程均已回收。原始模式拒绝 WAV 的检查也显式执行。

最终 bundle 的时间检查证据为 `.codex-work/video-time-final-verified/`。两份元数据变体另有 `.codex-work/video-time-index-variants/`，438 个压缩样本的字节、位置、时钟与独立 demux 一致，并通过真实 worker 分页及故障测试。

6 项纯时间测试覆盖 38,656 次生成排列查询、分数边界、空白/定格、畸形时钟、算术预算及所有观察到的构建/查询取消点；宿主另有 2 项非法时间结果验证。相关 Rust 普通测试共 72 项通过（首轮 71 项，随后新增 1 项取消/算术测试并重跑完整 6 项时间测试）；普通命令中 23 项外部条件测试显式 ignored，不计作通过。实际媒体证据分别由专用脚本取得。

本次 C 原生检查和原 worker 解码回归验证 77 帧、144,000 AAC 样本及大 packet、资源回收/故障路径；3 种原工程导出经独立解码通过。对应目录为 `.codex-work/video-time-{native,worker,encode}-regression/`，与最终 bundle 的 manifest/全部二进制相同。相关库/worker strict Clippy、runtime `native,gpu` 编译、parser/mapping WASM 编译、2 项 SDK 闭包和 6 项验证驱动测试通过。没有扩大为全 media group、真实浏览器、音频 edit mapping、其他 OS 或 GPU 播放验收。

复现入口（组件须已按本次 C ABI 构建）：

```sh
cargo test --offline --locked -p mo-media-isobmff
python3 tools/media-worker/build.py --component /absolute/path/to/component \
  --output /absolute/path/to/new-worker-bundle --release
python3 tools/verification/video-time-check.py \
  --input /absolute/path/to/nonfragmented-bframes.mp4 --variants \
  --worker-bundle /absolute/path/to/new-worker-bundle \
  --output .codex-work/new-video-time-evidence
```

`verification/current.py --groups media` 已增加第 10 步 `video-container-timing`，在原始索引检查之后运行实际 worker 和元数据变体。

## 下一层的责任

后续 `e639172` 已实现索引驱动采样和两种顺序下的访问候选，新增目标 DTS/候选元数据，并升级到 worker profile 5，见[索引采样记录](video-indexed-source.md)。本页 profile 4 和数字保持为历史证据，当前脚本须使用当前协议的 bundle。

映射结果描述容器应显示的样本，不等于所有 codec 都已证明“一压缩样本对应一画面”。目前实际证据只覆盖上述媒体。下一层采样须把 raw 模式、track 身份和每次实际解码的 PTS 绑定起来；缺帧或不支持的 codec 必须失败，不能把持续显示上一帧当成成功。既有 `VideoSampler` 仍面向宿主证明的连续区间，不能简单拿文件总时长填入它。

继续将这一合同接入有界素材采样、产品授权导入、预览/导出；Browser Host 实现同一时钟及解码输出验证。独立 SDK 发行、产品 pin 配对升级和实际页面操作均需各自验证。完整剩余范围见[实施记录](video-editing-progress.md)与[验收矩阵](../design/video-editing/acceptance.md)。
