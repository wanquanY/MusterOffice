# MusterOffice 文档

当前职责：[ADR 0007：纯内核与薄接入](decisions/0007-kernel-only-integration-boundary.md)及[内核与接入产品边界](design/implementation/kernel-host-boundary.md)。MusterOffice 目标是提供完整 PPT 计算、渲染/无界面播放及 SDK/MCP/CLI；接入方拥有权限、持久存储、业务任务和产品页面。计算拆分、薄 CLI 与默认本地 MCP 已进入实现；完整能力和替换验收仍在实施；以下文档保留各阶段事实，不将旧标准持久宿主当作新的交付要求。

[MCP 版本与 Plugin 状态](implementation/mcp-protocol-status.md)：官方协议、实际支持版本、SDK 补丁和平台安装验收分开记录。

[扫描任务内存复用](implementation/scan-task-storage.md)：消除万个对象的重复状态分配，跨端画质/取消验证通过；局部耗时改善约 20%，剩余吞吐回退及完整替换验收继续推进。

[保留式填充扫描](implementation/scan-continuation.md)：保留边表和抗锯齿状态，填充内部可让出，跨端像素和中段取消通过；万级小对象开销回退及完整性能/替换门禁仍开放。

[有界像素准备](implementation/pixel-preparation.md)：图片预乘、背景初始化和合成快照持有游标分步处理，原生/WASM 像素不变；单个路径的同步临界区与完整替换门禁仍开放。

[TS/WASM 分步播放](implementation/wasm-stepped-playback.md)：同源待完成帧、显式生命周期、步骤间取消与真实浏览器验证；细粒度中断和完整替换门禁仍开放。

[播放帧准备与完成](implementation/prepared-playback.md)：Rust 持有待完成帧及共享图片，重新检查实例和播放状态，复用同步路径的完整像素校验；TS/WASM 的分步播放接入继续推进。

[可恢复绘图状态](implementation/stepped-raster.md)：在准备和完整绘制操作之间让出执行，原生/WASM 保持旧像素；单个 primitive 内部让出与高层播放 continuation 仍在推进。

[浏览器 Worker 接入](implementation/browser-playback-sdk.md)：Chrome/Edge 已验证现有 71 帧、资源转移与错误隔离；实测揭示逻辑取消与延迟强制中断的差异，取消性能及产品替换门禁仍开放。

[TS/WASM 播放 SDK](implementation/wasm-playback-sdk.md)：类型化播放 owner、离线运行时包与独立 Node Worker 消费；71 个实际动画帧、输入释放、线程终止和组件失效隔离已验证，完整产品替换验收继续实施。

[Native 播放 SDK](implementation/native-playback-sdk.md)：已有作者/来源计划的一次准备、多帧采样、输入释放和明确进程生命周期；独立打包消费的 71 个历史帧保持像素，完整高级内容及产品播放接入仍在实施。

[Skill/Plugin 开发包](implementation/agent-package.md)：唯一 Skill、便携/兼容清单、配套原生程序与可复现打包已实现；独立客户端搬迁后真实导出与配置复用通过，实际用户客户端安装/激活和完整发行继续实施。

[可选 HTTP 计算入口](implementation/http-mcp.md)：`2026-07-28` 无会话 HTTP 共用 SDK 与调用方文件通道，真实导出、断线取消、并发隔离及资源预算已验证；HTTP 依赖不进入默认 stdio 或嵌入 SDK，附件桥接和产品网关验收继续实施。

[本地薄 MCP 与共享文件计算](implementation/thin-mcp.md)：默认 MCP 直接消费共享 SDK，通过调用方文件桥接计算与读取结果；旧数据库宿主改为显式兼容入口。真实协议调用、并发、取消、断线及独立输出对比见阶段记录；远程/附件接入和完整替换继续实施。

[计算合同与直接 SDK](implementation/computation-boundary.md)：独立纯计算库、Native SDK 与 worker、新计算合同/Schema、独立旧 WASM 兼容模块及实际外部打包消费。全量与专项证据分开记录，权限/存储/任务/UI 保持产品责任；后续次序见[实施目标](implementation/kernel-boundary-roadmap.md)。

[直接计算入口](implementation/direct-computation.md)：SDK/CLI 共用调用与收据，调用方显式提供资源和接收输出；实际创建、导入、原子编辑、两版导出及独立 SDK 消费通过。包含文件锁生命周期修正与当时完整回归证据；本地 MCP 后续进展见上。

[Musterwork 日志内容引用索引](implementation/product-ledger-content-roots.md)：原事务中的类型化引用、SQLite v187 有界迁移、历史升级修复、真实导出及日志正文回收验证。引用目录不等于永久保留指令；业务终态回收与完整替换验收继续实施。

[Musterwork 原生宿主与暂存维护](implementation/product-execution-spool.md)：固定 SDK/worker 升级、Device 启动和原维护循环、取消后的名额保留、真实导出提交及存储回归。正式工具、业务终态回收和完整替换验收仍在实施。

[原生导出暂存恢复](implementation/execution-spool-recovery.md)：父进程/worker 共享所有权、真实进程终止与有界恢复、私有启动协议和平台依赖 SDK。该库级阶段尚未刷新产品材料，产品接入见上方后续阶段。

[Invocation 资源接入准备器](implementation/product-owned-preparation.md)：创建、编辑、导出、checkpoint 与 Tool result 强制预留，真实 Runtime/worker 回归及原事务引用保留。终态回收、暂存恢复与正式工具入口继续实施。

[Invocation 预写额度与授权](implementation/product-invocation-content.md)：SQLite v186 原子预留、持久额度及写入前后权限校验。存储端已实现，准备器强制接入与完整回收仍待完成。

[Musterwork 原生候选与 Artifact 提交](implementation/product-native-commit.md)：同一 Runtime 事务完成版本、任务与日志提交；候选 /3、恢复 /4、SQLite v185，以及实际重开、回滚和历史回归。Invocation 预写生命周期与生产工具仍待完成。

以下按阶段列出文档，各阶段保留当时的验证范围和待办；当前状态见[实现进度](implementation/progress.md)。

[Musterwork 原生导出准备](implementation/product-native-export.md)：实际 worker 计算、完整输出准入、最终存储检查、五类质量声明与取消后的资源保留；候选持久恢复和 Artifact 提交由上述后续阶段实现。

[Musterwork 真实内核创建与编辑](implementation/product-native-mutations.md)：实际 SDK 计算、精确资源闭包与读取集进入原事务；新恢复封套 /3，最终原生专项 11 项、相关库 82 项通过，生产工具及 Artifact 导出继续实施。

[Musterwork 原生草稿与同库提交](implementation/product-native-drafts.md)：明确区分原生与旧 HTML 状态，复用现有 SQLite Attempt、Tool、Ledger 和内容保留事务；新工具生产者与 Artifact 导出提交继续实施。

[Musterwork SDK 构建与内容桥接](implementation/product-sdk-bridge.md)：可从源码构建的固定 SDK、产品输入范围读取、可取消输出流与最终存储检查；真实 SQLite/worker/SQLite 交付通过，Runtime 提交及产品界面继续实施。

[Musterwork 有界验证范围读取](implementation/product-content-range.md)：完整内容校验、逐块验证、私有文件句柄、取消名额与读取后权限复查；原生 SDK 桥接及产品提交继续实施。

[Musterwork 内容文件持久回收](implementation/product-content-retirement.md)：引用撤销与回收登记同事务、分批清理、原有维护入口及真实删除中断/旧库升级验证；业务保留策略与完整产品接入继续推进。

[Musterwork 内容文件恢复与所有权](implementation/product-content-lifecycle.md)：同库物理写入登记、OS 锁、真实进程退出恢复及取消竞争保护；完整作业回收、云端和产品提交继续实施。

[Musterwork 内容存储流式接入](implementation/product-content-stream.md)：共享有界传输、桌面真实文件存储、最终字节验证与旧写入共用；后续文件恢复进展见上，云端及 Runtime 原子提交继续实施。

[Rust 嵌入 SDK 与产品开发材料](implementation/embedded-sdk.md)：同源生产库、本地可复现归档、Musterwork 固定摘要准备与实际导出验证；产品 Runtime 提交和界面仍待接通。

[Musterwork 原生产物读取](implementation/product-office-manifest.md)：实际产品 Rust/TS 版本化读取、精确依赖和五类质量声明；创作提交与 Viewer/Player 接入继续推进。

[原生 MCP stdio 接入（旧宿主兼容）](implementation/mcp-stdio.md)：历史持久宿主的工具/资源、二进制通道、取消与输出预算；保留当时协议及文件验证，当前默认入口见本地薄 MCP。

[原生持久任务调度](implementation/native-scheduler.md)：有界执行池、同库任务领取、统一续租/取消、失败恢复与持久 CLI；有真实 worker、进程重启、旧库升级和跨端验证证据。

[原生 MCP SDK 选型验证](implementation/mcp-sdk-evaluation.md)：独立锁定的双协议 stdio 实验、实际 Schema 调用、输入边界与许可记录；尚非生产适配。

[统一宿主分发与运行能力发现](implementation/operation-discovery.md)：共享 owner 接口、权限/配置感知的操作目录、Native/WASM 同源 Schema 与有效预算。

[导出作业与公开资产](implementation/export-host.md)：指定版本导出、同库原子提交、续租/取消、持久结果读取及实际宿主验证。

[演示文稿私有交付计算](implementation/presentation-delivery.md)：可编辑模型、原生 PPTX、逐页 PNG、字体/资源上下文及质量证据，作为导出宿主的计算基础。

[任务私有输出与同库生命周期](implementation/job-results.md)：执行凭据、共享配额、持久分块、取消/过期回收和实际 PPTX 校验。

[流式 PPTX 与封存结果回读](implementation/sealed-export.md)：共享 ResultSink、原生暂存、实际字节校验和 CLI 流式导出。

[持久资源与原生导出资源绑定](implementation/resource-host.md)：分块、配额预留、冻结验证、范围读取和导出素材绑定。

[持久化文档操作宿主](implementation/operation-host.md)：真实文档创建/读取/编辑、幂等任务、版本提交及取消/恢复。

[保留式时间区间复用](implementation/retained-timing.md)：单份消费事件区间、取消与代次隔离、双端工作量诊断。

[容器生命周期](implementation/container-lifecycle.md)：因果事件调度、容器结束、子树终止与自动完成。

[行为结束条件](implementation/end-conditions.md)：统一事件调度、显式结束、点击顺序与原生条件列表。

[重复次数与时长](implementation/repeat-bounds.md)：无限重复、有限结束、顺序等待和祖先截断。

[行为时间变换](implementation/time-transforms.md)：有限变速、倒放、自动往返、加减速与原生读写。

[分层时间树](implementation/timing-trees.md)：作用域时钟、并行/顺序容器、保持状态与原生读写。

此前：[来源资源播放会话](implementation/source-playback-sessions.md)：准备一次，复用解码图片和局部文字路径，采样时重算放置及精度。

此前：[原生来源页面动画](implementation/source-playback.md)：精确原生时间、对象身份与共享文字/图片/渐变绘制。

此前：[可复用播放采样会话](implementation/playback-sessions.md)：一次准备、多次采样、版本隔离与显式释放。

此前：[动画共享绘制](implementation/playback-rendering.md)：分数角度、组合放置、实际双端像素及播放状态绑定。

此前：[原生旋转时间图](implementation/timeline-rotation.md)：创建、编辑、原生时间树与精确时间求值；实际播放合成与完整动画继续推进。

此前：[渐变坐标精度](implementation/gradient-coordinate-precision.md)：共享画笔的正确舍入、刚性变换像素、三端回归、工作预算与局部性能测量。此前：[来源对象变换编辑](implementation/source-transform-edit.md)接通对象绑定、直接声明修改、继承重算、Native/WASM/CLI 及独立文件核对；它暴露的坐标差异由本阶段继续修复。此前：[来源属性编辑底座](implementation/source-attribute-overlay.md)提供编码保留和原子属性操作。

此前：[椭圆渐变性能优化](implementation/elliptic-gradient-performance.md)，不可变系数预计算、严格嵌套判定、首次根舍入验证，以及实际 PPTX 公共入口的性能测量。此前：[原生圆形渐变页面](implementation/elliptic-source-pages.md)接通路径边界、焦点与 V12 绘制。

本仓库是 MusterOffice 产品与内核设计的唯一维护位置。Musterwork 保留项目导航和自身的接入设计，不维护另一份持续演进的内核方案。

一期开发已获授权并开始实施：[实际进度](implementation/progress.md)、[开发与验证](implementation/development.md)、[运行时合同](../contracts/README.md)、[Rust 文本塑形](implementation/text-shaping.md)、[Unicode 文本边界](implementation/unicode-text.md)、[字体资源复用与候选选择](implementation/font-cascade.md)、[双向文字分析](implementation/bidi.md)、[自动文字分段](implementation/text-itemization.md)、[混合字体回退](implementation/font-fallback.md)、[Unicode 断行](implementation/line-breaking.md)、[字体实例度量](implementation/font-instance-metrics.md)、[行上下文重塑](implementation/line-shaping.md)、[显式行几何](implementation/line-geometry.md)、[自动段落布局](implementation/paragraph-layout.md)、[ADR 0006](decisions/0006-implementation-authorization.md)。

此前实现：[椭圆渐变实际绘制](implementation/elliptic-gradient-raster.md)，V12 组件和 TS 已输出真实双端像素，并传播数值失败与工作预算；Rust 场合同、来源页面及复杂场性能继续推进。此前：[椭圆渐变首次交点计算](implementation/elliptic-gradient-scalar.md)，独立有界数值核已完成三端与精确参考验证；微基准显示仍需向量化，实际页面接入继续推进。此前：[径向焦点 WPS 校准](implementation/radial-gradient-calibration.md)，实际应用观察推动焦点位置与外圆缩放分离，已接通版本化双端查询；最终着色器仍待实现。此前：[原生径向渐变几何](implementation/radial-gradient-layout.md)，真实路径边界、外圆、内椭圆及焦点可通过双端接口查询；最终像素和目标应用校准继续推进。此前：[原生矩形渐变](implementation/rectangular-gradients.md)，已接入真实 PPTX 与共享双端，支持点/线/区域焦点及窄边距；圆形/形状渐变和目标应用验收继续推进。此前：[Office 渐变插值与 JSON 浮点修复](implementation/office-gradient-interpolation.md)，已接入原生来源与双端，兼容边界及应用差异明确保留。此前：[画笔共享存储与提前预算](implementation/paint-storage-admission.md)，处理复杂页面的色标复制放大并完成实际边界、双端与局部内存验证。此前：[原生线性渐变](implementation/native-linear-gradients.md)，二维平铺与原生组合变换已接入来源页面；路径渐变及应用侧差异继续推进。此前：[共享合成与背景区域重绘](implementation/background-compositing.md)，已实现有界快照和双端合成，背景透明度与平铺兼容性仍有应用差异。此前：[组合图片填充继承](implementation/group-image-inheritance.md)，已进入实际来源页面链路，并记录应用侧差异。此前：[来源资源页面](implementation/source-resource-page.md)，整合图片、形状和文字，提供实际 Native/WASM/CLI 入口。此前库级阶段：[图片世界坐标与裁剪](implementation/image-world-paint.md)。此前共享绘制阶段：[图片参数误差](implementation/image-precision.md)。此前：[路径裁剪](implementation/shared-clips.md)。来源图片的局部计算见[原生图片局部布局](implementation/image-layout.md)。底层绘制见[图片源区域采样](implementation/image-domain.md)，物理尺寸来源见[图片分辨率](implementation/image-resolution.md)，固定解码组件见[编码图片解码](implementation/image-codec.md)。

## 建议阅读顺序

1. [产品愿景与范围](product/vision.md)：已确认的产品定位和扩展方向。
2. [整体架构](architecture/overview.md)：共享核心与办公领域模块的边界。
3. [决策清单](decisions/README.md)：已确认内容、未决事项和实现准入。
4. [一期完整演示范围决策](decisions/0002-phase-one-complete-presentations.md)：高级对象与 Musterwork 替换目标。
5. [演示文稿详细设计](design/presentations.md)：v0.4 主设计，统一目标、架构、阶段与证据待办。
6. [能力、验证与实施门槛](design/presentations/verification-and-roadmap.md)：能力目录、T/A/B 场景、E0 风险验证与评审响应。
7. [迁移资料索引](references/musterwork/README.md)：六份来源设计快照、可核对摘要及历史基线。
8. [许可证与分发状态](governance/licensing.md)：开源之前需要完成的事项。

## Agent 产品接入

- [接入总设计](architecture/agent-integration.md)：MCP、Skill、Plugin、SDK/CLI 的分工，Musterwork 和独立产品的接入形式。
- [接口规格](design/agent-interfaces.md)：工具/资源、长任务、版本与授权、Skill/Plugin 分发和 I01–I16 验收。
- [语言与组件决定](decisions/0003-language-and-component-strategy.md)：Rust 主体＋TS 薄接入＋精选 C/C++ 组件方向已确认。
- [接入架构提案](decisions/0004-agent-integration-surfaces.md)：跨产品便捷接入目标已确认，具体架构与合同为本轮设计提案。

## 演示文稿专项规格

实施入口：[一期实施规格总表](design/implementation/README.md)，包括 104 项能力计划、数据与操作、核心算法、质量/性能/兼容验收、Musterwork 源码映射及 P00–P09 工作包。机器合同与运行检查见[设计合同](contracts/README.md)；本轮设计选择见 [ADR 0005](decisions/0005-presentation-implementation-baseline.md)。

- [文档模型与 Agent 协议](design/presentations/document-model.md)
- [排版、字体与原生对象](design/presentations/layout-and-native-objects.md)
- [时间、事件与媒体](design/presentations/timeline-media.md)
- [PPTX 与 Office/WPS 互操作](design/presentations/ooxml-interoperability.md)
- [执行、资源与性能](design/presentations/runtime-performance.md)
- [Musterwork 替换合同](design/presentations/musterwork-integration.md)
- [能力、验证与实施门槛](design/presentations/verification-and-roadmap.md)

历史依据：[v0.2 归档](archive/presentations-v0.2.md)、[v0.2 设计评审](reviews/2026-09-24-presentations-design-review.md)、[来源记录](references/musterwork-baseline.md)。评审保留原结论；其问题在 v0.3 中已有设计响应，尚未取得实现/实验关闭证据。

决策评估：[Musterwork替换收益与桌面包体积估算](reviews/2026-09-24-musterwork-benefit-estimate.md)，含实际包拆账、预测假设和历史性能口径。

## 文档责任

| 内容                         | 维护位置        |
| ---------------------------- | --------------- |
| 产品名称、愿景、场景和边界   | `product/`      |
| 跨领域架构与宿主职责         | `architecture/` |
| 各领域详细设计与验收         | `design/`       |
| 机器可检查的设计合同与样例   | `contracts/`    |
| 重要决定和未决问题           | `decisions/`    |
| 设计评审、证据与修订建议     | `reviews/`      |
| 被取代的设计历史基线         | `archive/`      |
| 外部项目依据、研究和历史事实 | `references/`   |
| 许可、发行与贡献治理         | `governance/`   |

当前已有文档计算、原子编辑和 XML/OPC 文件层基础实现，完整演示内核尚未完成。技术方向已确认；设计中的协议、具体组件和性能预算仍需对应实现与验证。设计通过、实现、验收和正式发行分别记录。

主题字体选择、显式资源名称/实例清单和真实塑形探针见[原生字体绑定](implementation/native-fonts.md)。

跨段落共享验证资源、清单到布局/轮廓计算，以及真实 Native 回归见[清单布局与文字路径](implementation/manifest-layout.md)。实际 PPTX 的原生 run、样式、自动脚本/字体槽与局部字形路径连接见[来源字形编译](implementation/source-glyphs.md)；文字矩形、内边距、段落间距和对齐的实际计算见[原生文本框布局](implementation/source-frame.md)。

来源文字画笔、页面变换、遮挡顺序及真实 HarfBuzz + Skia 像素验证见[原生文字到页面像素](implementation/source-text-page.md)。

批量字体曲线与实际失败验证见[字体实例轮廓](implementation/font-outlines.md)。

实际字形路径、共享资源与有误差保证的曲线边界见[段落路径](implementation/paragraph-paths.md)。

真实 CPU 像素、固定构建与失败隔离验证见[Skia 绘制组件](implementation/skia-component.md)。

精确 Q32 到设备批次、实际像素与原生隔离入口见[路径渲染](implementation/path-raster.md)。

共享路径、变换树和原始链精度核对见[仿射场景](implementation/scene-raster.md)。

作者对象与组合坐标语义、精度验证及外部差异见[作者页面坐标](implementation/page-placement.md)。

目标应用的组变换与静态角度证据见[组合缩放/旋转兼容核对](implementation/group-placement-compatibility.md)及[静态旋转导出](implementation/static-rotation-export.md)。

作者文稿到真实 CPU 像素的入口、统一误差预算与当前支持范围见[作者页面编译与绘制](implementation/page-render.md)。

已求值世界线宽、设备 ABI 2、端点/连接与参数/像素验证见[纯色描边](implementation/stroke-raster.md)。

作者描边、同源原生 PPTX 和 WPS/LibreOffice 实际观察及差异见[作者描边](implementation/stroke-author.md)。

渐进截断的目标应用调查见[尖角兼容调查](implementation/miter-compatibility.md)；新增求值原语、当前设备 ABI 3 与回归证据见[截断尖角](implementation/miter-clip.md)。

原生对象/主题线条、样式引用和缺省限值保留及实际来源编辑验证见[来源线条](implementation/source-lines.md)。

真实 PPTX 的逐属性线条继承、显式 draft 默认值、Native/WASM 查询与独立来源校验见[线条继承](implementation/line-styles.md)；颜色数值、来源页面绘制与目标应用保真仍需继续验收。

原生线条的颜色组合、工作精度与依赖诊断、真实 Native/WASM 查询和 Decimal 验证见[线条颜色](implementation/line-colors.md)；完整画笔、来源页面编译与目标应用保真继续推进。

原生预设/自定义形状的来源记录、物理绑定、真实文件编辑保真和标准语料核对见[来源几何](implementation/source-geometry.md)；公式求值、路径编译与来源页面绘制继续推进。

真实 PPTX 的导引公式、路径坐标、手柄/连接点求值和独立高精度核对见[几何求值](implementation/geometry-evaluation.md)；预设展开、路径编译与完整页面绘制仍在推进。

原生路径空间缩放、椭圆弧编译、逐段来源和误差界及真实 Native/WASM 验证见[原生路径](implementation/native-paths.md)；完整来源页面绘制、预设展开与目标应用验收继续推进。

形状、图片、组合、线条、主题和背景的六类原生填充声明、精确词法与编辑保真见[来源填充](implementation/source-fills.md)。后续已接通[填充继承与查询](implementation/fill-styles.md)，上下文颜色、完整画笔和来源页面绘制继续推进。

对象、组合、背景、主题和图片的原生效果、平坦来源目录与空背景列表修正见[来源效果](implementation/source-effects.md)。效果图求值、完整画笔与来源页面绘制继续推进。

纯色、渐变停靠点与图案颜色的工作精度计算、按需原生占位上下文，以及 Native/WASM/Decimal 验证见[填充颜色](implementation/fill-colors.md)。这一步尚未完成空间画笔或来源页面渲染。

线性/圆形径向渐变的实际路径与场景绘制、统一浮点 profile、像素验证和局部性能基线见[渐变画笔](implementation/gradient-raster.md)。来源渐变空间计算及完整页面仍在推进。

187 种原生预设的真实 PPTX 展开、调整参数、来源合同和跨端路径/绘制验证见[预设展开](implementation/preset-expansion.md)。完整来源页面与 Musterwork 接入继续推进。

真实 PPTX 的组合子原点、零尺寸轴、占位符方向、共享坐标计算与目标应用差异见[来源页面坐标](implementation/source-placement.md)。完整来源页面和 Musterwork 替换门禁仍未完成。

真实 PPTX 的静态页面合成、原生纯色画笔、来源绑定及整页失败边界见[来源页面编译](implementation/source-page.md)。当前已连接真实页面链路，文本、图片、高级内容与 Musterwork 接入验收仍在推进。

原生文本框、段落/列表、字符、字体与画笔声明，以及真实文字编辑保留和大合同类型生成见[来源文本](implementation/source-text.md)。继承、字体绑定与来源页面文字绘制继续推进。

真实 PPTX 的文本框逐属性继承、主题文本默认项、autofit 来源和同源查询入口见[文本框继承](implementation/text-body-styles.md)。后续段落/run 的库级继承实现与独立来源校验见[文字样式计算](implementation/text-cascade.md)；字体绑定与来源文字绘制继续推进。

显式 PPTX/字体资源、Native CLI 与 WASM 文字页面接口、结构化失败及当前双端验证见[来源文字页面运行接口](implementation/text-page-runtime.md)。

缺失字族/样式、合并 run 与插入样式来源、画笔物理位置及资源补齐重试见[可行动的文字资源诊断](implementation/text-resource-diagnostics.md)。

单下划线、单删除线、字体实例度量、共享页面预算与独立像素校验见[原生文字装饰绘制](implementation/text-decorations.md)。

下划线独立纯色 / noFill、来源继承、颜色预算及跨端执行见[独立下划线画笔](implementation/underline-paint.md)。

原生基线百分比、Q32 布局输入、来源及像素验证见[基线偏移](implementation/native-baseline.md)。

原生行距与段前/段后百分比、逐行字号、首尾抑制及双端验证见[段落间距](implementation/paragraph-spacing.md)。

原生正负字距、连字策略及统一笔位的双端验证见[字符间距](implementation/character-spacing.md)。

共享图片资源、透明度、仿射/平铺画笔与完整调用链验证见[图片绘制](implementation/image-raster.md)。

真实 PPTX 的图片关系继承、嵌入/外链选择及原始资源提取见[来源图片](implementation/source-images.md)。

图片与共享场景的变换、资源复用、精度恢复及统一绘制见[场景图片](implementation/image-scene.md)。
