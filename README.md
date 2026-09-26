# MusterOffice

**面向 Agent 的轻量、高性能、跨平台办公内核。**

MusterOffice 为 Agent 提供结构化的创建、查询、编辑、渲染、导出和验证能力。演示文稿是首个设计与验证方向，未来可以扩展到文档、电子表格及其他办公场景。

产品提供 MCP、Agent Skill、平台 Plugin、SDK/CLI 与可嵌入核心的接入设计，不绑定特定模型、Agent 框架或业务平台。各入口共用一份操作合同和同一内核。最终交付物供人阅读和编辑，质量、可编辑性与互操作能力是核心要求。

## 当前状态

最近完成：[原生 MCP stdio 接入](docs/implementation/mcp-stdio.md)将 13 个工具、授权资源与二进制通道接入同一持久宿主。release 的两代协议完成 167 次实际请求、两次真实导出及 24 份资源回读；独立文件/像素检查、十项适配器测试和 61 项宿主回归通过。完整协议/生态接入、高级内容及 Musterwork 替换验收继续推进。

最近完成：[原生持久任务调度](docs/implementation/native-scheduler.md)已将同库任务接入有界后台执行池，并统一创建/编辑/导出的续租和取消检查。837 项全仓回归、最终 61 项宿主回归及 11 项显式真实 worker 集成通过；实际 CLI 自动导出、进程终止重启、忙碌 renderer 下的控制、旧库升级和文件/页面回归已有阶段证据。正式 Agent 接入、完整高级内容和 Musterwork 替换验收仍未完成。

最近完成：[统一宿主分发与能力发现](docs/implementation/operation-discovery.md)将操作路由从 SQLite 宿主移至纯服务，提供权限/配置感知的操作目录及同源 Schema 查询。826 项全仓 Rust 回归、10 项显式真实 worker 集成、109 份合同、10 份实际 Native/WASM Schema 对照，以及历史数据/导出/页面回归通过；完整 Agent 和 Musterwork 接入继续实施。

此前：[导出作业与公开资产](docs/implementation/export-host.md)连接指定版本、持久任务、完整候选与同库原子发布。821 项 Rust 回归、9 项显式真实 worker 集成、107 份合同、31 次 release CLI 调用、两份旧库迁移及 72 次双端调用通过；完整高级内容、Agent 接入和 Musterwork 产品提交仍未完成。

此前：[演示文稿私有交付计算](docs/implementation/presentation-delivery.md)连接可编辑模型、PPTX、真实逐页 PNG、显式资源/字体及质量证据，并修复实际链路发现的默认文字样式导出缺陷。该轮 819 项 Rust 回归、2 项显式原生集成、107 份合同和实际两页 Native/WASM 对照通过。

此前：[任务私有输出](docs/implementation/job-results.md)连接标准宿主执行凭据、同库分块、共享配额和取消/过期回收；完整公开导出、交付清单与产品接入继续推进。

此前：[流式 PPTX 与封存结果回读](docs/implementation/sealed-export.md)连接共享 ResultSink、原生文件暂存和真实 CLI；完整导出作业、交付清单与产品接入继续推进。

此前：[持久资源与原生导出资源绑定](docs/implementation/resource-host.md)接通分块传输、字节预留、冻结验证、范围读取和既有 PPTX Writer 的授权素材读取。完整导出作业、交付清单与产品接入继续推进。

此前：[持久化文档操作宿主](docs/implementation/operation-host.md)接通创建、读取、原子编辑、幂等任务和 SQLite 版本提交；已有多连接竞争及真实进程终止/重启验证。

此前：[保留式时间区间复用](docs/implementation/retained-timing.md)完成共享内核缓存、播放代次隔离及实际会话成本验证；完整一期和 Musterwork 替换验收继续推进。

此前：[容器生命周期](docs/implementation/container-lifecycle.md)接通容器结束、父子事件和子树终止；原生读写、实际页面、历史回归及局部查询成本已有证据，完整一期及 Musterwork 替换验收继续推进。

此前：[行为结束条件](docs/implementation/end-conditions.md)接通点击／事件结束、顺序后继与原生读写，并统一平面／分层时间调度。

此前：[重复次数与时长](docs/implementation/repeat-bounds.md)接通无限重复、指定时长结束和父级截断下的反向采样。

此前：[来源资源播放会话](docs/implementation/source-playback-sessions.md)接通一次准备、局部文字/图片资源复用和 Native/WASM 显式 owner；完整一期及 Musterwork 替换验收继续推进。

仓库已进入**一期实现阶段**。用户已授权持续落实完整方案，直到具备启动 Musterwork PPT 替换验收的条件。Rust 文档计算、原子编辑候选、OPC 文件层、基础原生 PPTX 导出、来源索引与保留式文本修改已有实现，原生 CLI/WASM 共用核心及类型生成；尚无完整可安装的演示内核，不能替换产品。

- 已确认：名称 MusterOffice、独立项目、Agent 原生、轻量高性能、跨平台、可扩展办公场景、计划开源。
- 一期目标已确认：完整演示文稿内核，包含动画/转场、音视频、SmartArt、公式，并替换 Musterwork 的 PPT 能力；v0.4 已形成实施规格、104 项能力计划与设计合同校验，仍待实现与验收。
- 技术方向已确认：Rust 主体、原生/WASM 目标、TS 薄接入，允许经验证的 C/C++ 底层组件；不为语言纯度牺牲质量和性能。
- 基础实现依赖已锁定并登记；字体/绘制/媒体组件、实际兼容构建、公开许可证和正式发行方式仍需相应证据。
- 接入目标已确认：Musterwork 与其他 AI 产品可以标准化接入；MCP/Skill/Plugin/SDK 的具体合同已形成设计稿，尚无可安装包。
- 未配置远程仓库或公开发布，也没有将 Musterwork 产品代码复制进来。

## 从这里开始

实际进度见[实现状态](docs/implementation/progress.md)，构建和测试见[开发说明](docs/implementation/development.md)，本次授权见 [ADR 0006](docs/decisions/0006-implementation-authorization.md)。

此前推进[原生来源页面动画](docs/implementation/source-playback.md)：原生时间树驱动共享文字、图片和渐变绘制，保持源文件和对象身份；本阶段进一步实现资源会话，完整播放宿主与产品接入继续推进。

此前推进[可复用播放采样会话](docs/implementation/playback-sessions.md)：共享计划由 Rust/WASM 与 Native 常驻进程持有，严格绑定版本和 generation；完整宿主、媒体合成与产品接入继续实施。

此前推进[动画共享绘制](docs/implementation/playback-rendering.md)：精确分数旋转进入同一对象放置和 Native/WASM 页面绘制链路；文档保持不变，完整播放合成与产品接入继续推进。

此前[原生旋转时间图](docs/implementation/timeline-rotation.md)：接通文档模型、原子编辑、原生 PPTX 时间树和确定性属性求值；实际播放合成、完整效果及 Office/WPS 互操作继续推进。

此前[渐变坐标精度](docs/implementation/gradient-coordinate-precision.md)：共享画笔直接验证坐标舍入，修复[来源对象变换编辑](docs/implementation/source-transform-edit.md)暴露的平移、翻转和直角旋转差异，并核对实际双端像素、精确计算预算及性能开销。结构编辑、完整高级内容与 Musterwork 接入继续推进。

此前[椭圆渐变性能优化](docs/implementation/elliptic-gradient-performance.md)已接入实际页面。在明确测试条件下，两类嵌套椭圆底层绘制约快 5 倍，选定真实 PPTX 页面 API 的收益为约 1.0–4.0 倍；Skia WASM 增加 9,159 字节。完整高级内容、目标应用互操作和 Musterwork 接入仍未完成。

| 文档                                                    | 内容                                           |
| ------------------------------------------------------- | ---------------------------------------------- |
| [产品愿景与范围](docs/product/vision.md)                | 面向谁、解决什么问题、办公场景扩展边界         |
| [整体架构](docs/architecture/overview.md)               | 共享能力、领域模块、格式适配和宿主边界         |
| [Agent 生态接入](docs/architecture/agent-integration.md) | MCP、Skill、Plugin、SDK 分工及 Musterwork/独立产品接入 |
| [公共接口与分发合同](docs/design/agent-interfaces.md) | 工具、资源、异步任务、技能/插件、版本与接入验收 |
| [演示文稿设计评审](docs/reviews/2026-09-24-presentations-design-review.md) | 完整一期目标下的设计缺口、架构与迁移建议 |
| [演示文稿详细设计](docs/design/presentations.md)        | v0.4 总体方案与七份领域规格，覆盖完整一期设计 |
| [一期实施规格](docs/design/implementation/README.md) | 能力、数据/操作、算法、验收档案、Musterwork 实际字段与实施顺序 |
| [设计合同与校验](docs/contracts/README.md) | 104 项能力计划、5 份 JSON Schema、合成正反例及离线校验 |
| [迁移资料与历史基线](docs/references/musterwork/README.md) | 六份 Musterwork 设计快照、来源摘要与 v0.2 归档 |
| [决策与评审状态](docs/decisions/README.md)              | 已确认事项、待决策事项和进入开发的条件         |
| [项目身份决策](docs/decisions/0001-project-identity.md) | MusterOffice 命名、独立项目及扩展定位          |
| [一期范围决策](docs/decisions/0002-phase-one-complete-presentations.md) | 完整演示能力与 Musterwork 替换目标 |
| [语言与组件策略](docs/decisions/0003-language-and-component-strategy.md) | 已确认技术方向及后续选型边界 |
| [许可证与分发状态](docs/governance/licensing.md)        | 开源意图、尚未选择的许可证及迁出规则           |
| [贡献与工程规范](CONTRIBUTING.md)                       | 设计、测试、质量与版本纪律                     |
| [Agent 工作规范](AGENTS.md)                             | 本仓库内执行工作的约束                         |

## 产品原则

1. **Agent 原生**：结构化接口、稳定对象 ID、原子操作、可复现结果与可定位诊断。
2. **轻量和高性能**：测量完整依赖、字体、冷下载、峰值内存和持续运行成本。
3. **质量与可编辑性**：内容完整、对象可编辑，实际文件与目标应用分别验证。
4. **跨平台与可嵌入**：计算规则共用，文件、网络、权限和执行隔离由宿主负责。
5. **按办公领域扩展**：共享稳定的基础设施，保留演示文稿、分页文档和表格计算各自的模型。
6. **自主可控**：运行不依赖商业 SDK 采购或厂商在线许可验证。

完整文档导航见 [docs/README.md](docs/README.md)。本仓库目前未授予开源许可证，公开分发前需完成许可证和权属确认。

批量字体曲线与实际失败验证见[字体实例轮廓](docs/implementation/font-outlines.md)。

实际字形路径、共享资源与有误差保证的曲线边界见[段落路径](docs/implementation/paragraph-paths.md)。

实际路径编译、Native/WASM 像素与隔离发布见[路径渲染](docs/implementation/path-raster.md)。

共享几何、仿射组合与精度保证见[场景渲染](docs/implementation/scene-raster.md)。

作者对象与组合坐标语义、精度验证及外部差异见[作者页面坐标](docs/implementation/page-placement.md)。

目标应用的组变换与静态角度证据见[组合缩放/旋转兼容核对](docs/implementation/group-placement-compatibility.md)及[静态旋转导出](docs/implementation/static-rotation-export.md)。

作者文稿到真实 CPU 像素的入口、统一误差预算与当前支持范围见[作者页面编译与绘制](docs/implementation/page-render.md)。

已求值线宽、端点、连接和真实像素验证见[纯色描边](docs/implementation/stroke-raster.md)。

作者描边到页面像素及原生 PPTX 已接通；WPS/LibreOffice 的尖角差异保留为未关闭项，详见[作者描边](docs/implementation/stroke-author.md)。

原生对象/主题线条、样式引用和缺省限值保留及实际来源编辑验证见[来源线条](docs/implementation/source-lines.md)。

真实 PPTX 的逐属性线条继承、显式 draft 默认值、Native/WASM 查询与独立来源校验见[线条继承](docs/implementation/line-styles.md)；颜色数值、来源页面绘制与目标应用保真仍需继续验收。

原生线条的颜色组合、工作精度与依赖诊断、真实 Native/WASM 查询和 Decimal 验证见[线条颜色](docs/implementation/line-colors.md)；完整画笔、来源页面编译与目标应用保真继续推进。

原生预设/自定义形状的来源记录、物理绑定、真实文件编辑保真和标准语料核对见[来源几何](docs/implementation/source-geometry.md)；公式求值、路径编译与来源页面绘制继续推进。

真实 PPTX 的导引公式、路径坐标、手柄/连接点求值和独立高精度核对见[几何求值](docs/implementation/geometry-evaluation.md)；预设展开、路径编译与完整页面绘制仍在推进。

原生路径空间缩放、椭圆弧编译、逐段来源和误差界及真实 Native/WASM 验证见[原生路径](docs/implementation/native-paths.md)；完整来源页面绘制、预设展开与目标应用验收继续推进。

形状、图片、组合、线条、主题和背景的六类原生填充声明、精确词法与编辑保真见[来源填充](docs/implementation/source-fills.md)。后续已接通[填充继承与查询](docs/implementation/fill-styles.md)，上下文颜色、完整画笔和来源页面绘制继续推进。

对象、组合、背景、主题和图片的原生效果、平坦来源目录与空背景列表修正见[来源效果](docs/implementation/source-effects.md)。效果图求值、完整画笔与来源页面绘制继续推进。

纯色、渐变停靠点与图案颜色的工作精度计算、按需原生占位上下文，以及 Native/WASM/Decimal 验证见[填充颜色](docs/implementation/fill-colors.md)。这一步尚未完成空间画笔或来源页面渲染。

线性/圆形径向渐变的实际路径与场景绘制、统一浮点 profile、像素验证和局部性能基线见[渐变画笔](docs/implementation/gradient-raster.md)。来源渐变空间计算及完整页面仍在推进。

187 种原生预设的真实 PPTX 展开、调整参数、来源合同和跨端路径/绘制验证见[预设展开](docs/implementation/preset-expansion.md)。完整来源页面与 Musterwork 接入继续推进。

真实 PPTX 的组合子原点、零尺寸轴、占位符方向、共享坐标计算与目标应用差异见[来源页面坐标](docs/implementation/source-placement.md)。完整来源页面和 Musterwork 替换门禁仍未完成。

真实 PPTX 的静态页面合成、原生纯色画笔、来源绑定及整页失败边界见[来源页面编译](docs/implementation/source-page.md)。当前已连接真实页面链路，文本、图片、高级内容与 Musterwork 接入验收仍在推进。

原生文本框、段落/列表、字符、字体与画笔声明，以及真实文字编辑保留和大合同类型生成见[来源文本](docs/implementation/source-text.md)。继承、字体绑定与来源页面文字绘制继续推进。

真实 PPTX 的文本框逐属性继承、主题文本默认项、autofit 来源和同源查询入口见[文本框继承](docs/implementation/text-body-styles.md)。后续段落/run 的库级继承实现与独立来源校验见[文字样式计算](docs/implementation/text-cascade.md)；字体绑定与来源文字绘制继续推进。

文字资源缺失与原生 run 定位现已提供[可行动诊断](docs/implementation/text-resource-diagnostics.md)，并验证显式补齐资源后的实际双端重绘。

单下划线和单删除线已接入真实字体度量与页面绘制，当前支持范围与独立验证见[原生文字装饰](docs/implementation/text-decorations.md)。

原生 `uFill` 的独立颜色、noFill、继承和资源诊断已进入同一绘制链路，最新验证见[独立下划线画笔](docs/implementation/underline-paint.md)。

原生正负 `baseline`、精确百分比及样式合并误差已接入布局和页面绘制，见[基线偏移实现](docs/implementation/native-baseline.md)。

原生行距、段前/段后百分比、逐行字号及零值间距已接入双端绘制，见[段落间距实现](docs/implementation/paragraph-spacing.md)。

原生正负字距、字簇边界及断行/装饰线的统一笔位见[字符间距实现](docs/implementation/character-spacing.md)。

共享图片资源、仿射/平铺画笔、透明边缘处理及双端运行接口见[图片绘制](docs/implementation/image-raster.md)。真实 PPTX 的继承关系、包内资源提取和外链选择已接通[来源图片](docs/implementation/source-images.md)；解码、原生画笔编译和完整产品接入继续推进。

图片已接入[共享场景变换与绘制](docs/implementation/image-scene.md)，沿用路径复用、精度恢复和同一个后端；实际 PPTX 图片页面仍需完成解码及原生填充编译。
