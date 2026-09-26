# MCP、Skill、Plugin 与公共操作合同

最新实现：[原生 MCP stdio](../implementation/mcp-stdio.md)已把现有 draft 合同的 13 个工具、授权资源、原生二进制通道和持久任务接入两代协议，后续补齐[错误恢复与连接生命周期](../implementation/mcp-recovery.md)。[TS 控制/资源客户端](../implementation/operation-client.md)进一步复用同一合同，完成真实原生交付和资源恢复检查。下文定义的完整一期合同与生态接入要求保持不变；HTTP、完整公共动作、完整 SDK、Skill/Plugin 和 Musterwork 仍待完成，不能把当前工具集合当成目标范围。

版本 0.1 · 2026-09-24 · **接口设计提案，未发布 API**。上层结构见[Agent 接入总设计](../architecture/agent-integration.md)。本稿中的工具、包、命令、URI 与字段为待冻结候选；代码块是合同示意，不是当前可执行命令或完整 wire Schema。

实现跟踪：已有[持久化文档操作宿主](../implementation/operation-host.md)，支持作者模型创建/读取/原子编辑和本地任务回执；[资源宿主](../implementation/resource-host.md)进一步接通分块、摘要封存及范围读取，运行合同从 Rust 生成。[封存导出](../implementation/sealed-export.md)提供注入式 ResultSink、实际输出回读与原生文件暂存，仍不构成完整公共 export/bundle 交付。完整资源生命周期、导入/导出作业、完整信封字段、MCP/Skill/Plugin/SDK 与产品适配继续实施；本稿下列完整接口不能据此视为已经交付。

## 1. 标准采用与版本边界

宿主实现基础：[导出作业与公开资产](../implementation/export-host.md)将任务私有结果连接到指定版本导出、同库原子发布及重启读取；[统一分发与发现](../implementation/operation-discovery.md)进一步提供纯操作层 owner 接口、权限/配置感知目录和同源 Schema 查询。它们是可调用的 draft 服务能力；后续 MCP/TS 接入复用这些合同，完整目标信封和 Musterwork Artifact 继续实施。

[原生持久调度](../implementation/native-scheduler.md)已提供同库自动执行、独立控制连接以及共用执行池的 job/sync/auto 行为；该能力由 `hostScheduled` 如实报告。Rust API、开发 CLI、已实现 MCP 工具和 TS 控制层共用同一业务状态；完整 SDK、Skill/Plugin 与产品适配继续实施。

本次核对时，MCP 官方 `latest` 指向 **2026-07-28**，采用逐请求版本/能力元数据；早期版本采用 initialize 会话。不能把两代消息直接混用。本设计建议现代档案支持 2026-07-28，并提供 **2025-11-25 兼容档案**，两者分别测试；不推断所有现有产品已经升级。[MCP 当前规范](https://modelcontextprotocol.io/specification/2026-07-28)、[版本兼容](https://modelcontextprotocol.io/specification/2026-07-28/basic/versioning)

| 层 | 采用方式 | 一期基线或扩展 |
| --- | --- | --- |
| MCP Tools / Resources | 标准工具发现/调用、结构化结果及资源读取 | 基线；按所用协议版本封装 |
| stdio / Streamable HTTP | 本地进程与远程/自托管端点 | 两种都交付；浏览器页面不直接启动 stdio |
| MCP Tasks | 同一宿主 Job 的标准任务投影 | 可协商扩展；无支持时使用普通工具返回业务 job handle |
| Agent Skills 文件格式 | SKILL.md 与按需参考资料 | 基线技能包，独立于 MCP 扩展 |
| MCP Skills extension | 从 MCP 发现并读取同一技能内容 | 可协商增强，不作为基本创作前提 |
| MCP Apps | 内嵌预览/播放器的 UI 适配 | 可选增强；客户端支持域单独声明 |
| Plugin manifest | 按具体产品安装规范包装公共资产 | 平台适配，不定义新的通用办公协议 |

传输协议、操作 Schema、文档 Schema、能力/兼容档案、内核、Skill 和 Plugin 各自版本化。发行清单固定测试过的组合；升级协议不能隐式升级文稿或重解释旧 Artifact。未知必需版本/能力明确失败，不能静默丢字段。

## 2. Operation Contract

所有入口共享类型化操作定义。公共输入为文档/资源句柄、版本、操作和参数；可信调用上下文为宿主注入的 principal、scope、执行地点、期限、预算及 task fence。两者禁止混合，客户端传来的 tenantId 或任意路径不是授权凭据。

| 字段 | 合同 |
| --- | --- |
| contractVersion | 固定操作语义版本，与 MCP protocolVersion 不同 |
| requestId | 一次逻辑操作的幂等键；重试保持，改变参数必须换键 |
| documentId / baseRevision | 目标与乐观版本检查，修改类必需；新建不填 baseRevision |
| profileId | 已公布的能力/质量/资源档案；服务核对权限与实际可用性 |
| arguments | 操作特有的严格 Schema；大文本/操作集可引用已授权数据资源 |
| outputMode | 同步有界等待或异步提交偏好，不允许关闭必需质量检查 |

幂等键作用域为可信 principal/scope、操作名和 requestId；服务保存规范化请求摘要与结果。相同键不同参数返回冲突；重试不能生成第二个文档或重复导出。JSON-RPC request id 只关联本次消息，不代替业务幂等键。

结果公共字段：contractVersion、requestId、outcome、documentId/revision、job、artifacts、diagnostics、versions、usage、retention。`outcome` 为 `succeeded / accepted / failed`；accepted 只表示作业已持久接收，不表示 PPTX 已生成。各入口使用同一结果，不为 MCP 制造更弱的检查结论。

正式 Schema 从统一源生成/验证不同绑定；MCP 的 inputSchema/outputSchema 不另写业务规则。枚举、默认值、整数安全范围、字段上限、未知字段行为和错误码需冻结并做一致性检查。

v0.4 已提供[导出信封/交付清单的设计 Schema](../contracts/README.md)，固定 `outputMode=auto/sync/job`、状态分支、资源引用及五类质量声明。下面两个 JSON 示例纳入其正例校验。完整操作 IDL/SDK 生成器仍属实施工作，不因部分信封已有 Schema 而声称全部工具已实现；使用/保留期等宿主字段将在统一 IDL 中补全，不得由模型自授权限。

## 3. MCP 工具面

候选名称使用短前缀 `mo_`，实际宿主可能再添加连接器前缀。Skill 通过宿主工具映射绑定来源身份，不仅依赖同名工具。工具列表稳定排序，模块和权限决定可见集合；不按任意先前 tool call 暗中改变工具定义。

| 候选工具 | 输入与结果 | 关键约束 |
| --- | --- | --- |
| mo_capabilities | 协议/操作/模块/宿主/资源能力与限额 | 明确未实现、不可用和未验证；不返回用户文稿 |
| mo_assets_prepare | 类型/长度/摘要 → 上传或附件接入描述 | 仅准备资源，不能凭 URL 自动任意联网 |
| mo_assets_register | 已授权上传/附件/本地引用 → AssetRef | 校验字节、类型、摘要和权限；不接受模型自授路径权限 |
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
| mo_jobs_get | jobId → 状态、进展、终态结果/错误 | 授权后查询，返回建议轮询间隔和保留期 |
| mo_jobs_cancel | jobId → 取消受理/已终态 | 不伪称已经释放；提交竞争见第 5 节 |
| mo_resources_release | 明确的临时 lease IDs → 释放结果 | 不删除用户已提交文件；重复释放幂等 |

创建/导出不调用另一家模型来“自动补内容”；Skill/Agent 负责内容与设计选择。高层布局预设可以展开为同一领域事务。多个对象修改一次 apply 提交，避免 Agent 为每个字形进行一次工具往返。

MCP 封装提供 outputSchema、structuredContent 和必要的简短可读信息；为兼容需要时提供有界 JSON 文本镜像。图片只作为少量必要证据，完整文件返回资源链接。[MCP Tools](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)

工具 annotations 描述只读/破坏性/幂等等行为，但不承担权限控制。输出结构中的文稿文本必须作为数据呈现，不能提升为 system 指令或改变 Skill。参数/方法层错误与工具执行失败按对应 MCP 版本分开封装；业务诊断保留可机器处理的 code/path/retry 条件。

## 4. 文件、资源与传输

```text
AssetRef = identity + contentDigest + byteLength + mediaType + authorizedScope
ArtifactRef = AssetRef + role + documentRevision + qualityReportRef + retention
```

示意逻辑 URI 可为 `musteroffice://documents/<id>/revisions/<revision>`、`musteroffice://artifacts/<id>/manifest`。它们不是可跨服务直接使用的文件路径，也不是 bearer token；每次解析都经过 originating server 与可信身份授权。内容摘要是完整性证据，不能当访问凭证。

数据入口：

- **本地 stdio/CLI：** 安装时或任务上下文指定允许的输入/输出根和具体资源授权，校验真实路径/符号链接/打开时状态；核心只获得句柄。MCP roots 可辅助定位，不能自动视为完整权限沙箱。
- **产品内嵌：** 宿主将用户附件或 Content Store 对象映射为 AssetRef；资源跨执行地点时验证真实 bytes 或授权传输证明，不能仅转发另一存储域的 ID。
- **远程自托管：** 标准宿主提供授权上传/下载通道。产品可直接集成；仅有普通 MCP 的客户端由用户通过受控上传页面完成，随后工具使用登记的资源 ID。MCP 本身不被假定提供通用文件上传。

临时上传地址/凭据尽量经宿主 UI 或 SDK 数据通道传递，不放入常驻模型上下文和日志。只有明确能力允许的远程来源才由 ResourceBroker 获取，并限制重定向/地址范围；核心不自行联网。

大文件以分块、范围读取、流或本地文件句柄传输，验证后 seal。默认不将 PPTX、字体和媒体编码到 tools/call 参数/结果。小 JSON 描述和文本内容限额由 capabilities 声明；超出后走已登记数据资源，不能截断后返回成功。

MCP Resources 用于可授权的文档投影、报告、模板元数据和少量证据；大二进制通过描述资源返回传输方式。对仅支持 MCP blob 读取的客户端公布严格大小上限和标准下载替代路径，不伪造协议级任意 range 参数。[MCP Resources](https://modelcontextprotocol.io/specification/2026-07-28/server/resources)

输出 Artifact 的角色至少包含 editable-document、pptx、preview、playback-manifest、quality-report。所有引用绑定同一文稿版本及实际输出摘要；外部编辑后生成新资源，旧 QA 不继续适用。

## 5. 长任务、恢复和取消

宿主作业状态为 `queued → running ⇄ waiting_resource → succeeded / failed / cancelled`，取消请求为独立 `cancelRequested` 标志。该状态机属于办公操作宿主，不直接复制到所有协议版本的任务枚举。

| 客户端能力 | 行为 |
| --- | --- |
| 支持所选 MCP Tasks 档案 | 任务句柄映射到同一 Host Job；按该版本执行协商、查询、结果和取消 |
| 只有普通 tools/call | 长操作立即返回 outcome=accepted 与 jobId；用 mo_jobs_get/cancel 跟踪 |
| SDK/CLI | 同一 job ID、状态和幂等合同；SDK 可订阅，CLI 可有界等待 |

不能向未声明 Tasks 能力的客户端返回扩展任务结果。2026 Tasks 是可协商扩展，查询与终态结果遵循其实际规范；2025 档案的实验任务模型不与之混用，可先采用普通工具 job 模式。[Tasks 官方说明](https://modelcontextprotocol.io/extensions/tasks/overview)

2026 Tasks 的 working 可包含宿主 queued/running；需要客户端提供资源且支持相应交互时才映射 input_required。用户文稿检查不通过是一份带诊断的工具结果，不必把业务失败误作协议错误；任务状态、工具 isError 和公共 outcome 的组合需适配测试固定。

Job 在返回 accepted/taskId 前持久化输入摘要、operation/version、授权作用域、执行配置、幂等键和保留期。连接断开不等于取消持久 Job；协议当前请求终止和显式 job cancel 分开处理。stdio 子进程被结束时，标准宿主重启后根据 checkpoint/fence 恢复或给出明确终态，不能永久显示 running。

取消遵守核心检查点与强终止合同，但终态只有在执行和输出处理完成后确认。取消与提交竞争由宿主 CAS 裁定：取消先赢则禁止提交，提交先赢则返回已完成及结果，不删除已交付文件。重复 get/cancel、跨客户端查询和迟到结果均要验权。

持久 Job 与 MCP 连接身份分离但不与授权分离。重连后的访问由当前身份、scope 和 Job 权限判断；过期/撤权明确返回，不因知道 jobId 即开放。报告 retention/expiresAt；清理临时数据不能删除已转移给产品的 Artifact。

## 6. 合同示例与端到端调用

以下仅展示**工具 arguments 与规范化业务结果**，不展示 MCP wire envelope；现代请求元数据及旧版 initialize 由适配器按档案处理。

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

最小闭环为：能力发现 → 登记素材/源文件 → 创建或打开 → 查询 → 原子编辑 → 检查/指定状态预览 → 导出/任务查询 → 取回 PPTX。外部产品只需理解这些操作与文件入口，不需要重新实现字体、排版、质量检查或解包逻辑。

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

MCP 可用时优先使用它；只有宿主明确提供 CLI 绑定时走 CLI，不能自行下载执行不明二进制或猜包名。CLI 和 MCP 使用同一请求 ID/资源 scope；一项操作失败不能改用另一入口重复生成而不检查原 Job。

一个发布版本只维护一份 Skill 内容；文件包、MCP Skills 和平台 Plugin 引用相同摘要。支持 MCP Skills 的客户端可用其发现接口和 Resources 读取内容，读取文件与实际激活技能由宿主区分。扩展支持和基本 MCP 连接成功分开报告。[MCP Skills](https://modelcontextprotocol.io/extensions/skills/overview)

技能不内嵌模型凭据、联网服务或私有产品路径，也不复制 Schema 成为第二份权威。模型可能不遵循技能步骤，因此必需验证、权限与提交条件仍由操作服务/宿主强制实现。

## 8. Plugin 发行合同

公共 Release Manifest 描述版本、平台/架构、下载摘要/签名、组件及许可证、所需资源、支持的操作/MCP/文档版本、Skill 摘要、默认权限、安装/升级/卸载和兼容测试记录。这个 manifest 是本项目发行元数据，不宣称是 MCP 或 Agent Skills 官方格式。

平台包装只把上述清单映射成目标产品的 MCP 配置、技能注册、权限 UI 和可选 viewer 入口。原生客户端可安装对应二进制，远程客户端可配置自托管 URL；不为每个平台重新实现办公计算或携带另一套大型 JS 引擎。

安装先校验宿主/平台/组件完整性，注册能力，再做有界健康检查；权限按实际功能声明。升级保留用户文稿，明确资源/Job/Schema 的迁移；卸载移除注册和可回收组件，用户产物处理必须可控。禁止 install 脚本默认上传文稿、启动第二个 Agent 或偷偷开启全盘访问。

首批必须交付 Musterwork 官方适配及通用 MCP/Skill/CLI 资产。其他平台包装按明确的客户端版本逐个验证；不存在“一份 Plugin manifest 通吃所有 AI 产品”的承诺。框架不支持 Plugin 时仍能直接配置 MCP 或安装 Skill。

## 9. Viewer / MCP Apps

Viewer/Player SDK 负责页面导航、诊断定位、播放/暂停/seek、媒体权限提示和文件交付入口；它调用同一内核与资源合同，不用独立 DOM 排版重画 PPTX。产品外观和用户任务状态由接入方拥有。

支持 MCP Apps 的宿主可嵌入这一 viewer 的适配版；不支持时返回预览资源、受控浏览入口和 PPTX，保留基本创作/编辑能力。内嵌 UI、音视频/GPU能力及 CSP 根据宿主实际支持声明，不因“支持 MCP”自动宣称可完整播放。[MCP Apps](https://modelcontextprotocol.io/extensions/apps/overview)

工具只传递播放配置或采样需求；连续帧、媒体 bytes、音频时钟和 GPU 资源通过本地 SDK/数据通道。产品接入不能让每一帧触发模型或网络工具调用。

## 10. 授权与隔离

HTTP 服务按 MCP 授权规范实现相应认证/资源服务器行为；产品可对接自己的身份系统，不要求注册 MusterOffice 云账户。stdio 使用宿主注入的受限配置/凭据，不把远程 OAuth 流照搬成本地每次调用登录。[MCP Authorization](https://modelcontextprotocol.io/specification/2026-07-28/basic/authorization)

概念权限区分文档读/写、资源读/登记、导出、任务查询/取消及外部资源访问。权限由产品或标准宿主在任务上下文授予，Skill/模型参数不能提升。用户已授权的可逆编辑流程不因存在 MCP 再插入重复确认；覆盖/外发等行为仍遵从真实宿主策略。

同机、同一服务、同名工具均不代表同一租户。查询/下载/取消逐次验权，缓存按可共享内容与授权分离；错误输出不泄露另一文稿是否存在。日志排除令牌、私有源文件正文和敏感临时链接。

## 11. I01–I16 接入验收

| ID | 必須证明 |
| --- | --- |
| I01 | 干净机器安装本地 MCP 发行物及清单声明的必要资源，即可独立创建/编辑/导出，无 Musterwork 依赖 |
| I02 | 独立 HTTP 自托管：授权、上传、长任务、下载闭环，客户端不需本地执行引擎 |
| I03 | MCP 2026 与 2025 档案分别协商/封装/失败；无混用消息或错误能力声明 |
| I04 | 无 Tasks/Skills/Apps 扩展的客户端通过普通工具完成创作、job 查询及 PPTX 交付 |
| I05 | Tasks 启用后与普通 job/SDK 共用同一操作，断线/重启/过期/取消和提交竞争正确 |
| I06 | 文件 Skill、MCP Skills 与 Plugin 的技能内容一致，错误摘要/来源不能加载为同一技能 |
| I07 | Skill＋CLI 可独立运行；跨入口重试不重复修改/导出，无隐藏 Node.js 必需项 |
| I08 | SDK/MCP/CLI 同输入得到相同语义/质量结果，入口不私自补齐或丢失内容 |
| I09 | 大字体/媒体/PPTX 不经过模型上下文反复复制，测实际桥接内存与延迟 |
| I10 | 跨租户读/写/下载/取消失败，路径逃逸、未授权 URL 和文稿内指令不提升权限 |
| I11 | Musterwork 全消费者接入，Runtime 是唯一任务/提交 Owner，fence 和历史版本正确 |
| I12 | Web Worker、Desktop 与 Managed 路由和播放 SDK 正确，前台关闭不制造永久等待 |
| I13 | Plugin 真实安装/升级/卸载、权限与离线资源满足支持矩阵，用户产物不被误删 |
| I14 | MCP Apps 或独立 viewer 的动态播放正确，禁用 UI 扩展不影响工具生成和导出 |
| I15 | 一个独立第三方 MCP Client 无需改内核或客户端核心代码完成全链路，记录构建与限制 |
| I16 | Schema、包版本、Skill 与配置示例经 conformance kit 校验，能力清单与实际发行一致 |

当前 I01–I16 均未执行。接入性能报告在[内核工作负载](presentations/runtime-performance.md)之外记录工具描述上下文量、序列化/桥接成本、冷启动与插件完整体积；不能用空工具 ping 代替完整文稿端到端成本。
