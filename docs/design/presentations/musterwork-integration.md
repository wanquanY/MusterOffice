# Musterwork 演示文稿替换合同

v0.4 设计提案 · 2026-09-24。本页是 MusterOffice 的接入合同；实际源码字段、author@3 行为和 MW01–MW16 验收已在[适配实施规格](../implementation/musterwork-adapter-spec.md)展开。Musterwork 的具体 Runtime/Schema/发行仍由其仓库维护；本项目不跨仓库导入私有源码，不因迁移文档直接修改产品 wire 协议。

接入形态已补充为 **官方集成包：Skill＋MCP＋宿主/Artifact 适配＋Viewer/Player SDK**。公共架构见[Agent 接入总设计](../../architecture/agent-integration.md)，工具/资源/异步合同见[公共接口规格](../agent-interfaces.md)。这些能力也可独立提供给其他 AI 产品。

## 1. 已观察基线与目标

来源代码基线和历史文档见[迁移索引](../../references/musterwork/README.md)。现有产品有生成/编辑、源文件预览、模板维护、外部改稿和 Browser/Managed 路由；并非只有一个前台导出按钮。

内核接管：语义文稿、无 DOM 布局、字体计算、原生 PPTX 读写、静态与动态渲染、结构/内容/行为检查。Musterwork 保留：内容与审美推理、模板业务、权限与 Content Store、任务/进度/重试、工具发行、Artifact 提交和用户界面。

产品流程仍是 Agent 创建/修改、用户预览/播放/下载、Office/WPS 原生编辑。无需将 HTML 源码、内核图结构、codec 等实现细节放入普通用户操作流程。

## 2. 消费者完整映射

| 现有入口/消费者 | 新接入方式 | 替换门槛 |
| --- | --- | --- |
| Agent author / 改稿 | 旧合同适配为语义事务，调用 compile/export/verify | 真实文件、诊断、进度及成功提交语义一致 |
| 上传 PPTX 源预览 | read-only Reader + 懒加载页依赖 | 至少保留现有 80 页预览容量，不被编辑限制拦截 |
| 可编辑导入 | Reader + sourceBindings + 能力报告 | 至少保留现有 40 页范围；原生高级对象纳入新档案 |
| 模板参数化/编辑/压力验证/组装 | 语义模板 API、资源和逐单元结果 | 参数与动画/图表/Diagram/公式引用一致 |
| 模板后台维护 | 既有持久 Job/Runtime 调用 Managed 或获准本地宿主 | 页面关闭后仍按既有任务策略继续、恢复或明确终止 |
| Web 工作区预览/下载 | 新 Artifact reader + Worker 渲染/播放器 | 实际 bytes、完整资源、播放状态和下载为同一版本 |
| Desktop 本地预览/打开 | 受管进程 + 本地文件 hash 监听/重读 | 外部保存后旧 QA/页图失效，不覆盖用户修改 |
| API/Channel/无人值守 | 原生 Managed 宿主 | 无浏览器也能完整生成/校验/提交 |
| 历史 Artifact/文稿/模板 | 按原版本读取，显式创建升级副本 | 历史内容可访问、可下载、可继续进入新编辑流程 |
| 发布检查/发行组件/诊断 | 新内核能力清单、SBOM、全链路测试 | 已无旧编译/转换组件的活动调用与打包依赖 |

旧源码入口索引：`MW:packages/presentation-engine/src/portable/operations.mjs`、`portable/execute.mjs`、`import/core.mjs`，客户端 `apryse.mjs`、`portable/collect.mjs`；Runtime 路由 `MW:apps/agent-runtime/applications/cloud-worker/src/presentation_routing.rs`。`MW:` 仅表示来源相对路径，不是运行依赖。

## 3. 旧操作到领域 API

| 旧操作族 | 领域职责 | 适配注意 |
| --- | --- | --- |
| author | create/transact/compile/export/verify | 同一 revision 与资源闭包；候选通过后交回原有提交 Owner |
| source_preview | open/inspect/render.sample | 使用只读操作档案，不强行构建可编辑全模型 |
| import_extract | open + semantic import report | 区分原生理解、保留和缺失；高级内容不能以 opaque 算已支持 |
| document_contract/validate/inspect/edit | capabilities/validate/query/transact | 旧对象 ID 与新 ID 显式映射、冲突行为保留 |
| document_parse_html | 受限旧 AST 输入适配 | HTML 是旧输入合同，不继续用 DOM 当布局事实源 |
| source_page_validate/materialize | 单页及依赖闭包检查/实例化 | 共享母版/媒体/字体关系不能丢 |
| template_parameterize/edit/stress_plan/assemble | template API + 文档事务 | 模板业务状态、Agent 策略和任务持久化仍在产品 |
| template_inspect/instantiate/validate/page_validate | 模板查询/实例化/检查 | 参数与页/对象/时间目标共同校验 |

每行必须在 Musterwork 发布前绑定实际 Tool ID、请求/响应 Schema、执行路由、Artifact 版本和测试 ID；v0.4 实施规格已核对当前 author@2/@3 及关键持久合同并规定 successor 字段。正式继任 wire 名由配套实现登记，不能把设计示例当作已存在版本。公共 `mo_*` 工具不直接重命名历史 Tool；旧入口通过版本化适配映射到公共操作。

## 4. 合同与历史版本

当前观察到的合同包括 `presentation-document/1`、`presentation-native-engine/2`、`presentation-delivery-proof/2`、`presentation-candidate-state/2`、`presentation-artifact/3`、`workspacepreview/2`；这些是来源项目的基线，**不是本项目应沿用的命名空间**。源码/发行清单是版本最终依据。

| 合同类别 | 迁移策略 |
| --- | --- |
| Tool/Skill | 发布明确 successor；历史不可变调用仍按历史版本解析，不重解释字段 |
| 文稿 | 保留旧 AST bytes，显式转换为新文档版本并报告 ID/默认值/行为映射 |
| Candidate/Proof | 新证据绑定 engine/schema/profile/resource/output hash；旧证明不自动升级 |
| Artifact | 产品版本 owner 定义新模型、PPTX、预览、播放清单和报告关系；共享解析器唯一维护 |
| 历史 PDF 或旧预览 | 作为原版本资产继续可读；不因此引入新内核 PDF 运行依赖 |
| Template | 原版冻结，迁移产生新版本；已有文稿引用原资源不变 |

新播放清单应包含可验证的文档/plan 版本、资源摘要、能力要求和状态入口；它不是可执行任意代码的网页。Artifact 读者拒绝未知必需字段/能力，不能一边显示旧封面一边标记动态播放已完整支持。

历史读取不应无限依赖旧引擎。原 PPTX 可直接下载或由新 Reader 预览；继续编辑先进行显式转换/导入。如果存量确有转换器不能处理的已支持内容，需补齐 Reader 或完成经验证的一次性迁移，之后才具备旧引擎退役条件。

## 5. 宿主与任务所有者

现有 Browser/Managed 路由由持久策略选择，客户端优先、仅客户端、仅受管不能被内核混为自动 fallback。执行前冻结路由、资源授权、输入摘要、能力 profile、任务 generation/fence；普通渲染错误不得暗中换执行位置。

任务所有者仍是 Runtime：提交请求 → 选择执行宿主 → 提供资源 → 观察阶段 → 收集不可变候选 → 校验 fence/摘要 → 原子提交 Artifact。内核只返回计算进展，不自己写业务数据库、修改 Run 或再启动 AgentLoop。

浏览器关闭、设备断线、Managed 超时和后台接续分别按现有合同处理。模板维护允许的后台接续须沿用既有受控路径及 CAS，不能因某个 Worker 崩溃产生两个提交者。网络恢复后的晚结果也需匹配当前 fence。

进度至少区分资源准备、导入、布局、写出、实际字节检查、预览和提交；播放等待用户手势独立呈现。百分比基于可度量单元，未确定总工作量时显示阶段，不能伪造 99% 无限等待。

## 6. 外部编辑和结果一致性

用户用 Office/WPS 保存后，本地文件成为新版本；原已提交 Artifact 保持不可变。文件摘要改变触发重新读取和预览，标识本地修改，清除旧质量声明。页面数、文件名或 mtime 相同都不能代替 bytes hash。

“继续编辑该文稿”和“上传为可复用模板”采用不同业务入口与权限。继续编辑基于当前 bytes 生成新的语义版本及 ID 对齐报告；转换为模板还需要参数化、约束和压力验证。不能悄悄将用户文件注册成模板。

Agent 新结果不得覆盖用户已外部修改的文件；产品以新版本/冲突提示处理。原始图表数据、公式、SmartArt 和播放关系都需要参与差异检测，不能只比较截图。

## 7. 切换顺序和退役证明

1. 固定旧行为语料和所有消费者清单，保存历史合同、资源、输出及已知缺陷。
2. 发布隔离的开发候选适配器，复用既有任务 Owner；对授权语料做双轨对照，两个候选不能同时发布到用户 Artifact。
3. 静态与高级能力、全部宿主、历史数据、外部编辑、异常恢复分别通过；质量与性能不能互相抵消。
4. 用明确 capability/profile 路由受控启用，持续检查版本一致性。尚未迁移路径仍标记未完成，不能声称已全面替换。
5. 验收全通过后，默认新任务走自研；移除活动路径中的 DOM 测量、Chromium/Playwright 编译依赖、旧 PPTX 生成器与商业转换器及旧 Office/PDF 工具链。是否仍被其他非 PPT 功能使用需分别审计，不能盲删共享依赖。
6. 验证干净客户端/容器安装、离线字体媒体闭包、历史结果读取和故障恢复；证明不再隐式启动旧组件。

回滚是已发布版本和路由的受控操作，不能在一次失败调用内静默切回旧引擎。旧引擎未支持的新高级文稿不得送回旧引擎处理；可回滚到能理解同版本的已知良好内核，或停止该能力的新任务并保留数据。版本保留与读取兼容不等于永久维持两套运行内核。

## 8. 完成证据

Musterwork 替换报告逐消费者列：Tool/Schema/Artifact 版本、宿主、输入/输出 hash、测试、用户操作、失败/取消恢复、旧依赖移除证据。产品集成通过还必须结合[完整能力验收](verification-and-roadmap.md)，不能以“现有静态功能已迁移”替代一期完成。

本次只迁移设计依据、完善接入合同，没有修改 Musterwork 运行时代码、启用新路由、执行迁移或清理任何旧依赖。

## 9. 官方集成包的落地边界

| 集成件 | MusterOffice 负责 | Musterwork 负责 |
| --- | --- | --- |
| Skill | 可移植的演示文稿工作流、工具绑定约束、参考资料 | 注册/加载、用户上下文、现有工具发行与权限 |
| MCP | 统一工具/资源/任务适配，stdio/HTTP 与版本档案 | MCP Client/注册、Invocation 上下文和执行路由 |
| Host Bridge | 公共 Resource/Execution/Job/Commit 接口 | 连接 Content Store、Browser Gateway、Managed、CAS/fence |
| Artifact | 文件、模型、预览、播放及 QA 的公共引用与摘要 | 产品 Artifact Schema、共享解析器、持久提交与下载 |
| Viewer/Player | 同一内核的显示与播放 SDK | 面板/交互、实际权限、生命周期与状态展示 |
| 安装/升级 | 公共发行清单、二进制/资源完整性及兼容记录 | 产品 Plugin 或内置能力的具体安装发行机制 |

Agent 工具调用首先属于现有 Runtime Invocation，然后进入 MCP/Operation Service；返回的 Job 是这次 Invocation 所属计算的投影。桥接层绑定已有工作，不再递归创建另一条 Run 或 AgentLoop。Musterwork 若将工具内嵌为原生调用，也必须执行相同操作 Schema 和宿主合同。

Web 场景的 MCP 接口可以由受控服务承载，再通过既有客户端执行网关调度 WASM Worker；这不意味着把布局移回服务器。Desktop 采用受管原生执行，Managed 为无客户端任务提供相同能力。显示与播放直接使用 SDK 数据通道，不把每帧内容变成 Agent 工具消息。

旧 `author` 的候选通过后仍由原提交 Owner 按既有合同提交，不因接入 MCP 新增“请再批准一次生成”的阶段。公共 MCP 导出返回真实结果与报告；Musterwork 产品发布/工作区写入由已授权任务和提交策略决定。

完成替换时除原有消费者矩阵，还须通过公共接口 I11–I12，并证明同版本发行物可在独立第三方客户端使用。独立客户端不得需要导入任何 `MW:` 私有源码。
