# MCP、Skill、Plugin 与公共操作合同

2026-09-27 · 职责按 [ADR 0007](../decisions/0007-kernel-only-integration-boundary.md) 修订：SDK/CLI/MCP 为薄计算入口，权限、持久存储、任务与产品页面由接入方负责。设计中的完整接口与示例仍待逐项验收；现已生成计算请求/调用/收据等 Schema，并由直接 SDK/CLI 和本地薄 MCP 消费，见[当前实现](../implementation/thin-mcp.md)。该计算 draft 尚未冻结发行，旧 draft 宿主 Schema 保留原语义。

2026-09-28，模板描述与实例化已纳入同一计算调用及收据，详见[模板共享协议](../implementation/template-protocol.md)。Musterwork 候选 Agent 已接入其原目录读取、固定版本和独立实例提交，见[产品模板调用](../implementation/product-native-templates.md)。下表完整工具面仍是目标设计，模板生产、页面组装和完整替换继续实施。

已有[原生 MCP](../implementation/mcp-stdio.md)、[持久调度](../implementation/native-scheduler.md)、[TS 客户端](../implementation/operation-client.md)等阶段实现与证据；其中标准宿主/资源库属于旧方案，已隔离到显式旧版 MCP 兼容入口；默认本地入口不依赖它们。后续收敛须保留文稿计算及真实文件验证，不能把历史通过称为新边界已完成。

## 1. 标准采用与版本边界

SDK、MCP、CLI、Skill 和 Plugin 共用同一计算合同。既有阶段分别验证过 2026-07-28 与 2025-11-25 MCP 档案；发布时仍须绑定实际支持的协议和客户端构建，不能混用两代消息，也不能推断所有客户端均支持同一扩展。

| 层 | 必需或条件 | 责任 |
| --- | --- | --- |
| MCP Tools / Resources | 基线 | 操作发现/调用和有界投影；资源读取委托宿主桥接，不创建内容库 |
| stdio / Streamable HTTP | 两种适配均属一期 | 本地进程或远程计算；远程认证/网络策略归部署网关和接入产品 |
| MCP Tasks | 有宿主持久任务桥接时声明 | 映射同一产品任务；不由 MusterOffice 新建 JobStore |
| Agent Skills / Skills extension | 文件技能为基线，扩展按协商 | 工作流与说明，不授予权限、不保存业务状态 |
| MCP Apps / Viewer UI | 产品自行选择 | 不作为 MusterOffice 交付条件；保留完整无界面渲染/播放能力 |
| Plugin | 按平台包装 | 安装组件、注册 MCP/Skill、声明宿主需要授予的访问范围 |

操作、文档、绑定 ABI、协议、Skill 和 Plugin 分别版本化。身份、权限和持久任务合同从计算合同移出须使用明确版本，不能直接重解释已有 request/receipt 或关闭校验。

## 2. Operation Contract

公共输入包含文档/来源数据、显式资源、版本前提、操作和参数；可信计算上下文只有期限、预算、取消、进度及输入输出能力。接入产品在调用前授权，在调用后保存和提交；其 principal、tenant、scope、权限枚举、JobStore、fence 不成为内核必填参数。

| 字段/概念 | 计算合同 |
| --- | --- |
| contractVersion | 固定操作语义，与 MCP 协议版本分离 |
| requestId | 一次逻辑调用的关联身份；不凭此承诺跨进程持久幂等 |
| document / source / resources | 输入数据或调用方可解析的读取句柄，附必要身份与摘要 |
| documentId / baseRevision | 稳定文档身份和编辑前提；持久 head 由产品提供/提交 |
| profileId | 显式质量与资源档案，各入口不能降低质量 |
| arguments | 严格操作 Schema，批量编辑整体校验/成功/失败 |
| execution options | 计算预算、取消/进度、输出接收器；无自有队列配置 |

核心返回计算 outcome、文稿/revision、变更集、输出描述与完整依赖、诊断、版本及资源用量。计算成功与产品保存/发布成功分别报告。生成的实际 PPTX、预览和诊断输出需要完成规定检查，不能提前宣告成功。

宿主需要异步持久任务时，桥接层可增加 job、retention 和提交回执；只有宿主确认持久接收后才可返回此意义的 accepted。跨调用/进程幂等、取消与提交竞争由宿主保证。普通 SDK/MCP 不需要 Job 服务，进程内计算句柄不能伪装成持久 Job。

现有 [v0.4 设计 Schema](../contracts/README.md)和下方旧 draft 示例保留为迁移对照，不代表已完成上述拆分。正式 Schema 应从同一计算合同生成 SDK 类型、MCP input/outputSchema、CLI 与校验器；宿主扩展单列，不能复制一套领域规则。

## 3. MCP 工具面

候选名称使用短前缀 `mo_`，实际宿主可能再添加连接器前缀。Skill 通过宿主工具映射绑定来源身份，不仅依赖同名工具。工具列表按声明模块稳定排序，产品在授权后决定向 Agent 暴露的集合；不按任意先前 tool call 暗中改变工具定义。

| 候选工具 | 输入与结果 | 关键约束 |
| --- | --- | --- |
| mo_capabilities | 协议/操作/模块/宿主/资源能力与限额 | 明确未实现、不可用和未验证；不返回用户文稿 |
| mo_assets_prepare | 类型/长度/摘要 → 宿主数据通道描述 | 可选资源桥接；不创建内部上传存储或凭 URL 自动联网 |
| mo_assets_register | 宿主输入/附件/读取句柄 → AssetRef | 权限由宿主先检查；适配器核对类型、摘要与句柄，不创建内容库 |
| mo_presentations_create | 类型化初始内容/模板 → 文稿/revision | 支持最小有用初稿；不只是返回空壳 |
| mo_presentations_open | PPTX AssetRef → 文稿/导入报告或 job | 原生理解、保留与缺失分别报告 |
| mo_presentations_query | 对象/页/时间/数据选择器、cursor → 有界投影 | 受限查询；不执行任意脚本或数据库表达式 |
| mo_presentations_apply | baseRevision、原子操作集 → 新 revision/差异 | 覆盖文字/图表/表格/Diagram/Math/时间/媒体，统一引用检查 |
| mo_presentations_validate | revision、检查档案 → 诊断/证据或 job | 结构、视觉、编辑和动态检查分项 |
| mo_presentations_render | 页/对象/明确播放状态 → 预览资源或 job | 限定数量和尺寸；不是实时帧流 |
| mo_presentations_export | revision、目标档案 → 实际 PPTX/报告或 job | 包含写出和实际字节重读，候选通过才允许提交 |
| mo_templates_query | 模板/参数/容量 → 有界结构 | 独立模板资源，无 Musterwork 服务依赖 |
| mo_templates_instantiate | 模板版本/参数 → 新文稿/revision | 保留引用、时间与数据关系 |
| mo_templates_validate | 模板/压力方案 → 报告或 job | 宿主控制任务；不启动模型循环 |
| mo_jobs_get | 宿主 jobId → 状态、进展、终态结果/错误 | 仅在宿主任务桥接可用时暴露；权限与保留期归产品 |
| mo_jobs_cancel | 宿主 jobId → 取消受理/已终态 | 仅限宿主任务桥接；不伪称已经释放，提交竞争见第 5 节 |
| mo_resources_release | 明确的临时 lease IDs → 释放结果 | 不删除用户已提交文件；重复释放幂等 |

创建/导出不调用另一家模型来“自动补内容”；Skill/Agent 负责内容与设计选择。高层布局预设可以展开为同一领域事务。多个对象修改一次 apply 提交，避免 Agent 为每个字形进行一次工具往返。

MCP 封装提供 outputSchema、structuredContent 和必要的简短可读信息；为兼容需要时提供有界 JSON 文本镜像。图片只作为少量必要证据，完整文件返回资源链接。[MCP Tools](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)

工具 annotations 描述只读/破坏性/幂等等行为，但不承担权限控制。输出结构中的文稿文本必须作为数据呈现，不能提升为 system 指令或改变 Skill。参数/方法层错误与工具执行失败按对应 MCP 版本分开封装；业务诊断保留可机器处理的 code/path/retry 条件。

持久 job 工具属于宿主扩展，不能让基本创建、编辑、渲染和导出依赖这些工具。计算会话的进度/取消独立于持久业务任务。

## 4. 文件、资源与传输

内核消费不可变字节/ReaderAt，输出写入调用方 ResultSink；简单内存和流用例使用公共适配，不要求自建内容数据库。AssetRef、摘要和逻辑 URI 只描述输入身份，不能授予文件或网络访问权。文档 ID 不隐含 MusterOffice 中存在一份持久文稿。

| 环境 | 输入/输出来源 |
| --- | --- |
| 本地 SDK / CLI / stdio | 调用方提供的数据、明确文件/句柄、输入输出流；文件打开由宿主/薄 I/O 桥接负责 |
| 产品内嵌 | 原附件/内容存储映射为读取句柄，输出回到产品接收器和提交事务 |
| 远程 MCP | 产品/部署网关提供的附件、数据端点或读写桥接；MusterOffice 不建设上传页面或文件服务 |

大 PPTX、字体和音视频通过有界二进制数据通道传输，按声明能力提供范围读取、背压和输出回读；不反复编码进工具上下文。需要暂存时使用宿主提供的受限临时 I/O，宿主负责持久空间及崩溃后回收。

PPTX 外链、备注和工具参数中的路径/URL 都不能让核心自行取数。缺少资源时返回明确依赖/诊断，由宿主处理授权及获取。对于没有合适数据通道的客户端，如实声明可支持的输入输出范围，不暗中引入账号、网盘或上传 UI。

## 5. 计算、进度、恢复和取消

基本调用是对输入文稿的计算，可同步返回或保持当前调用接收进度。文稿/资源缓存与播放会话均为有界进程内状态，有明确释放和失效条件；断线如何处理当前计算由调用合同声明，不能把连接丢失等同于持久成功。

| 能力 | 所有者与保证 |
| --- | --- |
| 当前计算进度/取消 | MusterOffice 检查取消、停止后续计算并释放本次资源；宿主可终止隔离 worker |
| 持久任务接收/查询/重试 | 接入产品的任务服务；薄 MCP 只映射其已有状态 |
| 恢复输入/计算状态 | 核心提供可校验的序列化材料；产品决定保存、重载与任务恢复 |
| 幂等发布/版本 CAS | 产品原事务；内核完成原子编辑和版本前提检查 |
| 结果保留、过期和回收 | 接入产品；核心不能扫描产品数据库推断全局引用 |

无任务桥接时不公布持久 Tasks/Job 能力。进程重启后，过期会话句柄明确失效，宿主重载文稿或恢复自己的任务；不能返回永久 running 或伪造完成状态。支持任务桥接的独立验收使用接入方自己的持久设施，而不是 MusterOffice 自有 SQLite。

取消后的晚结果仍为计算候选；宿主必须重新核对当前执行权和提交条件。控制消息与数据通道都需要有界行为，实际系统调用中的不可即时取消部分应在验收中测量。

## 6. 合同示例与端到端调用

以下保留旧 draft 的**宿主持久任务桥接示例**用于迁移对照，不是普通 SDK 的必填接口，也不表示 MusterOffice 自有任务数据库。仅在接入产品提供相应任务服务时适用；不展示 MCP wire envelope。

```json
{
  "contractVersion": "musteroffice.operations/1-draft",
  "requestId": "export-007",
  "documentId": "deck-42",
  "baseRevision": "revision-8",
  "profileId": "presentations-full-proposed",
  "outputMode": "job",
  "arguments": {"format": "pptx"}
}
```

```json
{
  "contractVersion": "musteroffice.operations/1-draft",
  "requestId": "export-007",
  "outcome": "accepted",
  "documentId": "deck-42",
  "revision": "revision-8",
  "job": {"id": "job-19", "state": "queued", "pollAfterMs": 1000},
  "artifacts": [],
  "diagnostics": []
}
```

此时不能对用户说“PPT 已生成”。完成后 get 返回 outcome=succeeded、实际 PPTX/报告引用、字节摘要及有效期；失败包含可定位诊断。工具示例中的 ID 和 profile 均为占位值，不能用于声明兼容档案存在。

最小闭环为：能力发现 → 登记素材/源文件 → 创建或打开 → 查询 → 原子编辑 → 检查/指定状态预览 → 导出（有宿主任务服务时可查询）→ 取回 PPTX。外部产品只需理解这些操作与文件入口，不需要重新实现字体、排版、质量检查或解包逻辑。

## 7. Agent Skill 的内容与边界

采用 Agent Skills 的目录与 SKILL.md 形式，主文件包含 name/description 和工作流，详细图表、动画、模板、编辑及检查说明放到 references 按需读取。技能内容尽量小，字体、媒体与内核二进制不塞进技能目录。[Agent Skills 规范](https://agentskills.io/specification)

建议发行结构：

```text
musteroffice-presentations/
  SKILL.md
  references/creation.md
  references/editing.md
  references/animation-media.md
  references/quality-and-delivery.md
  references/tool-bindings.md
```

技能规定：先获取能力和已有资源；按用户意图创建或导入；使用批量事务；检查内容/布局/动态状态；根据诊断修复；导出真实文件并报告验证范围。Office/WPS 未现场打开的文稿不得因 Skill 文本宣称已经认证。

MCP 可用时优先使用它；只有宿主明确提供 CLI 绑定时走 CLI，不能自行下载执行不明二进制或猜包名。CLI 和 MCP 使用同一逻辑请求及输入版本；已有宿主任务时，跨入口重试先查询该任务，避免重复业务提交。

一个发布版本只维护一份 Skill 内容；文件包、MCP Skills 和平台 Plugin 引用相同摘要。支持 MCP Skills 的客户端可用其发现接口和 Resources 读取内容，读取文件与实际激活技能由宿主区分。扩展支持和基本 MCP 连接成功分开报告。[MCP Skills](https://modelcontextprotocol.io/extensions/skills/overview)

技能不内嵌模型凭据、联网服务或私有产品路径，也不复制 Schema 成为第二份权威。模型可能不遵循技能步骤，因此计算验证由内核强制执行；权限与业务提交条件由接入产品强制执行。

## 8. Plugin 发行合同

公共 Release Manifest 描述版本、平台/架构、下载摘要/签名、组件及许可证、所需资源、支持的操作/MCP/文档版本、Skill 摘要、默认权限、安装/升级/卸载和兼容测试记录。这个 manifest 是本项目发行元数据，不宣称是 MCP 或 Agent Skills 官方格式。

平台包装只把上述清单映射成目标产品的 MCP 配置、技能注册与访问范围声明；权限 UI 和 viewer 由产品自己实现。原生客户端可安装对应二进制，远程客户端可配置自托管 URL；不为每个平台重新实现办公计算或携带另一套大型 JS 引擎。

安装先校验宿主/平台/组件完整性，注册能力，再做有界健康检查；权限按实际功能声明。升级声明计算合同/文稿格式兼容范围，资源与 Job 数据迁移由接入方管理；卸载只移除注册和安装组件，不删除接入产品的文稿或业务存储。禁止 install 脚本默认上传文稿、启动第二个 Agent 或偷偷开启全盘访问。

首批必须交付 Musterwork 官方适配及通用 MCP/Skill/CLI 资产。其他平台包装按明确的客户端版本逐个验证；不存在“一份 Plugin manifest 通吃所有 AI 产品”的承诺。框架不支持 Plugin 时仍能直接配置 MCP 或安装 Skill。

## 9. 无界面渲染/播放与产品 UI

MusterOffice 提供文稿页模型、渲染帧/绘制结果、动画/媒体时间语义、播放/暂停/seek、页切换与诊断定位接口。接入产品实现查看器、编辑器、播放器页面、工具栏和媒体权限提示。内核不以 DOM 或另一套 UI 排版重新解释 PPTX。

MCP Apps 是接入产品可以选择的展示机制，不要求 MusterOffice 交付成品 Apps/viewer。没有 UI 扩展的客户端仍能通过工具生成、修改、采样预览并取得实际 PPTX。取消产品 UI 交付不减少无界面动态播放和媒体能力的验收。

连续帧、媒体 bytes、音频时钟和 GPU 资源通过 SDK/宿主数据通道。Agent 工具设置播放内容或请求指定状态采样，不能让每帧触发模型或网络工具调用。

## 10. 宿主授权与内核输入防护

身份、租户、权限、凭据和远程服务认证归接入产品/部署网关。MusterOffice 不提供账号、角色、ACL、OAuth 服务或业务权限数据库。宿主在开放工具及交付资源句柄前授权，拒绝未经允许的读取/写入/网络请求；计算层不再定义一套平行权限模型。

此边界不撤销输入防护：内核必须检查格式、解压/解码预算、摘要、句柄和版本有效性，隔离不可信计算，尊重取消和输出失败。宿主输入/输出桥接承担真实路径、网络及业务资源的访问控制；内容中的文字指令不能改变这些访问范围。

跨用户/租户隔离在接入产品的集成测试验证。缓存由明确计算会话持有，错误与进度不返回另一调用的内容；日志和性能事件不携带凭据或文稿正文。

## 11. I01–I20 接入验收

I01–I16 保留编号并按 ADR 0007 调整责任归属；旧宿主/协议阶段证据保持原范围，不自动证明修订后的验收。I17–I20 增加易用 SDK、发行依赖、实际开销和计算生命周期要求。

| ID | 必须证明 |
| --- | --- |
| I01 | 干净环境配置本地 MCP 及支持的宿主输入输出方式，完成创建/编辑/渲染/导出；不需要 Musterwork、自有账号、数据库或页面 |
| I02 | 远程薄 MCP 通过接入方网关/数据通道完成相同链路；认证、持久文件与任务在宿主，客户端不需本地引擎 |
| I03 | 已声明 MCP 档案分别协商/封装/失败，无混用消息或错误能力声明 |
| I04 | 无 Tasks/Skills/Apps 扩展的客户端通过普通调用完成创作和实际 PPTX 交付 |
| I05 | 可选任务桥接复用产品持久 Job，验证重连、取消/提交竞争和幂等；无桥接时不声明持久能力 |
| I06 | Skill 文件、可选 Skills 分发和 Plugin 引用同一技能摘要，来源/版本准确 |
| I07 | Skill＋CLI 独立调用同源内核，无隐藏 Node.js、数据库或 UI 框架必需项 |
| I08 | SDK/MCP/CLI 同输入和质量档案的语义、视觉、原生编辑和动态结果等价，不删减高级内容 |
| I09 | 大字体/媒体/PPTX 使用有界数据通道，测量实际复制量、桥接内存及延迟 |
| I10 | 产品授权拒绝在进入计算前生效；路径/URL/内容指令不能越过输入桥接；内核预算/格式防护保留 |
| I11 | Musterwork 消费公共接口，原 Runtime 拥有任务/权限/存储/提交；fence、历史版本与实际资源安全正确 |
| I12 | Web Worker、Desktop 与 Managed 使用同源渲染/播放计算，产品 UI 独立；关闭前台按宿主规则处理任务 |
| I13 | Plugin 安装/升级/卸载和组件完整性有效，不创建业务库，不删除产品文稿；离线资源按档案完整 |
| I14 | 预览与完整动态播放计算不依赖 MusterOffice 页面或 MCP Apps；产品 UI 可消费实际输出 |
| I15 | 独立第三方 MCP Client 无需修改核心源码或数据库即可完成全链路，记录所用文件/附件桥接和限制 |
| I16 | 同源 Schema、SDK 类型、Skill、Plugin 配置和版本清单一致；计算合同与宿主扩展分离，旧格式不被重解释 |
| I17 | 高层 SDK 示例只需初始化、提供输入、调用、接收结果和释放；无必填 JobStore/账号/权限/业务提交器 |
| I18 | 实际依赖图及发行物无强制自有持久宿主/权限/UI；按入口装配且一期能力、字体/codec 与离线闭包未缩水 |
| I19 | 同 build/设备/文稿/资源/质量下比较直接调用、SDK、本地及适用远程 MCP；绝对预算和事先固定的封装开销门槛均满足 |
| I20 | 会话/缓存/临时 I/O 所有权明确；取消、输出失败、进程结束及宿主重载正确，无隐式持久库或未声明后台回收服务 |

本修订未执行 I01–I20 验收。既有部分实现见[进度](../implementation/progress.md)，完整边界与源码收敛见[实施合同](implementation/kernel-host-boundary.md)。性能报告包含完整交付、全部进程和依赖，不以空 ping、静态样例或单个 worker 体积代替。
