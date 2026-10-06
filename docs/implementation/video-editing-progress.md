# 视频创作与剪辑实施记录

2026-10-06 · 目标执行中，尚未达到整体验收。

## 固定基线与开发边界

- 已发布 [MusterOffice v0.1.0](https://github.com/wanquanY/MusterOffice/releases/tag/v0.1.0)，固定源码 `a4901290c6ccf6660aabe38d42d2dd650428219f`；对应 CI 44 项步骤通过，发布前源码摘要一致，三个公开附件的远端 SHA-256 与本地一致。这是稳定源码基线，未提升产品组件频道或声明全平台预编译包。
- 用户已明确授权稳定版发布和完整视频方案实施。MusterOffice 主工作区功能分支 `codex/video-native-workbench`；产品保留现有独立 worktree。生产部署、Musterwork 公开发布不属于本目标。
- Musterwork 原 worktree 仍有可复用的 Admin、生成协议/计费和多模态请求绑定；已保存原 HEAD、工作区/暂存区二进制补丁和 42 个未跟踪文件的完整本地快照。保留该 worktree，按公共操作合同替换旧 CE.SDK 路径，不整库丢弃有效代码。
- 当前开发决定见 [ADR 0012](../decisions/0012-agent-video-domain.md)。纯内核候选不代表宿主保存成功，测试辅助绑定不作为产品 SDK 发布。

## 权威方案与工作包

[产品与架构](../design/video-editing.md) · [技术合同](../design/video-editing/technical-spec.md) · [完整验收](../design/video-editing/acceptance.md)。完整目标不缩减；本文件只记实际进展。

| 工作包 | 当前状态 | 退出条件 |
| --- | --- | --- |
| W1 视频领域与合同 | 基础已实现，未全部完成 | 独立模型、精确时间、受限解析与校验、Native/WASM 公共合同向量；补全效果、字幕、分析和工程包模型 |
| W2 媒体与合成 | 原生解码/采样、CPU/Metal 合成及音频块处理基础已实现；保调工程图及派生 PCM 已实现；Browser、导出及产品显示待实施 | 原生/浏览器媒体与合成、字体/色彩/音频及真实输出 |
| W3 编辑与历史 | 原子事务基础已实现，未全部完成 | 全部编辑操作、候选、冲突与有条件逆操作及增量索引 |
| W4 产品执行 | 待实施新内核接入 | Device/Browser Host、恢复、权限、任务和不可变组件消费 |
| W5 创作与交互 | 保留原 worktree，未完成替换 | 整理现有生成/Admin/计费、技能、人机共享编辑与消息卡片 |
| W6 完整验收 | 仅有下列内核基础证据 | 完整验收矩阵与可供用户实际操作的本地环境 |

## 已实现的计算基础

### 视频高层接口与独立 WASM

`mo-video-operations` 已实现不可变工程创建/打开、摘要、分页、候选编辑、条件撤销、帧计划与资源闭包；`mo-video-wasm` 和薄 TS 客户端共享同一合同，独立于 PPT 和媒体 codec。宿主 revision、epoch、幂等持久提交仍由产品负责，候选不等于保存成功。已有不可变开发包构建和实际 Native/WASM/打包 ESM 检查；尚未发行生产 SDK 或接入工作台。详见[接口及实际边界](video-authoring-api.md)。

### 精确时间

`mo-time` 从原 PPT 时间线抽取有界精确有理数运算。PPT 保留原合同与正除数约束的薄适配，视频可用反向映射所需的有符号除法。时间转换区分精确可表达与显式量化；不能表达时返回错误，量化返回舍入残差。支持四种明确舍入策略、负时间和分数帧率的绝对帧边界计算。

### 工程模型

`mo-video-model` 已有独立工程、序列、视频/音频/图形/字幕轨、片段、媒体/字体资源、流选择、嵌套序列、联动组，以及线性分段/倒放/定格时间映射。基础效果包括恒定变换、增益和显式相位范围的淡入淡出。剪开片段时平移效果相位，不重新开始淡入淡出。

校验覆盖身份与所有权、丢失引用、流/轨兼容性、半开区间重叠、源素材范围、时间映射、关联组、嵌套 DAG 深度和循环。JSON 拒绝未知字段及重复键，具有字节/节点/流/锚点/诊断/算术预算与取消检查。资源字节、URL、绝对路径和业务任务不进入模型。

顶层对象表使用有序持久树共享快照。10 万对象的计数测试证明：复制集合不复制记录，修改一个记录只复制该记录；这不是完整工程交互性能或内存峰值的验收。组件版本和许可见[固定来源记录](../../components/persistent-collections/component.json)。

### 原子编辑与逆操作

`mo-video-edit` 提供快照、`prepare` 和 `prepare_inverse`，实现标题、序列/轨道/资源管理、插入、删除、移动、分割、裁切、波纹插入/裁切、slip、roll、slide、基础效果/来源替换、关联/解除关联，以及覆盖、区间删除和显式音频策略的变速。波纹轨道集合、关联策略与分割后的新身份必须显式给出。区间/变速规则见[详细合同](video-editing-transactions.md)。

所有操作先在共享结构的候选中执行，完成后统一验证、计算摘要、读写集合、条件逆操作和预览失效范围。错误或取消不改变基线。逆操作按对象及关联依赖检查冲突，允许保留其他轨道的独立修改；新建关联、解除关联后成员被修改等情况不能被撤销静默覆盖。嵌套序列依赖和 solo 导致的其他轨道变化进入失效传播。

持久事务、幂等、版本号/epoch、outbox 和存储仍由产品实现。当前完整校验、摘要与失效传播是工程规模线性工作；不可把局部持久树更新的复杂度当成整个事务的性能。

### 工程编译与采样

`mo-video-compile` 已实现验证后编译、平衡区间索引、绝对帧/音频样本时钟、分组视觉计划和分层音频点查询。精确采样携带 before/after 侧；倒放与多级嵌套翻转该侧，不使用 epsilon，也不将中间有理数强制量化到作者时基。嵌套实例按视觉/音频轨道明确投影，透明度保留在分组，逐级音频保调/反向/定格策略不被扁平化。

VFR 呈现窗口按真实样本覆盖区间选择源帧，区分已知 gap 与尚未索引的 outsideWindow，验证内容摘要和流绑定。该纯计算索引只做元数据运算；demux/seek 由下述媒体组件承担；音频块处理的后续实现见下文，非原速保调的连续工程图见下文。接口、预算、边界及下一层责任见[编译实施记录](video-editing-projection.md)。

### 原生媒体叶子组件

已独立构建固定 FFmpeg 9.0.2 的 LGPL 动态库和原创受限 C ABI。源码 archive 摘要与官方发布密钥签名已核对；固定关闭 GPL/nonfree/version3、网络、协议和自动依赖探测。媒体输入只经宿主 read_at/取消回调，嵌入外部资源打开被拒绝，接口不接收路径或 URL。

已实际解码 H.264 B 帧/VFR 和 AAC，提供显式 stream selection、send/receive/drain、seek generation、原始平面复制、色彩/时间戳/声道元数据及预算/失败状态。没有默认系统 FFmpeg 依赖，没有把 C 原始帧转换成伪造的播放器输出。源码、许可、精确边界与复现见[媒体组件记录](../../components/ffmpeg/README.md)。

后续已实现 `mo-media-ffmpeg` 的 Rust RAII、回调 panic/错误边界、类型约束的单次流选择、线程归属与独立拥有的帧平面。构建时逐字段核对 C/Rust ABI，核实实际加载别名指向的固定组件；任何操作错误关闭 native context，取消/失败不返回部分帧。[执行边界记录](video-media-native.md)包含具体生命周期、panic 语义与复现。

可终止 worker 已完成：`mo-media` 分离无 codec 链接的数据，`mo-media-wire` 提供有界头与原始平面协议，`mo-native-media` 持有宿主资源和进程。总 deadline 不随读往返重置，错误/取消/崩溃会停止并回收进程；开发包校验固定摘要和 loader 闭包。详见[worker 实施及实际证据](video-media-worker.md)。连续区间的真实帧已接通工程采样计划；产品 GPU 显示、浏览器和编码导出仍未接入，不能把 worker 测试成功当成产品已经可用。

`mo-native-limits` 已提供后续 worker 启动使用的进程资源限制，实际验证 CPU 超限终止、地址增长拒绝、文件增长失败和继承限制。该模块已在视频 worker 接收素材前安装并读回核验；地址增长额度不等于 RSS 硬限额，也不是文件/网络沙箱。[平台边界与测试](native-worker-limits.md)明确保留这些区别。

### 真实源帧采样

`mo-video-source` 默认无 native 依赖，按精确源时间和 before/after 侧选择实际帧，使用有界缓存及 lookahead；可选 native adapter 复用既有 worker。真实素材暴露出 decoder duration 与相邻 PTS 重叠的问题，当前以宿主有证据的连续区间为前提，以相邻 PTS 定义呈现覆盖，保留原始元数据。seek/倒放不按平均帧号寻址，过冲先重读而不伪造 gap；取消/失败丢弃缓存并释放 decoder。[详细语义与局限](video-source-presentation.md)。

### 色彩与参考合成

`mo-color::rgb` 和 `mo-video-render` 已接通原始平面解释、明确显示 transfer、BT.2020/D65 线性预乘合成、SAR/显示矩阵、作者变换和组透明度。真实 worker 帧经过工程采样输出合成 PNG；CPU 原生/WASM 小向量包含 f32 bits 的完全一致验证。未知色彩解释、HDR 缺 tone-map、缺资源、预算及取消不会返回伪造成功。当前只提供参考过滤和简单显式 tone-map；文字与最终效果/导出仍待实现，GPU 后续进展见下一节。详见[色彩与合成记录](video-color-composition.md)。

### GPU 合成

`mo-video-gpu` 已通过共用 plan executor 接入真实 Metal GPU。CPU/GPU 使用一致的定点栅格坐标决策，修复浮点矩阵在最近邻边界选不同像素的问题；WGSL 执行同一分组/透明度语义。29 项硬件对照包含 1080p、最大宽度、HDR 线性值和四帧真实素材，并验证取消、预算、callback panic 后的恢复。当前返回未提交的 GPU 候选，仍需宿主显示、时钟和设备丢失处理；不代表实时工作台已可用。详见[GPU 实施记录](video-gpu-composition.md)。

### 音频块处理

`mo-video-audio` 已接通 PCM 归一化、命名声道/显式矩阵、有理采样时钟、带限重采样、倒放、增益/淡入淡出和嵌套混音。分块编译按片段和时间映射锚点调度，保持各层音高政策；窗口缺失不会伪装静音。真实 worker AAC 输出经五类实际 DSP，并与独立 CLI 和数学包络核对；Native/WASM 的完整 PCM bits 一致。工程保调图已接通；尚缺生产 Host 存储和声音设备；详见[音频块处理记录](video-audio-dsp.md)。

### 标准 PCM 与本地随机读取

`mo-video-pcm` 已实现连续解码 PCM 的内容身份、不可变分块候选、校验和、受限 LRU 窗口读取和 `CanonicalAudioResource`。真实 AAC 定位实验发现，通用 seek/flush 无法代表位精确的完整解码状态；本地派生块让定位/倒放和导出复用相同 PCM。两个真实素材的 264 次随机/反向读取以及完整倒放混音逐 bit 一致；Native/WASM 完整/部分物化 14 查询一致。生产本地/OPFS 存储与后台准备仍由产品 Host 接入，首次物化及冷定位性能尚未达成验收。详见[连续解码与分块读取](video-canonical-pcm.md)。

### 保调顺序计算

`StretchStream` 已实现固定 128 帧量子、整数输入时钟、两侧延迟补偿、有限源尾部和可重试缺源。已用固定 Signalsmith Stretch/Linear 接入 Native 与 WASM，12 组实际跨端 PCM 输出一致，并验证短片段、音高、重试、取消和分配失败。工程混音器已通过连续派生图调用该能力，并保留裁切/分割上下文；不随窗口查询重建 DSP。详见[保调实施记录](video-pitch-stretch.md)。

### 工程保调图和上下文

`AudioMixer` 已通过 `RetimeRequest`、`RetimeMaterializer` 与 `CanonicalRetimedAudio` 使用连续派生 PCM。分割和裁切保存 `audioContext`，拉长恢复隐藏速率锚点；同向速度段共享 DSP，反向/停帧分开。音频依赖摘要区分名称/视觉变化与上游声音变化，保留每层音高政策。真实 Native 集成和 Native/WASM 完整图比较见[工程保调记录](video-audio-retime.md)。生产存储/后台调度/设备播放仍待接入。

### 原生编码、封装与候选输出

`mo-media-ffmpeg::Encoder` 已实际导出 H.264 VideoToolbox/AAC MP4，另提供 FFV1/float PCM MP4 无损审计档案。精确帧/样本时钟、短尾、有限 packet 交错、短写和失败关闭已实现；硬件帧重排问题在固定上游组件中按 SPS 上界修正。12 组真实文件经独立工具核验，ASan/UBSan 和既有媒体 9 步回归通过。详见[编码实施记录](video-native-encoding.md)。工程合成到 I420 和有限导出会话见下一节；编码 worker 见下文，宿主校验发布和产品导出仍待连接。

### 工程合成到编码输出

`mo-video-export` 已将不可变工程、精确 CFR/音频样本时钟、CPU 合成、I420 输出和音频 mixer 接到编码接口。缺音频派生资源可以等待并从当前进度继续；错误和取消关闭候选。6 个真实 MP4、96 帧和 185,408 个 PCM 值的 Native/WASM 位级对照通过。详见[工程导出记录](video-project-export.md)。当前测试图形/程序 PCM 不代表真实素材全流程或产品验收；编码 worker 见下一节，宿主提交仍待接通。

### 编码进程与候选写回

`mo-native-media::NativeEncoder` 已实现 `VideoEncoder`，复用固定组件包、进程额度和既有 Session。私有协议升级 `/2`，有界 I420/PCM 输入及候选 WriteAt 写回、总 deadline、关闭阶段禁止写入、完成计数校验及 kill/reap 已实现。真实工程经受控进程导出并回读，故障路径与既有解码回归已验证；详见[编码 worker 记录](video-encoding-worker.md)。宿主持久化/校验发布和用户 UI 仍待接入。

### 合同与验证入口

`contracts/generated/video-*.schema.json` 从相同 Rust 类型生成，并生成对应的草案 TypeScript 类型，包含工程、编辑请求、逆操作、校验结果、视频帧/音频点计划/音频块计划、标准 PCM 块引用和呈现窗口。它们是开发中的合同，还未独立发行或接入真实产品。

`tools/mo-video-conformance` 是专用测试驱动。相同 Rust 代码编为 Native 和 WASM，比较完整候选文档、摘要、错误、读写集合、失效范围和逆操作；不是另写一套 TypeScript 语义。默认 CI 已加入长序列测试和视频跨端一致性步骤。

## 当前验证证据

| 检查 | 实际结果 | 不能推断的结论 |
| --- | --- | --- |
| 原 PPT 时间线 | 168 项既有测试通过共享算术适配 | 不等于新的媒体渲染已实现 |
| Rust 工作区回归 | 一次完整运行 1,675 passed、28 ignored；后续视频限定修正另跑专项 | ignored 中需要独立环境/worker 的检查不伪装通过；不是所有 CI 分组重跑 |
| 最新时间/模型/事务/编译专项 | 78 passed、1 个长序列测试默认 ignored 并再次单独执行通过；同次另跑 PPT 时间线 168 passed，总计 246 passed | 不代表所有最终编辑功能已实现 |
| 既有 MCP 纯计算入口 | 17 项测试通过，独立 Cargo.lock 已纳入共享 mo-time | 未增加视频 MCP 工具，legacy-host/HTTP 组合本次未全部重跑 |
| 精确时间生成测试 | 4,096 组有理算术性质；107,892 帧的 30000/1001 时间及 48k 音频累计边界 | 这是样本调度数学，未播放或编码真实一小时视频 |
| 事务长序列 | 固定种子 100,000 次；47,422 成功、52,578 拒绝、47,422 次逆操作精确恢复（已扩展区间与变速语料；本轮时间采样改动后重跑，计数保持一致） | 语料覆盖当前操作子集，不足以关闭完整 SEM-06 |
| 原子失败/取消 | 每个观察到的取消检查点拒绝候选，基线不变；中途失败、锁轨、缺素材、超预算验证通过 | 未检验数据库断电和业务 outbox |
| Native/WASM 实际运行 | macOS arm64 Native 与 Node v23.5.0 的 WASM，25 步：20 次成功、5 次拒绝，结果完全一致；重复键两端均拒绝 | 不等于浏览器媒体、WebGPU/WebCodecs、Windows/Linux 原生或产品验收 |
| 跨端 fixture 摘要 | `568babf4de9b90e46fca80bee7377ef437167f9ae1053254355c736fd008be5a` | 绑定本次基础语料，不是最终媒体档案 |
| 跨端结果摘要 | `3bc203a14d17be214aec45b5c9588f221296ab9b6229f28c8518adb5b126195a` | 同上 |
| 采样编译实际跨端运行 | macOS arm64 Native 与 Node v23.5.0 WASM；15 查询、11 成功/4 拒绝，完整输出与统计一致；分组/字体绑定/图像/音频策略/VFR 结果另有预期断言 | 使用合成元数据，不能代表真实解码、GPU、声音质量和产品环境 |
| 编译 fixture / 结果摘要 | `2020028ae82f19c49e65c5e35958f8794a3393a7be7b216c11d1e4a480b78360` / `5b691d973f12abe93476db5250fa24033e979b0b2c2960ca3648ab0e853bb986` | 对应上述 15 查询语料 |
| 区间索引 | 2,048 组随机查询与独立扫描一致；10 万稀疏区间的四处查询均少于 64 节点访问 | 不是全工程编译、播放帧率或 UI 延迟基准 |
| 原生真实媒体 | macOS arm64 固定 FFmpeg 9.0.2；77 帧 B-frame/VFR 的时间/像素 MD5 与独立 CLI 解码一致；前后 seek、13 字节短读、EOF、取消与预算通过 | 仅这一实际素材/平台，不是浏览器、硬件、多 codec/多平台矩阵 |
| 原生 AAC | 已知 3 秒/48k 源返回 144,000 样本且时间连续；与参考 PCM 最大差 `2.24e-8`、RMS `3.32e-9` | 已明确处理参考旧版 CLI 多出的尾 padding；不是完整混音/保调/音质测试 |
| Rust 原生所有权 | 8 项行为测试与 3 项 compile-fail 通过；真实 WAV 逐字节往返、资源 drop、C 内回调 panic、逐检查点取消、独立拥有帧和预算 | 包装不能抢占阻塞回调或限制全部 vendor 私有分配；worker 证据单独记录 |
| Rust 真实媒体 | 显式执行 1 项真实素材测试并独立核验：77 帧像素 SHA-256/PTS、前后 seek 及 BT.709 标签一致；144,000 AAC 样本与参考误差同上 | 不是 renderer/浏览器/产品执行，原生 adapter 只在本机 macOS arm64 验证 |
| 进程限制基础 | 6 项父测试实际启动隔离子进程，验证地址增长拒绝、内核 CPU 终止、继承限制、非法参数、文件描述符与文件增长受限 | 已与视频 worker 连接；不是 RSS、系统调用沙箱或完整产品隔离验收 |
| 原生媒体 worker | 默认协议/宿主 8 项，真实 worker 6 项、包完整性 3 项、真实压缩媒体 1 项均通过；77 帧/144,000 音频样本与参考一致，1.5 MiB 大媒体包经短读成功；宿主无 codec 链接 | macOS arm64 开发包；无完整系统调用沙箱、浏览器或用户时间线验收 |
| 真实工程源采样 | 10 项时间/行为测试；90 正放 + 90 倒放 + 8 随机定位均与独立像素/PTS/覆盖区间一致，实际使用 Rust 工程和 worker；缓存最高 221,184 active bytes | 仅已知连续区间语料，无通用 edit list 解析、GPU、音频 DSP、浏览器或产品播放 |
| 色彩与参考合成 | 19 项单测（含后续定点坐标检查）及显式真实用例通过；77 帧的 2,838,528 个线性通道值与独立 zscale 最大误差 `4.768e-7`、RMS `6.967e-8`；真实计划输出四帧及旋转/透明度合成 | CPU 参考档案，不是高质量缩放、GPU、全 HDR 或成片导出 |
| 像素跨端一致 | Native/WASM 9 查询、7 渲染/2 拒绝，f32 bits、RGBA8 和诊断一致；编辑 25 步/编译 15 查询回归摘要不变 | 未执行浏览器 WebCodecs、WebGPU 或实际播放器 |
| 实际 GPU 合成 | Apple M4 Max/Metal，29 用例最大线性误差 `7.153e-7`，RGBA8 最大差 1；26 取消点、三类预算、callback panic 恢复通过 | 未验收 Windows/Linux/浏览器、设备丢失恢复、实时 FPS、零拷贝或产品显示 |
| 音频块处理 | 17 项音频基础测试、18 项编译测试；真实 AAC 五个完整输出；Native/WASM 8 查询的 PCM bits/计划/统计一致，任意分块与大块切片一致 | 生产 PCM Host、设备播放与浏览器未完成；单一素材不等于完整音质/性能验收 |
| 标准 PCM 随机读取 | 10 项默认测试；两段真实 AAC 各 144,000 样本/36 块；共 264 查询与两个完整倒放输出逐 bit 一致；Native/WASM 14 查询一致 | 测试文件 store 不是生产 Host；冷定位/长文件准备、OPFS、设备播放和完整产品未验收 |
| 保调计算 | 7 项原生行为；12 组实际 Native/WASM 全 PCM 输出一致；35 个分配故障点与 ASan/UBSan 通过 | 工程变速图与派生 PCM 另有集成测试；浏览器设备和产品播放器未完成 |
| 工程保调与派生 PCM | 13 项真实 Native 集成；5 个派生任务、124,555 个跨端比较样本逐 bit 一致；分割/拉长/嵌套政策/失效/候选重试与有界缓存通过 | 测试内存 Host；生产持久化、浏览器设备、声音试听与工作台未验收 |
| 原生编码候选 | 12 组实际文件/258 视频帧；短尾/分数帧率/时钟/独立解码通过，无损像素与 PCM 相等；两档案 ASan/UBSan、失败/预算/取消通过 | 编码 leaf adapter；未接工程渲染、worker 导出、浏览器或产品交付，AAC 质量与复杂内容单独验收 |
| 媒体内存检查 | wrapper 与 FFmpeg C 实现分别构建 ASan/UBSan 档案，同一真实检查通过 | 汇编与平台框架不因此被插桩；不是 hostile-media 隔离保证 |
| 原生媒体库体积 | 本机 release 配置 5 个 FFmpeg 库及 C wrapper 共 7,392,248 bytes | 不包含 Rust worker、字体、GPU、额外 codec 或系统框架，不是最终应用体积 |
| 代码与合同 | 整个 Rust 工作区 strict Clippy、WASM 编译、全部 schema/TypeScript 合同检查通过；新增视频 schema 使用相同生成/检查入口 | 未发布新版 SDK、组件频道或 UI |

长序列复现：

```sh
cargo test --release --offline --locked -p mo-video-edit --test generated_sequences -- --ignored --nocapture
```

跨端复现使用 `tools/verification/current.py --groups wasm`（包含原有 PPT wasm 门禁），需按既有构建文档提供固定 native 组件与匹配的 wasm-bindgen。视频测试脚本为 `tools/verification/video-domain-parity.mjs`；它要求本次构建的 Native 程序和 Node wasm-bindgen 输出，不接受用一份预录结果冒充其中一端。

## 明确尚未完成

- 当前已支持锚点线性变速；完整曲线变速/反查、转场、关键帧、字幕词锚点、分析与互操作等最终语义仍待补全。音频策略已保存，实际重采样/混音基础已实现，非原速保调及工程派生图已实现，生产 Host 接入和完整音频处理仍待完成。
- 原生 worker 已通过连续区间的真实工程采样测试，CPU/Metal GPU 已执行同源合成，但尚未成为产品播放链；音频混音已有内核实现，设备播放、GPU 显示/纹理互操作、产品工程导出与浏览器 I/O 尚未接入；编码/容器的 leaf adapter 已实现，编码 worker 已连接，宿主交付仍待连接。
- 生产 SDK/MCP/CLI、Device/Browser Host、Agent 技能、新时间线 UI、保存恢复和项目包尚未接入新内核。
- CE.SDK 仍存在于产品旧 worktree，未宣称依赖退役；旧 scene 数据须审计、迁移或保留可导出的原件，不能仅换 engineId。
- 完整功能、媒体质量、性能、体积、多平台与真实 provider 计费没有完成验收；当前不能交付为“用户已可验收全部视频功能”。

## 本地材料

发布核验材料：忽略目录 `.codex-work/stable-release-20261006/`。产品旧实现快照：其 worktree 的 `.codex-work/video-native-migration-20261006/`。

测试记录：`.codex-work/video-foundation-workspace-tests-02.log`、`video-domain-tests-final.log`、`video-edit-generated-100k-02.log`、`video-domain-conformance.json`、`video-foundation-clippy-workspace.log`、`video-final-component-clippy.log`、`video-schema-check.log`。区间编辑增量记录：`video-range-tests-final.log`、`video-range-generated-100k.log`、`video-range-conformance.json`、`video-range-clippy-final.log`、`video-range-schema-check.log`。这些本地记录不进入发行依赖。

编译与采样增量记录：`video-projection-tests-final.log`、`video-projection-conformance.json`、`video-projection-edit-regression.json`、`video-projection-workspace-clippy.log`、`video-projection-generated-100k.log`、`video-projection-schema-check-final.log`、`video-projection-repository-check.log`、`video-projection-types-check.log`。对应复现脚本为 `tools/verification/video-projection-parity.mjs`。

真实媒体增量记录：`.codex-work/media-native-checked/report.json`、`media-native-sanitized-current/report.json` 和 `media-native-gate-current/report.json`。素材 SHA-256 为 `ca7211a9b5e1bbdb66c7bd98e515484d7b6bc5ee8bbeeadb6cabb06d90bd73a6`。检查脚本 `tools/verification/media-native-check.py` 已加入当前 CI 的 media 分组；没有宣称该提交的远端 Linux CI 已经运行通过。

Rust 接入增量记录：`.codex-work/media-rust-gate-final/report.json`、其 `media-rust/report.json` 和 `.codex-work/media-rust-workspace-clippy.log`。media 分组现在顺序执行 C/Rust 真实媒体门禁，固定组件缺失仍报告失败。

进程额度增量记录：`.codex-work/media-process-limits-evidence-final.log`。父测试自动拉起专用 child fixture，完成后等待回收；该 fixture 的 ignored 标记不表示六项真实子进程检查被跳过。

跨进程媒体增量记录：`.codex-work/media-worker-gate-final/report.json`、其 `media-worker/report.json`、`.codex-work/media-worker-workspace-clippy-final.log`。media 分组顺序执行 C、Rust、构建 bundle 和实际 worker 四步；详细协议、额度与发行边界见 [worker 记录](video-media-worker.md)。

工程源采样增量记录：`.codex-work/video-source-gate-final/report.json`、其 `video-source/report.json`、`video-source-workspace-clippy-final.log`、`video-source-wasm-check.log`。media 分组已增加第五步实际工程采样检查；WASM 此处只验证编译，不宣称浏览器解码通过。

色彩与参考合成增量记录：`.codex-work/video-render-gate-final/report.json`、其 `video-render/report.json`、`video-render-parity.json`、`video-render-domain-regression.json`、`video-render-projection-regression.json`、`video-render-workspace-clippy.log`。media 分组已增加第六步真实色彩/合成，WASM 分组增加独立像素向量；边界与具体证据见[实施记录](video-color-composition.md)。

GPU 增量记录：`.codex-work/video-gpu-gate-final/report.json`、其 `hardware.json`、`video-gpu-color-regression/report.json`、`video-gpu-{render,domain,projection}-parity.json`、`video-gpu-workspace-clippy-final.log`。独立硬件门禁为 `tools/verification/video-gpu-check.py`；通用 CI 的 ignored 不视为硬件通过。

保调增量记录：`.codex-work/stretch-gate-final/report.json`、其 `video-stretch-parity.json`、`.codex-work/stretch-asan/build.json`、`stretch-workspace-clippy.log`。新增 `stretch` 验证分组，使用实际原生程序、Rust WASM 和单独 DSP WASM，不接受录制输出。

工程保调增量：`.codex-work/audio-retime-gate-final/report.json`、`video-stretch-parity.json` 的 `retime` 字段，以及 `.codex-work/audio-retime-all-tests-final.log`。测试使用真实 DSP 和内存持久候选，不代表生产工作台完成。
