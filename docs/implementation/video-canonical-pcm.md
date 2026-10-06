# 视频音频的连续解码与分块读取

2026-10-06。状态：计算内核、资源适配及原生/Node WASM 实验已实现；Musterwork 生产存储、后台任务和播放器尚未接入。

关联[音频 DSP](video-audio-dsp.md)、[技术合同](../design/video-editing/technical-spec.md)和[整体进度](video-editing-progress.md)。实现：`mo-video-pcm`；数据档案：`musteroffice.pcm.f32le-tiles/1-draft`。

## 实际发现及决定

音频定位不能假设 decoder flush 是完整解码状态的快照。固定 FFmpeg 9.0.2 源码中，AAC 的 `flush` 清理 overlap，而 `random_state` 在初始化时设定，PNS 会持续推进它。FFmpeg 同时区分 initial padding、trailing padding 与 seek preroll；这些字段不是“seek 后任意区间必然等于从头连续解码”的承诺。[官方参数说明](https://www.ffmpeg.org/doxygen/9.0/structAVCodecParameters.html)。

本次实际复现：新建 worker 连续解码 50 帧，再 seek 到 0，与另一次新建 worker 的连续解码对照。

| 素材 | seek 后首个输出帧最大差 | 再解码至第 21 个输出帧的最大差 |
| --- | ---: | ---: |
| 既有 AAC 音调素材 | 0.141822 | 2.10553e-5 |
| 固定种子噪声、24 kbps AAC、显式启用 PNS | 0.267526 | 0.157183 |

首帧误差含有重叠状态影响；预热后仍存在的噪声样本差异与上述随机状态机制一致。该实验只证明所测档案不能把通用 seek/flush 当作位精确 checkpoint，不声称所有 AAC 或所有定位方式都有相同误差，也不把 FFmpeg 的 API 行为误称为它承诺外的缺陷。

因此，定义一份从新建解码器按源顺序得到的标准 PCM 物化结果。播放、倒放、定位、分析及导出可以复用同一份数据。它是派生缓存，不成为原始素材或工程的唯一副本。缓存不做额外重采样、增益或有损编码；保存的是既有 f32 内核输入。

## 已实现的职责

`PcmSource` 把媒体 binding、声道、源采样格、可用范围、decoder 构建/参数档案及分块大小绑定为内容身份。文档内的 resource ID 不影响内容身份，允许不同工程复用同源块；读取时仍校验当前请求的完整 binding，权限由宿主负责。decoder profile 必须包含构建和影响输出的参数，不能只填“aac”这样的名称。

`PcmSource::normalize` 检查真实帧的 generation、采样率、声道布局、PTS 和 timebase。时间戳必须精确落在声明的 PCM 格点上；不以 duration 累加，不猜测缺失 PTS，也不把舍入后的近似值当作准确样本索引。此处仍依赖宿主提供已正确处理 edit list/priming/padding 的连续可用范围，通用容器解析尚待完善。

`PcmTileBuilder::prepare` 从旧 builder 和一个连续 PCM 块生成候选下一状态和完整 tiles。块边界与 decoder 输出帧大小、宿主调用分块方式无关。输入缺口/重叠、丢失前缀、非有限 PCM、元数据变化或提前 EOF 都拒绝。只裁掉宿主明确声明范围外的数据。原 builder 不变，宿主先完成 blob/索引发布再采用候选；候选中的 `complete/completed_tiles` 是计算完成状态，不是持久保存或断电恢复承诺。

分块大小可设为 256…65,536 个样本帧。本次原生验证使用 4,096 帧，48k mono 每块 16,384 bytes，最后一块保留实际长度。载荷为交错 f32 little-endian，不把大 PCM 塞进 JSON。`PcmTileRef` 只含来源身份、序号、精确起点、帧数、字节数和载荷 SHA-256，已经生成同源 JSON Schema/TypeScript 合同。

`PcmTileReader` 按样本窗口定位块，校验返回索引、身份、边界、长度、校验和和 PCM 有限性后才组成输出；返回的多个块不必常驻内存。内部 LRU 有独立块数/字节预算；窗口大小、读取块数、当前载荷＋解码 tile＋输出＋缓存也受预算约束。未生成的数据是 `Missing`，绝不替换为零或邻近块。已经通过校验的缓存块可以在后续失败或取消后保留。

`CanonicalAudioResource` 将 reader 接到现有 `AudioResources`，先检查 Mixer 的本次输出额度，再访问存储。缺块转换为携带 resource 和样本区间的 `AudioError::PendingSource`，宿主据此调度准备任务。该适配器只负责一个已授权资源；多资源索引、声道转换政策、权限 lease 和任务调度由宿主组装。

`PcmTileStore` 是注入能力。公开 crate 无路径、网络、数据库或原生 codec 依赖。测试中的文件系统 store 只是宿主职责的可执行示例，不作为 MusterOffice 的业务持久存储。Desktop 后续由项目 Host 接本地派生资产目录，Web 由 Browser Host 接 OPFS/本地配额。

## 内存、取消和恢复边界

- builder 默认候选载荷上限 32 MiB，单步最多 1,024 个 tile；旧状态、输入及宿主分配另计。持有一块未满 carry，不持有完整音轨。
- reader 默认工作载荷上限 64 MiB、缓存上限 16 MiB/32 tiles；宿主 callback 必须遵守请求额度。统计不覆盖 decoder 内部分配、进程 RSS、存储实现或调用方持有的旧输出。
- 构建/读取中有检查点，取消/失败不返回部分 PCM 窗口。已完成的不可变派生块可以继续保留，不改变工程 head。
- 暂停时保留活的连续 decoder 和 builder 可直接续作；进程丢失后不能仅凭 PCM 前缀恢复 AAC 私有解码状态。重建须新开连续解码并重放到所需位置，复用/校验已发布块。不能冒用普通 seek 作精确恢复。
- 缓存淘汰不删除源文件。宿主负责磁盘额度、读 lease、不可变发布、孤儿清理、持久化和取消任务。core `complete` 不证明宿主 commit 已发生。

这一方案把**热随机读取**从 codec 历史中分离，但没有消除首次物化成本。长文件首次访问尚未物化的远端时间区间需要调度连续准备；不能宣称它已经达到冷定位 p95 目标。后续要实测导入准备、后台预算和冷/热访问，必要时研究经验证的 decoder checkpoint/index；最终目标及性能门槛不因此下调。

## 本次验证

- 默认 10 项测试：任意 decoder 分块产生同一 tile/摘要；负起点、明确首尾裁切、缺口/重叠、提前 EOF；负索引与边界/随机窗口；缺块、错误索引、载荷损坏、binding 变化；逐检查点取消；核心读写预算；准确 PTS；PendingSource 发布后可继续读取；Mixer 额度在存储访问前拒绝。
- 真正 Native/WASM 运行：完整与部分物化两种情况，共 14 查询，块身份/摘要、源完成状态、PCM bits 和缓存统计完全一致；两端都拒绝跨入未发布 tile 的窗口。数据格式和 reader 是同一份 Rust 代码。
- 两段真实 AAC 都各输出 144,000 样本和 36 块；decoder 关闭后重新打开文件索引，分别执行 132 次随机/反向窗口读取，共 264 次、429,656 个窗口样本，与本次连续解码逐 bit 相同。
- 同一磁盘 reader 通过 `CanonicalAudioResource` 驱动真实 `AudioMixer`：两个完整倒放输出逐 bit 等于各自连续 PCM 的逆序，不在内存里预存整段音轨再伪装窗口读取。
- 实际 builder carry 最高 16,384 bytes；reader 工作载荷最高 78,896 bytes，配置上限 131,072 bytes。它们只是已定义的核心载荷指标，不是总内存/FPS/定位延迟结果。
- 独立 CLI 重新解码比较：音调最大差 `2.2352e-8`、RMS `3.3189e-9`；含噪素材最大差 `4.4703e-8`、RMS `4.6510e-9`。旧 CLI 的已知尾 padding 只在参考输出裁到 144,000，未裁剪本实现来匹配。

Native/WASM 完整物化结果摘要：`634050642fc185f43c1123134f60d326a5b9f08a431fd0bfd022e516a25f12ac`；部分物化摘要：`7157e5ebd58efbf50dbf444825deda37bf962ca565dd5240cb53ceca0c3ee615`。音频 DSP 既有跨端结果摘要保持不变。整个 Rust workspace strict Clippy、schema/TypeScript 和新增门禁编排测试通过；未据此声称整套产品验收通过。

## 复现与后续

```sh
cargo test -p mo-video-pcm -p mo-video-audio --locked --offline
python3 tools/verification/video-pcm-check.py \
  --native-evidence <native-media-evidence> \
  --media-evidence <worker-media-evidence> \
  --worker-bundle <verified-bundle> --output <fresh-directory>
node tools/verification/video-pcm-parity.mjs \
  target/debug/mo-video-conformance <this-build-wasm.js> <fresh-report.json>
```

`current.py` 已分别在 `media`/`wasm` 组接入。实际本地材料为 `.codex-work/video-pcm-gate-verified/report.json`、`video-pcm-parity.json`、`video-pcm-workspace-clippy.log`，不把 ignored 的真实测试当作默认已通过。

后续仍需：Device/Browser Host 存储与任务接入、统一磁盘缓存预算、权限和恢复验收；通用容器时间/priming 元数据；生产保调任务调度（[计算和派生分块已实现](video-audio-retime.md)）；声音设备与 A/V 时钟；工程 UI 与 Agent 操作；完整编码导出及产品验收。该模块没有独立发布新版组件频道，也没有启用服务器音频渲染。
