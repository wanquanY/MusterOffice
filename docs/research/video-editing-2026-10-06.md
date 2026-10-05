# 视频剪辑项目、底层算法与 Agent 接口研究

2026-10-06 · 研究截止日 · 支持 [MusterOffice Video 完整设计](../design/video-editing.md)

## 1. 研究方法与证据边界

本轮问题是：怎样把视频剪辑作为 MusterOffice 的第二个原生办公场景，让 Agent 和用户编辑同一个工程，并尽量利用用户设备完成预览和导出。研究不是寻找一个包装成 MCP 的现成工具就结束，也不预设自研一定优于成熟产品。

采用三层证据：

- **源码观察**：取得 12 个仓库的固定提交元数据及目录树，下载固定提交的源码归档或文件快照，保存 62 个关键源码、文档及许可文件；对其中的时间、编辑、事务、媒体队列和渲染路径做定点阅读。未遍历全部实现，也未构建或运行这些第三方编辑器。
- **官方资料**：阅读项目原作者的架构、API、平台和商业说明。闭源产品只确认公开合同，不推断私有算法。动态网页无法完整读取时，标为部分证据。
- **本地实验**：用自行生成的数学向量和媒体验证时间量化、VFR 与 PTS/DTS 差异；不是端到端性能比较。

事实、推断和方案选择分开表述。许可证是本次快照中的文本观察，不构成对整个依赖闭包的许可结论；最终分发需对应真实构建材料检查。

### 1.1 本地材料

固定提交与文件 SHA-256 清单保存在本仓忽略目录 `.codex-work/video-research-20261006/`。`repos.json` 是提交清单；`source-files.json` 是文件路径、来源提交、大小与摘要；`sources/` 是只读研究快照；`experiments/` 保存原始实验结果。第三方源码不进入 MusterOffice 原创代码或公共发行包。

下表时间是**所取提交的时间**，不是整个项目最后活动时间。尤其不能从旧提交日期推断项目已经停止维护。

| 仓库 | 固定提交 | 提交时间（UTC） | 本地关键文件数 |
| --- | --- | --- | --- |
| AcademySoftwareFoundation/OpenTimelineIO | [`fc5c58e16f68`](https://github.com/AcademySoftwareFoundation/OpenTimelineIO/commit/fc5c58e16f6832972cdb5c656dd8a6d87a4e5b02) | 2026-10-01 20:06:50 | 6 |
| mltframework/mlt | [`37b174a36ff7`](https://github.com/mltframework/mlt/commit/37b174a36ff7422b3b0c839c305b74cf3ff49d90) | 2026-10-05 18:32:40 | 4 |
| KDE/kdenlive | [`86a5ffe27146`](https://github.com/KDE/kdenlive/commit/86a5ffe2714695daefde30732feec44381689837) | 2026-10-05 02:06:12 | 4 |
| olive-editor/olive | [`7e0e94abf661`](https://github.com/olive-editor/olive/commit/7e0e94abf6610026aebb9ddce8564c39522fac6e) | 2024-12-05 10:34:16 | 5 |
| Vanilagy/mediabunny | [`9d36fc5779b2`](https://github.com/Vanilagy/mediabunny/commit/9d36fc5779b254904bc609e66f2cd80ed8390b4b) | 2026-10-05 13:27:10 | 10 |
| WebAV-Tech/WebAV | [`42dd6fd65ccb`](https://github.com/WebAV-Tech/WebAV/commit/42dd6fd65ccbda39c729265c14aab56ac4d28235) | 2026-01-10 09:53:51 | 5 |
| remotion-dev/remotion | [`328178a68a80`](https://github.com/remotion-dev/remotion/commit/328178a68a8031167649273405f2405028bd2305) | 2026-10-05 16:59:22 | 4 |
| GuanYixuan/pyJianYingDraft | [`6bb2d06a713f`](https://github.com/GuanYixuan/pyJianYingDraft/commit/6bb2d06a713f7b58e1375269daec835a03882086) | 2026-09-26 09:04:37 | 5 |
| sun-guannan/CapCutAPI | [`cfa4779a52f1`](https://github.com/sun-guannan/VectCutAPI/commit/cfa4779a52f1dc59a56d90cc5868778eb50c6a97) | 2026-10-03 12:43:09 | 5 |
| OpenShot/libopenshot | [`0fbacca9d54a`](https://github.com/OpenShot/libopenshot/commit/0fbacca9d54a07982a2a000de5625d8526d45664) | 2026-09-27 23:44:39 | 5 |
| OpenCut-app/OpenCut | [`e66801077856`](https://github.com/OpenCut-app/OpenCut/commit/e668010778568641babef2cc40be4703ae6916d6) | 2026-09-24 08:58:46 | 3 |
| 0xsline/OpenChatCut | [`297b3279e8f3`](https://github.com/0xsline/OpenChatCut/commit/297b3279e8f34bd8cb984a64634efdd8db3cb858) | 2026-10-05 13:45:35 | 6 |

`CapCutAPI` 的请求重定向到 `VectCutAPI`，本地清单保留最初请求名，源码链接使用重定向后的仓库名。OpenCut、OpenCut Classic 和 OpenChatCut 是不同的研究对象，不把重写计划或某个分支的功能套到另一个产品上。

## 2. 会改变选型的研究结论

1. **有针对 Agent 的实际编辑项目，也有同方向的原生重写。** OpenChatCut 提供 MCP 与共用编辑命令；OpenCut 官方重写计划采用 Rust 核心、Editor API、MCP 和无界面模式。因此“面向 Agent 的原生剪辑”不是无人验证的方向，但新 OpenCut 的规划不能当成已完成产品。[OpenChatCut](https://github.com/0xsline/OpenChatCut)、[OpenCut 官方重写跟踪](https://github.com/OpenCut-app/OpenCut/issues/811)
2. **Web 编辑不必等于服务端编码。** WebCodecs、WebAV、Mediabunny 以及 Remotion 的客户端渲染都提供浏览器内处理路径。代价转移到设备能力、内存、页面生命周期和本地存储；这不等于所有浏览器都具有相同的 codec。[WebCodecs 标准](https://www.w3.org/TR/webcodecs/)、[Remotion 客户端渲染](https://www.remotion.dev/docs/client-side-rendering)
3. **不能说剪映生态没有官方编辑接入。** 火山引擎 AI MediaKit 的官方 Web SDK 包含编辑器与工程接口，但所读指南明确描述服务端异步合成。这是与本地优先目标不同的运行方式。它与第三方修改剪映草稿的项目不能混称。[官方集成指南，部分读取](https://docs.volcengine.com/docs/Intelligentprocessing/WebClipSDKIntegrationGuide?lang=zh)
4. **“原生、小体积、高性能”必须落实到模块与测量。** 一个 Rust 包装层不消除 codec、字体、纹理搬运、代理文件及运行时的成本。包体与性能要按完整发行闭包测量，不能只报告核心 WASM 或空工程的大小。
5. **本项目最值得拥有的是工程语义、可靠编辑和媒体执行计划。** 成熟的解复用、codec、重采样可复用；而操作事务、精确时间、人工与 Agent 并发、恢复、质量档案必须由自己的公共合同约束。这是基于下列对照的架构建议，不是本轮已经证明的性能优势。

## 3. 项目对照：采用、借鉴与不采用的边界

“源码”指已阅读关键路径；“官方”指官方文档层；“部分”指读取受限或仅覆盖特定接口。此表不把库、编辑器、交换格式和云渲染服务当成同一种产品评分。

| 项目/类别 | 证据层 | 最值得吸收的能力 | 与本目标的取舍 |
| --- | --- | --- | --- |
| OpenTimelineIO / 交换模型 | 源码 | Clip/Track/Stack、源范围与可用范围、转换边界 | 适合互操作；不承担解码、实时预览或权威业务提交 |
| MLT / 媒体框架 | 源码+官方 | producer/consumer、惰性帧求值、多轨及转场 | 借鉴拉取式执行；不直接把属性图暴露为 Agent 工程协议 |
| Kdenlive / 桌面编辑器 | 源码 | 统一时间线修改入口、稳定身份、可组合撤销与吸附 | 编辑语义参考价值高；Qt/UI 绑定及 GPL 来源不能直接并入原创内核 |
| Olive / 节点编辑器 | 源码 | 节点渲染、预览缓存、范围失效、取消优先级 | 借鉴算法分层；README 对快照明确标注不稳定，未做产品成熟度背书 |
| libopenshot / 编辑库 | 源码 | 时间线、FrameMapper、重采样与帧缓存 | 对照混合帧率与内存策略；完整库不是我们的跨端工程合同 |
| GStreamer Editing Services | 官方 | 非线性编辑语义、pipeline、硬件插件与内存类型 | 原生媒体栈的有效替代；插件闭包及浏览器路径与本目标仍需单独解决 |
| Shotcut | 官方 | 面向完整编辑器的 MLT 工程和交互组织 | 作为用户操作与互操作对照；没有开展源码深读 |
| FFmpeg | 官方+本机实验 | 容器、codec、音视频转换和明确的收发状态机 | 选为原生底层候选；它本身不提供完整的 Agent 工程与用户编辑状态 |
| ffmpeg.wasm | 官方 | 在浏览器复用部分 FFmpeg 能力 | 可作为特定兼容模块评估，不作为默认大媒体执行路线；不引用旧基准预测今天速度 |
| WebAV | 源码 | 浏览器剪辑组合、WebCodecs、流式输出 | 借鉴低层浏览器路径，避免另建与 Rust 不同的项目语义 |
| Mediabunny | 源码 | 容器读写、样本访问、解码与背压 | 作为浏览器媒体 I/O 优先候选；当前快照 MPL-2.0，按发行闭包审计 |
| Remotion | 源码+官方 | React 声明式合成、模板、客户端与服务器渲染 | 对代码生成/模板很有价值；自有数据工程与原生核心不依附 DOM，且存在商业许可条件 |
| Editly | 官方仓库 | 声明式 CLI/API 视频装配 | 适合自动拼接参考；不能据此推断已有完整人机共享编辑器 |
| OpenCut 官方重写 | 源码+官方 | 计划中的 Rust 共享核心、MCP、Editor API | 路线相近；当前所取树是重写基础，不当作已完成替代引擎 |
| OpenChatCut | 源码+官方 | Agent/MCP、共享编辑命令、提案与人工编辑闭环 | 重要产品/事务对照；AGPL 来源与 Remotion 基础不直接复制为 Apache 原创领域 |
| pyJianYingDraft | 源码 | 剪映草稿生成、轨道和素材适配 | 可研究外部交换；其自动导出依赖特定 Windows UI，不能作为默认无界面核心 |
| VectCutAPI（原 CapCutAPI） | 源码 | HTTP/MCP 工具封装、草稿对象操作 | 有接入价值；README 明确部分 Agent/在线编辑/云渲染未开源，不能称整套开源引擎 |
| 火山 AI MediaKit / 官方商业服务 | 部分官方 | Web 编辑 SDK、工程和素材 API、异步任务 | 完整服务路线可选；已读导出路径是服务端合成，与默认本地导出目标不同 |
| IMG.LY CE.SDK / 商业 SDK | 官方 | 可嵌入 UI、无界面引擎及原生服务入口 | 有效替代，不能误写成只有 Web UI；权威工程与商业依赖仍需接受相应边界 |
| Adobe Premiere Pro / 商业编辑器 | 官方 | UXP API、受锁保护的 CompoundAction 事务 | 可做既有专业工作流适配；不能等同可嵌入、可再分发的跨端内核 |
| DaVinci Resolve Studio / 商业编辑器 | 官方 | Python/Lua、工作流插件与集成接口 | 自动化出口备选；本轮未验证具体版本的全部脚本能力与远程合同 |
| Final Cut Pro / 商业编辑器 | 官方，含归档文档 | FCPXML 工程交换 | 作为选择版本的交换目标，不能把 XML 导出视为嵌入式渲染能力 |
| Descript / 商业产品 | 官方 | 文本与媒体联动、明确区分改稿和剪媒体 | 借鉴用户语义；不推断其私有 ASR/时间算法 |
| Shotstack / 云服务 | 官方 | JSON 编辑描述、异步渲染与交付 | 适合显式云渲染集成；当前目标不默认部署或消费云合成 |
| Creatomate / 云服务 | 官方 | 模板、修改项、RenderScript 与异步成片 | 模板/营销场景参考；不把公开 API 理解为本地 SDK |

表中每个项目的直接证据在以下源码章节和第 9 节来源索引。未安装商业试用、未购买服务、未运行真实付费生成，也未进行完整 GUI 易用性测试。

## 4. 源码阅读：具体学习什么，怎样改进

### 4.1 OTIO：学习时间域，不能因名称假定整数精度

源码区分素材可用范围、片段来源范围以及在父时间线中的位置。这支持“源内容”和“放置实例”分离。`RationalTime` 在该快照中以 `double` 保存 value/rate；名称并不等于任意精确的整数分数。[RationalTime 源码](https://github.com/AcademySoftwareFoundation/OpenTimelineIO/blob/fc5c58e16f6832972cdb5c656dd8a6d87a4e5b02/src/opentime/rationalTime.h)、[架构说明](https://github.com/AcademySoftwareFoundation/OpenTimelineIO/blob/fc5c58e16f6832972cdb5c656dd8a6d87a4e5b02/docs/tutorials/architecture.md)

**设计选择**：内部沿用 MusterOffice 有界整数字符串 ticks/timescale，运算检查溢出、范围和舍入；仅在 OTIO 边界显式转换。不能从上述类型事实推断 OTIO 存在可见漂移缺陷，本轮没有对它运行误差测试。

### 4.2 Kdenlive：先保证一个修改入口，再处理事务

`TimelineModel` 的设计说明要求外部经模型修改时间线，并维护自身身份；`undohelper.hpp` 把操作与逆操作组合，失败时回滚已执行步骤。吸附实现有有序查找与忽略本次拖动对象边界的处理。[TimelineModel](https://github.com/KDE/kdenlive/blob/86a5ffe2714695daefde30732feec44381689837/src/timeline2/model/timelinemodel.hpp)、[撤销组合](https://github.com/KDE/kdenlive/blob/86a5ffe2714695daefde30732feec44381689837/src/undohelper.hpp)、[吸附实现](https://github.com/KDE/kdenlive/blob/86a5ffe2714695daefde30732feec44381689837/src/timeline2/model/snapmodel.cpp)

**吸收**：稳定 ID、统一入口、分组撤销和局部吸附。**改变**：我们的事务运行在不可变基线上的候选状态，成功后宿主 CAS 提交；prepare 与 apply 共用 reducer，避免两套校验逻辑分叉。闭包式撤销不能直接跨进程持久化，所以公共协议采用可序列化操作、回执及带前置条件的逆操作。这是场景差异，不是说 Kdenlive 的桌面撤销设计错误。

### 4.3 MLT 与 Olive：借鉴惰性图和失效范围

MLT 的帧可以携带后续求值逻辑，消费者拉取图像/音频，多轨和转场决定本时刻参与的内容。Olive 将节点处理、帧缓存和预览预缓存拆开，并处理范围失效、当前帧优先与取消。[MLT tractor](https://github.com/mltframework/mlt/blob/37b174a36ff7422b3b0c839c305b74cf3ff49d90/src/framework/mlt_tractor.c)、[MLT 官方架构](https://www.mltframework.org/docs/framework/)、[Olive 渲染](https://github.com/olive-editor/olive/blob/7e0e94abf6610026aebb9ddce8564c39522fac6e/app/render/renderprocessor.cpp)、[预览调度](https://github.com/olive-editor/olive/blob/7e0e94abf6610026aebb9ddce8564c39522fac6e/app/render/previewautocacher.cpp)

**设计选择**：编译为区间索引 + RenderPlan DAG，按需求求值，缓存键包含输入内容、效果版本、时间映射、字体及质量档案；当前播放/seek 优先于后台代理与缩略图。需要注意，所读 Olive 缓存代码含时间戳/UUID 等管理，不应仅凭 `FrameHashCache` 类名宣称它已经实现我们的完整内容寻址方案。[帧缓存](https://github.com/olive-editor/olive/blob/7e0e94abf6610026aebb9ddce8564c39522fac6e/app/render/framehashcache.cpp)

不采用整套旧工程/UI 对象作为核心真相。我们需要能在无窗口环境运行、可取消、有资源上限且可跨端验证的执行计划；实际吞吐是否更好仍要实测。

### 4.4 libopenshot：帧映射和缓存不是平均帧率换算

`Timeline.cpp` 将帧缓存与 FrameMapper 纳入获取路径；`FrameMapper.cpp` 包含输入/输出帧率和音频样本映射、重采样处理。它说明混合帧率不能只修改项目 fps 标签。[Timeline](https://github.com/OpenShot/libopenshot/blob/0fbacca9d54a07982a2a000de5625d8526d45664/src/Timeline.cpp)、[FrameMapper](https://github.com/OpenShot/libopenshot/blob/0fbacca9d54a07982a2a000de5625d8526d45664/src/FrameMapper.cpp)

**设计选择**：CFR 项目以精确帧时刻采样，VFR 来源按实际展示时间戳定位；解码顺序按 DTS，画面选择按 PTS。缓存用字节/帧/时间预算共同约束，不能仅按 CPU 数量无限放大。未运行该库的 VFR 测试，不声称它存在对应缺陷。

### 4.5 Mediabunny 与 WebAV：媒体队列比 API 外观更关键

Mediabunny 的样本读取代码专门处理 B 帧重排、关键包起点、解码队列和异步错误；编码路径等待编码队列及封装背压，并释放媒体资源。WebAV 的组合器通过 OffscreenCanvas、片段时间、音频缓冲和流式封装组织浏览器输出。[解码/采样](https://github.com/Vanilagy/mediabunny/blob/9d36fc5779b254904bc609e66f2cd80ed8390b4b/src/media-sink.ts)、[编码/背压](https://github.com/Vanilagy/mediabunny/blob/9d36fc5779b254904bc609e66f2cd80ed8390b4b/src/media-source.ts)、[WebAV 组合器](https://github.com/WebAV-Tech/WebAV/blob/42dd6fd65ccbda39c729265c14aab56ac4d28235/packages/av-cliper/src/combinator.ts)

**吸收**：边解码边消费、有界队列、flush/drain 状态和所有失败路径释放。**改变**：不复制固定队列常数作为通用最优值；结合分辨率、帧字节及播放延迟动态预算。浏览器容器库只负责样本 I/O，不能拥有另一份独立项目模型。README 中的速度倍率未在本机复现，不用作选型结论。

### 4.6 Remotion：客户端路径是重要反证

官方已将客户端渲染标为稳定能力（文档标注从 v4.0.491 起）；底层使用 WebCodecs/Mediabunny，支持范围不同于完整浏览器页面截图。下载的 Web renderer 源码还可看到媒体渲染队列管理。不能沿用“Remotion 必须服务器渲染”的旧判断。[客户端渲染](https://www.remotion.dev/docs/client-side-rendering)、[Web 渲染入口](https://github.com/remotion-dev/remotion/blob/328178a68a8031167649273405f2405028bd2305/packages/web-renderer/src/render-media-on-web.tsx)、[操作队列](https://github.com/remotion-dev/remotion/blob/328178a68a8031167649273405f2405028bd2305/packages/web-renderer/src/render-operations-queue.ts)

**保留的价值**：Agent 编写动效、模板与声明式内容。**不作为本次权威核心的原因**：目标是可检查的数据操作、共享原生语义和有界执行，不能把运行任意 React/JS 当作修改工程的标准动作。商业条件也需按组织和用途核对，源码可见不等于无限制开源。[官方许可](https://www.remotion.dev/docs/license/pricing)

### 4.7 剪映相关：草稿、自动导出、官方 SDK 是三层不同能力

pyJianYingDraft 的控制器使用 Windows UI Automation 定位和操作剪映窗口；其 README 对自动导出写明旧版界面限制，而草稿相关功能有另一套版本支持。不能概括为“新剪映全部不支持”。[草稿项目说明](https://github.com/GuanYixuan/pyJianYingDraft/blob/6bb2d06a713f7b58e1375269daec835a03882086/README.md)、[控制器](https://github.com/GuanYixuan/pyJianYingDraft/blob/6bb2d06a713f7b58e1375269daec835a03882086/pyJianYingDraft/jianying_controller.py)

VectCutAPI 给创建草稿、添加视频/音频等提供 MCP 调用，但其 README 对部分 Agent、网页编辑端和云渲染代码标注未开源。接口包装能降低 Agent 接入难度，仍不能自动解决编辑器版本、无界面合成和可分发内核问题。[MCP 封装](https://github.com/sun-guannan/VectCutAPI/blob/cfa4779a52f1dc59a56d90cc5868778eb50c6a97/mcp_server.py)、[开源范围](https://github.com/sun-guannan/VectCutAPI/blob/cfa4779a52f1dc59a56d90cc5868778eb50c6a97/README.md)

火山 AI MediaKit 的官方指南则描述真实 Web SDK、业务代理、服务端工程与异步合成；其中导出复制当前编辑数据，用户可以继续编辑，值得吸收“导出绑定快照”的语义。本轮直接页面正文受动态渲染限制，主要依据官方域名的搜索索引片段，未验证账号权限、商业条款或运行 SDK。[官方指南](https://docs.volcengine.com/docs/Intelligentprocessing/WebClipSDKIntegrationGuide?lang=zh)

**选型**：剪映/CapCut 作为可选交换或外部工作流，AI MediaKit 作为明确的商业云方案备选；均不成为默认本地剪辑的唯一执行依赖。

### 4.8 OpenCut：相同方向不等于已经完成

固定提交 README 清楚说明重写中，计划包含 Rust 核心、Editor API、MCP、无界面与插件；经典版本是另一仓库。目录树和 FFmpeg 版本/散列清单可以支持“正在搭建基础”的观察，不能支持“已有成熟跨端 Rust 剪辑内核”的结论。[重写说明](https://github.com/OpenCut-app/OpenCut/blob/e668010778568641babef2cc40be4703ae6916d6/README.md)、[媒体构建清单](https://github.com/OpenCut-app/OpenCut/blob/e668010778568641babef2cc40be4703ae6916d6/crates/media/setup/ffmpeg.json)

**设计启示**：核心与 UI 分离、Agent 和批处理共用能力是合理方向；同方向项目应持续跟踪。采用与否应看公开合同、真实运行和质量数据，不因 MIT 许可或 Rust 标签就替换自己的设计，也不把尚未实现的能力当作竞争缺陷。

### 4.9 OpenChatCut：对 Agent 人机协作最有价值的实际参考

公开命令接口覆盖时间线、素材重连、字幕与文字剪辑；提案提交代码检查是否过期、保存失败及恢复结果。外部提案路径也检查当前版本，并限制回滚覆盖更新内容的条件。这比让 Agent 点击按钮更接近本项目目标。[编辑命令](https://github.com/0xsline/OpenChatCut/blob/297b3279e8f34bd8cb984a64634efdd8db3cb858/src/editor/storeCommands.ts)、[提案保存](https://github.com/0xsline/OpenChatCut/blob/297b3279e8f34bd8cb984a64634efdd8db3cb858/src/agent/proposal-commit.ts)、[外部提案](https://github.com/0xsline/OpenChatCut/blob/297b3279e8f34bd8cb984a64634efdd8db3cb858/src/agent/external-proposal-apply.ts)

**值得保留**：共用命令、隔离草稿、版本检查、一次可撤销提交。**我们的进一步约束**：内容 head、操作回执和待发布事件在宿主同一原子提交中写入，媒体文件先按内容固化，再提交引用，减少所读函数中多步骤保存/补偿恢复的复杂度。

这里比较的是已读函数的提交边界，**没有据此断言整个 OpenChatCut 缺少事务或会丢数据**；底层持久化和全部故障测试未审计。其工程将 Remotion 作为主要基础，仓库采用 AGPL；可以研究接口与行为，不把源码直接移入 Apache 原创模块。[项目说明](https://github.com/0xsline/OpenChatCut/blob/297b3279e8f34bd8cb984a64634efdd8db3cb858/README.md)、[仓库许可](https://github.com/0xsline/OpenChatCut/blob/297b3279e8f34bd8cb984a64634efdd8db3cb858/LICENSE)

## 5. 闭源产品的有效启发和边界

### 5.1 Premiere：公共自动化也需要可靠事务

Premiere Pro 的 UXP 文档在 26.3 中强调在 `lockedAccess` 内创建动作再提交 CompoundAction，用来避免其他编辑与撤销介入引起的不一致。由此可见，专业编辑器有正式 API，且并发约束不应交给 Agent 自觉遵守。[官方变更记录](https://developer.adobe.com/premiere-pro/uxp/changelog/)、[CompoundAction](https://developer.adobe.com/premiere-pro/uxp/ppro-reference/classes/compoundaction)

本方案采用可串行化提交与显式冲突，而非长时间锁住用户 UI 等待模型思考。其适用差异需要明确：提案可以长期存在，写锁/提交窗口必须短。

### 5.2 CE.SDK：可用替代不应被矮化

CE.SDK 公开提供无界面设计引擎与 Node Native 接入，不只是页面上的时间线组件。[原生接入](https://img.ly/docs/cesdk/node-native/get-started/overview-e18f40/)、[无界面引擎](https://img.ly/use-cases/headless-design-engine/)

选择自有内核意味着承担媒体质量、编辑语义、兼容与维护责任。若采用商业 SDK，必须接受其工程/发行合同、授权和集成边界；本轮没有获取商务报价或验证全部离线权益，不能断言它必然更贵、更慢或不能本地运行。

### 5.3 Descript：改字幕和按文字剪辑必须明确分开

Descript 的官方教学把媒体与文本关联起来，同时区分只移除文稿内容的动作。这是重要的交互经验：用户改错字不应无意删除画面。[官方教学](https://help.descript.com/getting-started/edit-like-a-doc)

我们的词锚点绑定源素材时间；同一素材多次入片，不共享会误删所有实例的可变标记。ASR 只是可更新分析结果，编辑决策保存在工程操作中。

### 5.4 云渲染 API 与外部专业编辑器

Shotstack 和 Creatomate 提供声明式输入与异步交付；适合需要无人设备在线、批量模板视频的场景。[Shotstack API](https://shotstack.io/docs/api/)、[Creatomate Render API](https://creatomate.com/docs/api/reference/create-a-render)

Resolve 的公开脚本/工作流集成、Final Cut 的 FCPXML 交换则适合与专业桌面工具衔接。[Resolve Studio](https://www.blackmagicdesign.com/products/davinciresolve/studio)、[FCPXML](https://developer.apple.com/documentation/professional-video-applications/fcpxml)

本项目默认本地执行；以后若增加云模式，必须成为独立可选执行档案，明确素材上传、资源费用及结果归属。外部工程交换必须报告不支持的效果，不能把成功写出 XML 当作完整无损编辑闭环。

## 6. 算法选型与需要战胜的失败模式

以下是研究后的**推荐实现**，不是对参考项目已有缺陷的指控。具体合同与公式见 [技术规格](../design/video-editing/technical-spec.md)。

| 问题 | 采用的设计 | 为什么不选更简单的做法 | 实施证据要求 |
| --- | --- | --- | --- |
| 时间精度 | 有界整数有理时间；源、工程、片段三个时间域；边界统一舍入 | 每帧先取整再累计会引入漂移 | 长时、混合帧率和音频样本向量 |
| VFR/B 帧 seek | 关键帧索引、按 DTS 送解码、按 PTS 选帧、generation 丢弃旧结果 | 平均 fps 推帧号或提前停止喂包会选错画面 | 可识别帧标签、乱序展示、跨 GOP 随机 seek |
| 倒放与变速 | 分段 TimeMap；GOP 有界反向窗口；保调时伸补偿延迟 | 负 playbackRate 或视频变速而音频不映射不能完成可靠处理 | 边界、速度拐点、声画和缓存预算 |
| 非线性编辑 | 不可变快照 + 原子 reducer；稳定 ID；明确 ripple/roll/slip/slide | 通用 JSON patch 无法表达锁轨、源余量和关联组语义 | 不变量/逆操作/随机序列 |
| Agent 与人工交错 | 提案读取版本、声明读写集、CAS 提交；有限安全重基 | 最后写入覆盖会吞人工修改；全量长锁破坏交互 | 并发、重复请求、恢复回执 |
| 帧图与缓存 | 区间索引、增量 DAG、完整内容键、LRU/成本预算 | 全片重算慢；只按文件路径缓存会误用旧内容 | 对照禁缓存输出、失效和内存测试 |
| 解码到 GPU | 显式资源/所有权、可测的纹理与色彩转换路径 | “WebGPU/wgpu”不自动保证零拷贝 | 每阶段复制量、硬件回退和设备丢失测试 |
| 音频 | 精确样本时钟、预分配混音、重采样/时伸延迟补偿 | 视频帧循环推动音频造成累积误差与卡顿 | 长时同步、峰值/响度和盲听 |
| 字幕与颜色 | 固定字体塑形；确定布局；线性合成、明确 SDR/HDR 策略 | DOM 预览与另一套导出文字会不一致 | 中文/RTL/emoji、颜色/alpha 标准语料 |
| 断点导出 | 固定快照、可重放计划、分段校验、状态预热与最终封装 | 生拼编码片段可能多帧、音频断续、效果状态不连续 | 从头/恢复导出逐段对照 |

### 6.1 媒体与图形基础的具体选择

原生以 FFmpeg 窄接口承担读写、codec 与音频转换，核心拥有编辑和合成语义。FFmpeg send/receive 在 `EAGAIN` 时要求推进另一端，必须显式 drain；不能靠 sleep 猜测队列就绪。[官方状态机](https://ffmpeg.org/doxygen/trunk/group__lavc__encdec.html)

浏览器以 Mediabunny 处理容器和样本，WebCodecs 处理平台 codec。WebCodecs 不保证任何指定 codec 在所有实现上可用；能力探测还需实际配置与样本校验。[标准](https://www.w3.org/TR/webcodecs/)

原生 wgpu 和浏览器 WebGPU 承担共享 RenderPlan 的合成，CPU 保留确定性参考档案。外部纹理的零拷贝取决于平台实现，不能把标准能力名称当测量结论。[WebGPU 外部纹理，部分索引证据](https://www.w3.org/TR/webgpu/#gpuexternaltexture)、[wgpu](https://docs.rs/wgpu/latest/wgpu/)

Signalsmith Stretch 是保调变速的优先候选：作者提供输入/输出延迟及自动化时序说明。这些延迟必须映射回工程时钟；本轮未编译 Native/WASM，也未证明质量优于其他时伸算法。[作者说明](https://signalsmith-audio.co.uk/code/stretch/)

GES 文档明确支持硬件插件及内存类型处理，因此它是原生替代路线。未选择它作为默认组合，是为了缩小默认插件闭包并保持与浏览器的自有计划一致；这是可维护性判断，不是性能实测排序。[GES 硬件文档](https://origin.gstreamer.freedesktop.org/documentation/gst-editing-services/hardware-acceleration.html)

### 6.2 应避免的产品和架构做法

- 用 Agent 模拟点击作为唯一编辑 API：无法稳定给出语义回执与恢复边界。
- 将生成服务、剪辑工程、解码器和计费塞入一个“视频服务”：资源生命周期与责任混杂。
- 认为“有 MCP”就已 Agent 原生：还要可检查、可分组、可撤销、幂等、版本冲突可定位。
- 让浏览器和桌面各自维护编辑 JSON：效果、变速、字幕迟早出现不同解释。
- 以画布能播放作为导出一致性证据：字体、色彩、采样时间和音频需要共同合同。
- 默认播放器塞满工程参数：大部分用户首先需要看结果和下载，编辑 UI 应在主动进入后展开。

这些取舍与用户目标相关，不适合作为对所有剪辑产品的绝对评价。例如脚本式合成非常适合模板批量生成，云渲染非常适合无人值守；本方案的默认场景是人机接续编辑和本地资源优先。

## 7. 本地实验与可复现结果

### 7.1 逐帧量化的累计误差

环境：macOS 26.2 arm64、Python 标准库 Fraction。取 107,892 帧、30000/1001 fps，总时长 3599.9964 秒。

```python
from fractions import Fraction
n = 107892
frame = Fraction(1001, 30000)
exact = n * frame
video_error_ms = (n * round(frame * 1_000_000) / Fraction(1_000_000) - exact) * 1000
audio_error_ms = (n * round(frame * 48000) / Fraction(48000) - exact) * 1000
print(float(exact), float(video_error_ms), float(audio_error_ms))
```

| 算法 | 本轮结果 |
| --- | --- |
| 每帧时长先取整为微秒，再累计 | 比精确时刻多 35.964 ms |
| 每视频帧分配固定的四舍五入音频样本数，再累计 | 比精确时刻多 899.1 ms |

正确做法是从统一绝对时刻映射边界，必要时使用余数累积，而不是把每个片段/每帧的量化误差永远相加。这是对两种构造算法的实验，不是对任一被研究编辑器的缺陷复现。

### 7.2 VFR 与解码/展示顺序

使用本机 FFmpeg 7.1.1 从测试图生成 3 秒合成素材，无外部用户媒体。下面命令在空目录可复现；`-y` 只用于这份明确的测试输出。

```sh
ffmpeg -v error -f lavfi -i 'testsrc2=size=160x90:rate=60:duration=3' \
  -vf 'select=if(lt(t\,1)\,not(mod(n\,2))\,not(mod(n\,3)))' \
  -fps_mode vfr -c:v mpeg4 -bf 2 -q:v 5 -y synthetic-vfr.mp4
ffprobe -v error -select_streams v:0 -show_packets \
  -show_entries packet=pts_time,dts_time,duration_time,flags \
  -of json synthetic-vfr.mp4
```

结果为 70 个视频包。排序后的 PTS 间隔有约 1/30 秒和 1/20 秒两类（输出小数中 0.033333 为 20 次、0.033334 为 10 次、0.05 为 39 次）；PTS 与 DTS 不相同。例如一个包 PTS=0.1 秒、DTS=0 秒，后续包 PTS=0.033333 秒。

这验证了测试素材确有可变间隔和重排：按包到达次序展示、用固定 fps 反算所有来源时刻都不成立。该实验只提供设计向量，没有测量渲染速度、seek 延迟或跨浏览器一致性。

## 8. 决策、反证与尚未证明的事项

| 候选路线 | 有力理由 | 未采用为默认主线的理由 | 仍保留的用途 |
| --- | --- | --- | --- |
| 商业 SDK 作为核心 | 已有编辑与渲染合同，集成面明确 | 工程权威、分发与长期控制受外部合同约束；不完全贴合现有纯计算领域 | 对照、商业集成、特定客户方案 |
| 剪映草稿 + UI 自动化 | 能连接用户已有剪映工作流 | 本次观察的导出依赖界面/版本，跨端与无界面恢复不足以由草稿协议保证 | 交换适配 |
| Remotion/模板代码为权威工程 | 动效与 Agent 代码生成自然，有客户端路径 | 任意程序与可检查的有界编辑数据并非同一合同；原生共享语义需额外抽象 | 可审查模板、隔离的烘焙素材 |
| 直接使用大型原生媒体框架 | 原生管线成熟、插件丰富 | 浏览器不是同一执行闭包，仍需自己的事务/工程；体积性能未比较 | 原生后端替代评估 |
| 自有领域 + 成熟媒体基础 | 所有调用者共享语义、本地执行、适配可替换 | 工作量与验证责任最大，性能与体积不能先验保证 | 本方案建议 |

待验证的关键事项：

1. 原生解码表面与 wgpu、浏览器 VideoFrame 与 WebGPU 的真实复制路径；支持矩阵要按硬件与驱动固定。
2. 指定浏览器的编码格式、长片内存、后台节流和存储配额行为。OPFS 不等于永不被清理的备份；需要显式保存工程包。[OPFS](https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system)、[存储配额与驱逐](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria)
3. 时伸与重采样 Native/WASM 的音质、延迟、体积；分段恢复后能否达到同一质量档案。
4. FFmpeg 所选 codec、系统硬件组件与完整发行闭包的实际义务；当前推荐构建不默认包含 GPL/nonfree 组件。[官方许可](https://ffmpeg.org/legal.html)
5. 商业 SDK/服务的账号可用性、离线/分发合同、计费与地区规则；本轮没有合同或付费调用证据。
6. OpenCut 重写的实现进度、OpenChatCut 未审计路径与所有参考产品实际性能。设计可以吸收它们的新成果，不靠贬低对方为自研辩护。
7. 本次没有重做所有视频生成厂商和 Seedance 具体版本的可用性验证；模型协议与计费在总方案中保留正式适配设计，不能将该设计写成上游已经开放或本地已经跑通。

## 9. 其他直接来源与阅读范围

源码链接在第 4 节均固定到本次 SHA；以下为正文和对照表补充来源，访问日均为 2026-10-06。

| 来源 | 本轮使用范围 |
| --- | --- |
| [OTIO 官方架构](https://opentimelineio.readthedocs.io/en/latest/tutorials/architecture.html) | 时间线元素与交换模型 |
| [GES 编辑模式](https://gstreamer.freedesktop.org/documentation/gst-editing-services/ges-enums.html) | ripple、roll、slide 定义 |
| [Shotcut 功能](https://shotcut.org/features/) | MLT XML 与编辑器能力；未运行 UI |
| [Editly 原作者仓库](https://github.com/mifi/editly) | 声明式 CLI/API；未下载构建 |
| [ffmpeg.wasm 基准](https://ffmpegwasm.netlify.app/docs/performance/) | 历史测试条件，不能外推为本轮设备测量 |
| [Mediabunny 官方介绍](https://mediabunny.dev/guide/introduction) | 容器/浏览器媒体边界，结合固定源码验证 |
| [WebAV 原作者仓库](https://github.com/WebAV-Tech/WebAV) | 功能说明，速度宣传未复测 |
| [火山编辑 API](https://docs.volcengine.com/docs/Intelligentprocessing/EditingtoolAPI?lang=zh) | 动态正文读取受限，不能作为全部 API 已核验依据 |
| [火山裁切工具](https://docs.volcengine.com/docs/Intelligentprocessing/SubmitvideocuttingtaskAPI?lang=zh) | 官方索引中的异步裁切任务，部分读取 |
| [CE.SDK 视频 SDK](https://img.ly/products/video-sdk/) | 商业能力入口；未进行报价或二进制对比 |
| [Final Cut 工作流归档](https://developer.apple.com/library/archive/documentation/FinalCutProX/Conceptual/FinalCutProXWorkflowsGuide/Importing/Importing.html) | 历史 FCPXML 交换思路，不当作最新版本完整兼容承诺 |
| [AudioWorklet](https://developer.mozilla.org/en-US/docs/Web/API/AudioWorklet) | 浏览器实时音频处理边界 |
| [EBU R128](https://tech.ebu.ch/publications/r128) | 广播响度参考；不把广播目标当所有社交视频的统一默认 |

本报告交付的是可追踪的研究与推荐架构。对照产品的运行基准、商业试用、完整许可闭包审计以及本项目完整实现的实测结果均未完成，已分别进入 [验收规格](../design/video-editing/acceptance.md)。
