# MusterOffice Video 技术规格

2026-10-06 · v1.0 · 完整技术目标，部分基础正在实现。上位：[完整方案](../video-editing.md)。依据：[研究证据](../../research/video-editing-2026-10-06.md)。本文件中的 API、crate 名称与格式版本不是已发行接口；实际源码与验证范围见[实施记录](../../implementation/video-editing-progress.md)。

## 1. 不变量

- 同一快照、资源字节、计算档案与操作，必须产生相同编辑结果及结构诊断；媒体硬件输出不要求字节相同。
- 原素材不可变；每个剪辑决定、分析注释和输出都能追到精确资源与工程版本。
- 拒绝操作后工程不变。操作成功须有可保存回执，重试不重复产生修改。
- 时间线变更原子；预览状态与持久化 head 分开；界面不会把未提交的候选当成已经保存。
- 解码/渲染/输出队列有界，取消能传播到所有有资源所有权的层。
- 浏览器和桌面共用语义；不支持的能力明确返回，不静默丢弃或换成云端计算。
- 字节摘要、对象 ID、请求 ID 只做身份，不授予读取权。

## 2. 模块与依赖

建议按职责组织，文件接近 1,200 行评估拆分，不超过 2,000 行。下表不是要求机械建立同样数量的 crate。

| 单元 | 职责 | 禁止依赖 |
| --- | --- | --- |
| `mo-video-model` | 工程、资源描述、时间范围、轨道、片段、效果、校验 | UI、网络、数据库、解码器 |
| `mo-video-edit` | 语义操作、暂存候选、读写集合、逆操作、增量失效 | 设备、任务平台、Musterwork |
| `mo-video-compile` | 工程→视频/音频计算图、时间映射、能力和输出计划 | 特定窗口、任意脚本执行 |
| `mo-video-render` | 图像合成数学、效果合同、共享 shader、CPU 参考 | 产品身份、持久保存 |
| `mo-video-audio` | 样本调度、混音、自动化、重采样/时伸接口、响度 | 浏览器 DOM、音频设备选择 |
| `mo-video-operations` | 无界面创建/查询/编辑/编译/验证高层 API | 强制业务 Job/数据库 |
| `mo-video-native` | 已授权媒体句柄、FFmpeg 窄 FFI、GPU 平台互操作 | 产品用户/积分数据 |
| `mo-video-wasm` | 同源语义导出、对象生命周期、计算预算 | PPTX/模板整体依赖 |
| `video-client` | TS 薄绑定、资源/帧句柄、Worker 通信 | 应用 router/store/Electron |
| `video-browser-media` | WebCodecs、Mediabunny、WebGPU/AudioWorklet 平台适配 | 另一套剪辑语义 |
| 产品 Host | 授权、资源、持久任务、版本提交、Artifact | 重复实现内核时间与编辑算法 |

`mo-common` 仅抽取通用标识/精确单位/诊断/预算；字体与文本布局采用现有内核基础。不要让视频依赖全部 `mo-kernel-api`/`mo-wasm` 的演示文稿实现。保留原 PPT API 兼容，视频模块独立可调用，进一步抽取共享代码必须通过原有 PPT 回归。

大图像/PCM/压缩包不在 Rust↔JS 边界重复 JSON 序列化。控制数据用结构化消息；媒体使用可转移句柄、授权 range reader 和有界 sink；原生线程或进程之间由媒体宿主负责生命周期。

## 3. 工程与资源模型

### 3.1 三种独立数据

1. `VideoDocument`：跨端可移植、声明式剪辑决定。
2. `ProjectEnvelope`：宿主持有的 project ID、权威 head、权限、保存位置、操作日志、任务与租约。不是通用视频格式必须依赖的业务系统。
3. `DerivedAssets`：代理、索引、缩略图、波形、分析与渲染缓存，可根据来源和算法版本重建。

工程包拟定 `.movideo`（发行前检查 MIME/扩展名冲突）：ZIP64 容器，包含 `manifest.json`、`document.json`、必要资源清单、可选媒体和字体。包可选择内嵌资源或仅引用外部资源；导出时明确“完整可搬迁”或“需重新关联素材”。不包含凭证、绝对本机路径、临时 Blob URL 或任意执行脚本。

打开校验：版本、字节上限、压缩比/展开预算、路径穿越、重复条目、哈希、循环引用、资源数量和字体许可。未知必须字段拒绝编辑；明确标注的可保留扩展可读入并原样保存，不能把未知必需效果渲染成成功的空效果。

### 3.2 核心对象

| 对象 | 关键字段 | 不变量 |
| --- | --- | --- |
| Document | format, documentId, canvas, frameRate, audioProfile, colorProfile, sequences, resources | 保存格式与引擎版本独立，明确迁移 |
| Sequence | id, tracks, durationPolicy, markers, constraints | 可嵌套但不能循环；显式时长或自动内容时长 |
| Track | id, kind, ordering, locked, muted, solo, clips | 顺序用稳定标识序列；时间位置不取决于数组下标 |
| Clip | id, assetId, streamId, timelineRange, sourceRange, timeMap, effects | 半开范围、稳定身份、来源范围可用 |
| LinkGroup | id, members, linkagePolicy | 音画同步偏移、组选范围显式 |
| Transition | id, leftClipId, rightClipId, interval, effect, handlesPolicy | 不足素材余量时报错或按明确策略定格，不能默认偷减内容 |
| Caption | id, textRuns, cueRange, wordAnchors, styleRef | 改字不自动剪媒体；词源时间映射独立 |
| Analysis | sourceDigest, streamId, algorithmVersion, sourceRanges, confidence | 建立在源媒体时间，剪辑后通过 timeMap 投影 |
| Resource | id, digest, byteLength, mediaDescriptor, provenance | 路径/URL 由宿主 resolver 另行提供 |
| Effect | type, schemaVersion, parameterValues, keyframes, requiredProfile | 内置/签名发行效果，不能嵌入任意代码 |

Track kind 至少包括视频、音频、字幕、图形和调整层。视频轨的基础剪辑片段默认不重叠；覆盖放到另一轨或使用显式 transition；音频允许重叠混音。调整层按明确层级作用于下方选定合成组。画布大小不改变源像素比例；显示矩阵处理旋转与非方形像素。

同一文件可有多个音视频流，`streamId` 必须被操作绑定；不能仅用“文件时长”代表被使用的流时长。后者影响裁切、输出与按秒计费。

### 3.3 时间定义

沿用精确 `RationalTime { ticks: int64 decimal string, timescale: u32 }` 思路，分数帧率单独为 `{ numerator, denominator }`。半开区间 `[start,end)`，空范围、负 duration、溢出和非法比例直接拒绝。

- 运算采用带检查的整数中间值和约分；临时分母必要时使用有 bit budget 的大整数。不要把所有时基的最小公倍数永久放入一个巨大 timescale。
- 现有 timescale 上限 `1e9` 需要保留兼容；无法精确表示的运算结果在显式输出边界按声明 rounding 转换并报告误差，不悄悄四舍五入保存。
- 整个视频不通过反复 `time += roundedFrameDuration` 前进。第 n 个输出视频采样从 `n × denominator / numerator` 直接计算。
- 音频第 k 个输出样本为 `k/sampleRate`；第 n 帧音频样本数用相邻累计样本边界相减，不把一个取整样本数用于所有帧。
- 微秒仅是某些 API 的边界单位；例如 WebCodecs 输入时间由精确累计时间独立转换，误差有界，不把微秒作为内部唯一真相。
- PTS 是呈现时间，DTS 是解码顺序；允许合法负 DTS。源 start offset/edit list、音频 priming/padding、VFR 实际样本间隔都要保留。

本次合成实验：约一小时的 30000/1001 视频若逐帧把时长取整至微秒再累加会偏移 35.964 ms；若每帧固定分配四舍五入后的 48 kHz 音频样本则偏移 899.1 ms。这证明模型应避免该算法，不是对某个参考产品的缺陷判定。[实验与复现](../../research/video-editing-2026-10-06.md#7-本地实验与可复现结果)

### 3.4 时间变换

正常速度映射为 `sourceTime = sourceStart + speed × (timelineTime - clipStart)`。正向映射落在源半开范围内；反向播放按源样本覆盖区间选择最后可呈现样本，不直接请求 `sourceEnd` 的越界点。

`TimeMap` 为分段映射，支持恒速、定格与变速曲线。作者关键帧使用显式源时间锚点和插值模式；各段声明方向，正反方向切换处形成分段边界。编译时验证单调段、来源范围、时长、拼接连续性和允许的不连续跳切。不要把浮点速度积分后的随机累计误差写回工程。

定格段明确固定源样本，音频策略为静音/继续独立音轨/用户选定处理；反向视频是否反向音频必须显式。保调变速与直接升降调是不同模式。光流插帧属于可选质量模块；未安装时只提供明确标记的帧采样/混合模式，不能用“变速”名称承诺光流质量。

曲线的保存真相是 `(timelineTime, sourceTime)` 锚点和有界控制参数，不能同时保存一条可独立变化的速度曲线。速度是映射的导数；速度编辑器将用户输入编译为同一映射，再显示编译后的准确端点与时长。单调三次段需限制斜率，避免插值过冲超出素材。固定点/有理运算和最终采样舍入顺序属于算法版本；源时间反查工程时间时先定位单调段，再在显式误差预算内求逆，定格返回时间范围而非伪造唯一逆值。

对转场，分别在左右片段自己的 TimeMap 上采样，再用转场局部进度合成；不能将一个变速片段的源时间直接作为另一个片段的时间。嵌套序列通过映射组合进入父时间，检查最大嵌套深度、循环和运算预算。

## 4. 语义编辑与事务

### 4.1 操作字典

| 操作 | 精确语义与约束 |
| --- | --- |
| insert | 在指定轨道插入；是否推移后继及哪些关联轨道由 rippleScope 决定 |
| overwrite | 替换目标区间；明确保留两端片段及关联音频策略 |
| split | 在合法时间点切分，生成稳定新 ID，继承效果并转换关键帧/字幕映射 |
| trim | 改变源入出点与工程边界，校验可用素材余量 |
| ripple trim/delete | 改边界后移动指定同步集合后继内容，锁定轨道冲突原子失败 |
| roll | 同时调整相邻片段共享边界，总时长不变，双方 source handles 足够 |
| slip | 保持工程位置和时长，平移片段源范围 |
| slide | 移动选中片段并调整相邻片段边界，保持被约束区域总长 |
| move/group move | 基于稳定对象身份移动，明确吸附候选与音画 link policy |
| retime | 替换 timeMap，显式持续时间/ripple/audio policy |
| set effect/keyframe | typed 参数、单位与插值，不接受任意 JSON 穿透 |
| replace asset | 替换资源引用，显式适配源范围和画幅，保留可成立的剪辑决定 |
| apply transcript edit | 词锚点→源区间→工程区间；重复使用同一素材时指定实例集合 |

磁吸是“候选建议”计算，不是保存格式中的隐式行为。按缩放比例将像素阈值转换为工程时间，候选按“用户指定标记 > 播放头 > 片段边界”的可配置明确优先级及稳定 ID 消歧；相同输入必须得到相同候选。

### 4.2 计算与持久化分离

```text
宿主读取 snapshot/head 与授权资源
  → 内核 prepare(snapshot, transaction, capabilities)
  → 候选快照 + read/write set + inverse + invalidations + diagnostics
  → 宿主在短事务中比较 expected head / authority epoch
  → 原子保存新快照、回执、历史记录、事件 outbox
  → UI 与 Agent 消费同一 committed receipt
```

`prepare` 使用与正式编辑完全相同的 reducer，在 copy-on-write 的暂存状态执行所有操作。失败丢弃候选；不维护“模拟一套、提交另一套”的实现。宿主提交候选时验证输入 head、资源依赖、计算合同版本和候选摘要；prepare 后发生新修改则必须重新准备。

普通独立 SDK 调用可以在调用方内存中应用候选并返回新快照，不强制数据库、租约或业务 Job。Musterwork 集成才使用既有持久化和幂等设施。

以下是拟定产品命令封装示例，非已可调用接口：

```json
{
  "contract": "musteroffice.video.edit/1",
  "projectId": "project-a",
  "operationId": "op-6d1c",
  "expectedRevision": "42",
  "expectedDocumentDigest": "sha256:...",
  "authorityEpoch": "7",
  "operations": [
    {
      "type": "trim",
      "clipId": "clip-b",
      "edge": "end",
      "delta": {"ticks": "-2", "timescale": 1},
      "rippleScope": {"kind": "explicitTracks", "trackIds": ["video-main"]},
      "linkedMediaPolicy": "preserveIndependentAudio"
    }
  ]
}
```

`projectId/authorityEpoch/operationId` 由宿主认证和绑定，内核不把这些字符串当授权。合同还需定义操作规模、诊断上限和取消预算；schema 与实现从同一类型源生成。

回执包括 `operationId, baseRevision, committedRevision, before/afterDigest, createdIds, affectedRanges, resourceDependencies, inverseRef, warnings`。重放同一幂等键且 payload 相同返回原回执；同键异 payload 拒绝。纯计算候选不是“已保存”的回执。

### 4.3 并发、撤销与崩溃

默认采用单工程线性提交、短 CAS。Agent 的长推理不持有工程锁，手动拖动也不占用持久锁；冲突返回最新 revision、冲突对象及可重试信息。

允许自动重定位的范围必须窄：基于对象身份且其读集合未变化的独立属性修改可重新 prepare；波纹编辑、资源替换、字幕对应删片和跨轨组操作默认不自动重基。不能用 last-write-wins 或通用 JSON CRDT 解决不交换的剪辑语义。

撤销采用有条件的逆操作，条件包括目标对象和关系版本。若后续编辑依赖该结果，返回冲突并可创建恢复副本，不回退整个 head 抹掉其他修改。连续拖动只产生一个提交；回放历史重建的是文档，播放头、选区和缩放是独立个人视图状态。

事务落盘前崩溃：head 不变。落盘后回执丢失：按幂等键查回执。事件发送失败：宿主 outbox 重放；前端按事件序号去重和补齐，不依赖当前对话被打开。多标签页只有获得当前写入 epoch 的产品 Host 可提交；原 owner 失联重新选举后旧 epoch 必须被拒绝。

## 5. 媒体输入、定位与派生数据

### 5.1 Probe 和索引

Probe 返回实际流信息、旋转矩阵、像素比例、色彩标记、采样率/声道、流起点和结束范围；文件扩展名不能代替解码检查。生成模型提供的时长不是最终媒体时长证明。

索引至少包含 packet offset/size、DTS/PTS/timebase、随机访问点及 codec 初始化信息。解析和探测在隔离 worker、有界输入下执行；只在需要的区域延迟读取数据。ISO BMFF edit list、解码前导和破损时间戳必须诊断。

### 5.2 Seek 与连续播放

1. 工程时间经片段 `TimeMap` 转为源时间。
2. 从呈现索引选覆盖该时间的样本；不使用 `round(t × avgFrameRate)` 作为 VFR 规则。
3. 定位到满足解码依赖的随机访问点，按 DTS/码流顺序送入，按 PTS 接收/排序输出。
4. 丢弃目标之前的呈现帧。B 帧重排下不能仅按某个输入 packet 的 PTS 超过终点就停止投喂；结束需要解码器完成所需输出与 drain。
5. Seek 改变 generation；旧 generation 的异步帧即使晚到也不能覆盖新画面。退出时仍负责释放其句柄。
6. 拖动可优先显示代理/缓存帧，松手后校准精确帧；预览分辨率变化不修改工程时间。

反向播放以有限 GOP/窗口正向解码后逆序消费，受内存预算控制；不能把整段长片全部解码入内存。缺失资源为明确的 unavailable 状态，编辑时可用占位标识，正式导出必须失败或采用用户显式选择的占位输出策略。

### 5.3 代理、波形与缩略图

代理是原资源的派生物，绑定原 digest、流、时间映射、分辨率、色彩转换和生成算法版本。VFR 转 CFR 代理须记录源时间映射；不能仅复制 duration。源文件变化后代理失效。最终导出默认读原始素材，明确选择代理导出时写入输出质量信息。

缩略图按时间窗口、缩放层级生成，优先可见区域；波形以 min/max + RMS 的多分辨率金字塔存储，不把全量 PCM 传给 UI。场景切分使用可解释的镜头候选和来源区间，闪光/运动可能误判；分析不会自行剪掉内容。

字幕/ASR/人声检测等产物绑定源 digest 和模型版本。用户修订的字词形成独立版本，重新分析不能覆盖人工修订。Agent 可请求缩略联系表、指定区间波形摘要或文本，不默认拿到所有原始帧。

### 5.4 素材理解的算法边界

| 分析 | 基础实现方向 | 不能越过的边界 |
| --- | --- | --- |
| 镜头候选 | 低分辨率亮度/颜色直方图差异，结合边缘变化、局部峰值、双阈值及最短间隔抑制闪光误判 | 输出带分数的候选；渐变、快速运动及闪光另做语料，不把候选自动当剪点 |
| 静音/语音区间 | 分窗 RMS/峰值估计与带迟滞的区间合并；可插拔 VAD 补充语音判断 | 安静不等于无意义；保留词前后余量，剪口播须用明确操作 |
| 按文字剪辑 | ASR 词时间→用户修订层→选定片段实例的 TimeMap→语义编辑 | ASR 重跑不可重置人工取舍；误识别和重叠说话有可见诊断 |
| 语义检索 | 分镜联系表/文本与可选多模态嵌入；索引绑定来源、区间和模型版本 | 查到摘要不意味着已获原素材上传权限；结果可回到原片核查 |
| 自动构图/配乐建议 | 检测结果形成可检查的裁剪关键帧或增益包络 | 不逐帧在线调用模型；不将随机分析放入确定性渲染图 |

算法模块可升级，剪辑操作与已接受关键帧仍按原合同保存。质量语料记录准确率/召回率、误剪和用户修正，不用“模型返回成功”作为内容质量证据。

## 6. 合成图、缓存与调度

### 6.1 编译计划

不可变工程编译为带版本的 DAG：源样本→时间映射→画幅/颜色转换→片段效果→轨道合成/转场→文字/图形→输出变换。音频有独立图，按共同时间基准汇合。

图节点声明输入、输出格式、资源预算、时间邻域和状态需求。模糊的空间邻域、转场的前后片段、时伸的前后缓冲都进入依赖分析。普通编辑不需要重建全部图；受影响对象与时间范围传播到后继，纯粹 UI 选区变化不触发渲染失效。

时间索引以每轨有序区间索引为基础，查询目标为 `O(log n + k)`；k 为相交对象数。片段移动更新局部索引。图与操作复杂度有基准测试，不仅标注理论阶数。

对不重叠主视频轨，按起点有序查找并验证前驱即可；音频、叠加与跨轨范围采用带 `maxEnd` 的平衡区间树或等价的持久索引。播放头递增时用事件边界推进 active set，随机 seek 重新查找。波纹编辑实际影响 m 个对象时至少产生 O(m) 的必要更新，不能宣传所有编辑都为 O(log n)。连续拖动复用基线结构与增量候选，松手才生成完整可保存操作。

### 6.2 缓存身份

缓存键绑定：节点语义版本、参数 canonical digest、输入资源 digest/流、时间或样本区间、输出尺寸、色彩/像素格式、字体资源、效果算法版本、质量档案及必要的设备实现标识。

跨工程可共享无权限含义的内容身份，但返回字节前仍由宿主校验当前访问权。CPU/GPU/磁盘分别设预算和淘汰策略；内核只持有有界运行缓存，磁盘缓存、清理和配额由宿主提供。缓存写入以临时对象完成后再发布，读者不能见到半张帧。

素材回收与派生缓存淘汰分开：当前 head、保留历史、未完成导出及尚未提交提案引用的原资源具有宿主 pin；不能因 UI 删除片段就删除仍被导出使用的文件。先标记可回收，再经宽限期核对当前引用；释放历史的产品策略必须明确。媒体对象固化与元数据提交之间留下的孤儿可清理，已提交快照引用的对象不可被当成孤儿。

优先级：音频实时补充→当前播放帧→用户 seek→可见缩略图→邻域预热→后台导出。导出有公平调度，不能无限饥饿；内存紧张优先回收派生数据，不回收未保存工程。

### 6.3 背压与资源释放

每一阶段暴露可提交容量：reader→decoder→frame pool→compositor→encoder→muxer→sink。下游慢时停止上游，不能无限创建 Promise、VideoFrame 或 AVFrame。媒体队列按字节预算及设备最小重排需求共同确定，不能把“最多 4 帧”硬编码给所有 codec。

帧采用引用计数/拥有权句柄；缓存、GPU 在途工作、编码器消费分别有明确释放点。WebCodecs 输出用完显式 `close()`；异常、取消、迭代器提前结束同样清理。GPU 设备丢失时撤销在途预览，重建资源并恢复已提交版本；导出按任务检查点重试，不宣布部分文件成功。

### 6.4 图形后端和拷贝预算

Rust 定义 RenderPlan 与效果数学。原生 wgpu 使用平台图形能力；浏览器 WebGPU 适配可由 TS 直接管理外部帧纹理、执行同源 shader/计划。不要为了“全 Rust”把所有 VideoFrame 复制进 WASM 内存。

WebGPU external texture 可能避免复制，但标准不保证；原生硬件解码帧与 wgpu 之间的纹理共享也需要专门平台实现。因此记录每条路径的 CPU 拷贝次数、GPU upload/download、同步等待和内存峰值；“硬件加速”不能代替这些指标。[WebGPU 规范](https://www.w3.org/TR/webgpu/#gpuexternaltexture)

CPU 参考后端用于正确性和有限能力运行，不以 CPU 低性能静默满足已经承诺的实时档案。高级效果缺失时保存工程、报告 capability mismatch；不把它变成透明空操作。

## 7. 色彩、文字和音频

### 7.1 色彩与画面

必须携带 primaries、transfer、matrix、range、alpha 方式、像素比例和旋转信息。视频 YUV、浏览器已转换 RGB、图片 ICC/sRGB 是不同输入路径；平台已做过颜色转换时不能再次套用 YUV 矩阵。

基础交付档案为 SDR，内部工作空间明确为线性 RGB、预乘 alpha；合成完成后转换为目标 transfer/range。高精度 GPU 中间目标按能力使用浮点格式，基础输出 H.264/AAC MP4 或受支持的 WebM。HDR/广色域原素材必须识别；SDR 输出显式 tone map，不隐式当普通 SDR 丢弃元数据。HDR 成片属于单独的完整质量档案，未验收时不在导出中显示支持。

基础效果包括变换/裁剪、颜色调整、模糊、遮罩、简单混合、淡化和方向转场。关键帧的 step/linear/cubic 插值有版本化定义；角度是否走最短路径、颜色在哪个空间插值都不能随前端库默认值改变。

### 7.2 文字

复用固定字体资源、HarfBuzz/现有文字塑形与布局能力。字体 fallback 必须显式列出并可打包，不能依赖宿主系统碰巧存在某字体。字幕统一行距、基线、描边/阴影、emoji、双向文字及换行规则；浏览器 DOM 只承载编辑控件，不能决定最终成片排版。

编辑光标和点击命中通过同源布局输出坐标；字体缺失时明确替代及变化，正式导出不得悄悄换字形。模板字体和贴纸素材同样进入资源/许可清单。

### 7.3 音频

工程以固定采样率（基础 48 kHz）调度，内部 float PCM；源采样率和声道通过声明映射转换。混音、自动化与导出共用样本区间算法。

- 每段 source time→sample index 的映射有明确 rounding，累计误差受限；滤波器/重采样器维持相位状态，不能每个视频帧重新初始化。
- 音量用线性增益计算，UI dB 转换明确；相同素材复制后并行播放会增加响度，不能默认归一化掩盖叠加结果。
- 配乐压低支持手工关键帧或分析生成 envelope；分析结果可检查和撤销。
- 保调变速需补偿输入/输出延迟、首尾 pre-roll/padding 和跨段连续状态。Signalsmith 的公开接口明确有两侧延迟，不能把输出第一个样本直接当源 t=0。[原作者接口](https://signalsmith-audio.co.uk/code/stretch/)
- 交叉淡化提供线性与等功率模式；自动化边界避免爆音。降噪、人声分离作为显式分析/处理模块，不属于基础混音隐含行为。
- 预览由音频设备实际播放时钟驱动视频，设备切换/暂停后重建 clock anchor；无音频时使用单调时钟。浏览器 AudioWorklet 消费有界环形缓冲，缺数据要报告 underrun，不能靠频繁定时器伪造同步。
- 离线导出使用确定的样本计数，不依赖墙钟。响度、true peak、clipping 给出检查结果；广播 -23 LUFS 是 EBU R128 的具体目标，不能当成所有短视频平台统一要求。[EBU R128](https://tech.ebu.ch/publications/r128)

## 8. 导出、校验与续作

```text
创建导出意图（固定 revision/resources/profile）
 → 探测目标能力与磁盘预算
 → 编译视频/音频计划
 → 有界解码、合成、编码
 → mux / sink 收尾
 → probe + 抽样解码 + 时间/样本/流校验
 → 宿主原子发布 Artifact
```

输出策略：目标格式真实支持时优先硬件编码，质量/码率/像素格式受明确档案约束；硬件失败是否切换软件需要同档案能力成立且任务可安全重启。不能把失败后降分辨率当成功恢复。没有完全无效果且切点满足条件的证据时，不启用 stream-copy；精确裁切涉及 GOP 依赖，不能默认无损复制任意区间。

每次导出固定工程、全部资源、字体、算法、codec 配置、颜色和输出目的地。状态至少为 `queued / preparing / rendering / finalizing / verifying / completed / cancelRequested / cancelled / failed / interrupted`；后台执行器的租约和 epoch 由产品宿主持有。

恢复策略：

- 编码器内部状态一般不能直接持久化；默认重新计算未完成单元。
- 支持续作的档案以独立可解码片段/checkpoint 为单位，记录依赖摘要、时间区间、音频 pre-roll、效果 warm-up 和编码参数；段前预热、段后裁切确保与整段语义一致。
- 有状态效果如果没有可验证的检查点协议，从最近安全点或任务起点重算；绝不能任意拼接 AAC/视频字节。
- 最终 mux 验证时间戳连续性、音频 padding、关键帧边界与总时长，再原子发布；部分结果不替换正式文件。
- 保存检查点与 Artifact 提交分开；应用崩溃后可以发现已验证但未发布结果，通过幂等提交恢复。

下载大文件使用流式写出或授权文件句柄；只在有明确大小上限时才收集为 Blob。浏览器内存、OPFS 空间、文件授权或 codec 不足时给出可行动诊断。

## 9. Agent 合同

### 9.1 工具集合

命名为示意，最终必须进入现有工具合同发布，不建立私有旁路。

| 工具 | 输入/输出 | 侧重 |
| --- | --- | --- |
| `video.capabilities` | 设备/组件档案→可用操作、codec、效果、限制 | 能力发现，不靠技能文本猜测 |
| `video.inspect` | 工程版本、范围、分页→片段/轨道/资源摘要 | 模型上下文有界，支持定位稳定对象 |
| `video.create` | 工程设置、授权资源→初始快照/回执 | 不修改原素材 |
| `video.prepare_edit` | 版本与操作批次→候选、差异、诊断 | 可解释地显示影响，无额外用户审批要求 |
| `video.apply_edit` | 同一候选/前置条件→正式回执 | 宿主 CAS、幂等、历史 |
| `video.preview` | 版本、时间或区间、质量→图像/短预览资源引用 | 准确绑定编辑版本 |
| `video.validate` | 版本与输出档案→缺失资源、越界、音频/字幕问题 | 结构检查与视觉判断分别报告 |
| `video.export` | 固定版本、输出档案→宿主任务/最终文件引用 | 长任务、取消、恢复 |
| `video.history` | 工程、操作 ID→历史摘要/有条件撤销 | 不修改其他人的后继成果 |
| `media.analyze` | 授权素材、分析种类/范围→版本化分析产物 | 允许本地或明确授权远端计算 |

生成图片、生成视频、TTS/ASR 使用产品已有/新增独立工具；视频计算核心不认识某家模型 API key。工具装载取“视频创作技能声明 ∩ 用户授权 ∩ 当前设备能力 ∩ 已发布服务能力”，不能只注册 schema 却没有执行器。

### 9.2 Agent 工作流

读取当前工程/选中范围→检查素材与设备→按需生成/分析→构造有明确策略的操作批次→内核检查并提交→请求预览与质量检查→根据结果继续编辑→导出与交付。

“镜头二”需要先解析为稳定 clip ID；多个同名片段或已变更版本返回歧义信息。智能建议输出结构化 proposal，置信度低的场景边界不能默认成为删除命令。工具错误分 `invalid_argument / missing_resource / capability_mismatch / stale_revision / permission_denied / budget_exceeded / cancelled / media_corrupt / device_lost / storage_exhausted`，附路径和修复建议；内部堆栈留诊断日志。

模型不得接收全部大文件、密钥、任意本地路径、裸 SQL 或 shell 滤镜字符串。外部 MCP 使用相同语义与宿主资源桥接，普通 SDK 在调用方环境直接执行，无强制账号/服务器依赖。

## 10. Device、Browser 和 Cloud 的执行

### 10.1 桌面

既有 Rust Device Runtime 持有任务、授权、资源和工程 head；隔离的 Office Video Worker 执行媒体计算。Electron/Vue UI 只通过产品端口操作，不另起 JS Agent Loop，也不自行保存另一份权威 scene。

资源授权绑定当前用户/工作区/工程与调用；原生进程只能获得本次授权句柄。worker 版本、组件哈希、能力档案与任务记录绑定，软件升级后旧任务恢复必须验证合同与输出兼容。

### 10.2 浏览器

产品 Browser Project Host 持有本地工程和任务；WASM 负责编辑/编译，Media Worker 负责编解码/渲染，AudioWorklet 负责音频输出。大媒体优先 OPFS/用户文件句柄，IndexedDB 存元数据/head/回执；配额、持久化申请与备份交互由产品拥有。[OPFS](https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system)

跨存储原子性不能假设存在：先将媒体写入不可变内容对象并 flush/校验，再在 IndexedDB 原子提交 manifest/head/receipt；崩溃留下的孤儿对象延迟回收。用户主动清理站点数据可能清掉工程与素材，需要工程包导出/备份入口，不能承诺浏览器存储永久可靠。

多标签页优先用可用的 Web Locks 协调同工程写入者，最终裁决仍是 IndexedDB 同一读写事务中的 epoch/head 比较；广播只负责通知，不负责一致性。可用性检测失败时采用同一持久事务内的租约/epoch 选举，过期回调提交必须失败。任一受支持浏览器都需真实验证保存原子性，不能把一个全局 JavaScript 变量当跨窗口锁。[Web Locks](https://developer.mozilla.org/en-US/docs/Web/API/Web_Locks_API)

原子性和持久性分别验证：正式 head/receipt 事务请求 `durability: "strict"`，缓存可以采用 relaxed。`complete` 事件、浏览器 API 的存在或默认 durability 都不能单独证明断电后数据已保存；实际平台若无法达到该持久化档案，应公开能力限制并提供工程文件备份，不以“已永久保存”误导用户。宿主数据库的 fsync/日志策略同样进入恢复测试。[IndexedDB durability](https://developer.mozilla.org/en-US/docs/Web/API/IDBDatabase/transaction)

普通 Worker 与 transferable 作为必需路径；若启用 SharedArrayBuffer/WASM 线程，部署需满足跨源隔离并验证现有登录、附件、第三方资源响应头。线程只是已声明性能档案的加速能力，缺失时不悄悄放宽实时承诺。

### 10.3 云端 Agent 调用本地执行器

沿产品已有的受授权任务通道新增 typed video capability。请求绑定 `principal/workspace/project/revision/executor/session/authorityEpoch/operationId/expiry`；浏览器/设备验证作用域、当前资源权和能力，返回签名或受认证通道内的结果回执。不得开放“任意 localhost HTTP + projectId”即能编辑的旁路。

云端只持有任务控制状态与必要摘要，本地权威持有工程 head。失联→任务 waiting_for_executor/interrupted；重新连线核对 epoch 和幂等回执。收到超时不能自动换设备执行同一个编辑，更不能把本地资源同步到服务器以绕过断连。

通用协议重放语义统一，但权限和网络传输由接入产品实现。独立 SDK/MCP 用户无需实现 Musterwork 的这些业务协议。

## 11. 兼容、迁移与能力档案

工程 schema、操作 contract、效果语义、组件 release、codec build、资源闭包分别版本化。文档迁移是确定性、可诊断且保留原件的转换；向旧版本导出必须检查所有必需能力。

基础交付的目标平台包括 macOS arm64、Windows x64、Linux x64 原生，桌面 Chrome/Edge/Safari/Firefox 浏览器。具体 OS/浏览器版本在验收时固定，不因页面存在 WebCodecs 就承诺完整功能；每个平台必须至少通过一个明确的常用视频/音频导出组合。其余容器/编码按能力清单公开。

OTIO 交换覆盖轨道、剪切、间隙、基础转场、来源范围和可映射标记；特效、调色、字幕、嵌套/变速的支持逐项声明。FCPXML 以选定 DTD 子集实现并对照真实应用。遇到无法表达的效果，提供保留工程原件、导出损失报告、用户选择烘焙片段三种明确结果；不声称无损。

PPT→视频以只读固定版本的演示播放计划作为媒体源，时间与字体资源全部绑定；视频→PPT 以媒体引用和封面接入。两个领域通过公共计算/资源合同互通，不能互相写内部文档模型。

## 12. 安全、许可与可观测性

输入解析、字体和媒体解码设置 CPU/内存/尺寸/时长/嵌套深度预算；原生非内存安全依赖置于可终止 worker。输入工程不允许插件脚本/任意网络请求。远程素材由产品授权下载、验证重定向及大小后交付，避免让媒体库自行访问任意 URL。

原创内核继续 Apache-2.0。研究 GPL 源码的设计不等于可以复制进 Apache 原创模块；具体代码复用按来源许可处理。FFmpeg 精选构建默认禁止 `enable-gpl/enable-nonfree`，保留实际 configure、源码/修改及发行义务；不默认携带 x264。Mediabunny 的本次快照为 MPL-2.0，必须在模块清单中保留，不能笼统标 MIT。[FFmpeg 官方许可](https://ffmpeg.org/legal.html)

可观测性记录 operation/job correlation、阶段耗时、队列深度、copy bytes、缓存命中、内存/GPU 峰值、取消延迟及错误码；不记录原素材内容、用户全文或密钥。对用户只显示完成任务所需的信息，详细诊断通过受控入口导出。

设计的实现与测试追踪见[验收规格](acceptance.md)。
