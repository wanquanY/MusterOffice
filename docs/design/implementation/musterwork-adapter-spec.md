# Musterwork 适配实施合同

v0.4 设计基线。源码核对基线：Musterwork commit `eb0eefcd6d1783789e4f2049181c2914178547fe`，2026-09-24；记录的是此次工作区观察，不代表将来源码永远相同。`MW:` 表示该项目相对路径，不是本仓库依赖或公开源码迁入。

## 1. 连接边界与产品所有权

2026-09-27 以 [ADR 0007](../../decisions/0007-kernel-only-integration-boundary.md)及[内核与接入产品边界](kernel-host-boundary.md)为当前合同。本文带日期的实现记录保留当时范围，不代表新边界已实现，也不再要求 MusterOffice 交付业务宿主或产品页面。

最终连接形式为：Agent Skill/原工具 → Musterwork 现有 Runtime Invocation → 公共 SDK 或薄 MCP → 同源 Native/WASM/Managed 计算 → 不可变候选 → 原 Runtime 提交 → 产品 Artifact 与自己的预览/编辑/播放页面。

SDK 可以直接调用；不为统一入口强制增加 MCP 网络往返。文件引用、账号/权限、业务任务、generation/fence、持久存储、路由和提交事务由 Musterwork 持有。MusterOffice 提供渲染和无界面播放接口，不提供 Viewer/Player 产品 UI；其他接入方同样不必安装自有标准持久宿主。

适配改动须按责任核对：PPT 语义与计算在内核；资源句柄、原任务与结果保存属于产品薄桥接；共享存储/日志/全局回收治理属于产品工程。直接影响该链路的安全缺陷仍需修复，但全产品治理不是所有 Agent 接入的共同前提。SQLite v188 快照引用代码仍在开发、未完成库级验证；不能沿用 v187 的通过记录，也不因边界调整删除现有数据或迁移。

## 2. 已观察合同和必须升级的地方

| 来源与现有行为 | 新适配要求 | 测试 |
| --- | --- | --- |
| `presentation_tool.rs` 接受 `artifact.presentation.author@2/@3`，参数为 `{operation, parameters}` | 历史版本继续按原 schema 解释；新 typed 文档操作采用明确 successor 或公共工具绑定，不把新字段塞进旧版本 | MW01 |
| `author@3` 禁止 review/publish；compile 校验草稿 pin 后调用原子 Export | 新引擎候选通过后仍直接交原提交 owner；不得新增用户“再发布一次”步骤 | MW02 |
| 旧 draft 是 `presentation-document/1` 的 HTML/AST 合同 | 历史编辑可沿原 AST 语义后转换编译；全高级对象编辑必须进入新模型，不能反向压进旧 AST | MW03 |
| `presentation-html-artifact.ts` 当前 document≤4MiB，PPTX≤128MiB，assets≤512 | 新版 parser/准入/上传下载采用新档案，同步验证 Content Store 与网关；旧版本限额不被悄悄扩大 | MW04 |
| 当前 assets 仅 PNG/JPEG/SVG/TTF/OTF，无音视频 | 新清单按 image/font/audio/video/model3d/embedded/source/other 有限角色登记；每种媒体类型有解码/用途校验 | MW05 |
| 当前导出预览 1–40 页、单边≤8192；源文件预览另有80页路径 | 两个 operation profile 分离；新共享解析器不能误用创作40页验证源文件80页 | MW06 |
| `presentation-delivery-proof/2` 的字体证据依赖 `rendered-pdf-font-inventory/1` | 发布原生字体/布局/实际 PPTX 字节证据 successor；不能伪造 PDF hash 或重用旧 discriminator | MW07 |
| `presentation-candidate-state/2` 无 PDF；/1 有 PDF，解析器严格检查 | 候选升级包含原生 plan/playback/resource/evidence 引用；历史 decoder 原样保留 | MW08 |
| `presentation-artifact/3` 的 `target_application_status` 只能为 `not_proven` | 新版区分包检查、预览、原生编辑、播放、外部目标证据；不能原地扩大旧字段语义 | MW09 |
| compile/export 的 Artifact/Resource/Ledger 与 Tool receipt 共用 fenced transaction | 适配器只产 immutable refs 和验证收据，不提前写 Artifact 或成功状态 | MW10 |

确切历史版本为 `presentation-native-engine/2`、`presentation-delivery-proof/2`、`presentation-candidate-state/2`、`presentation-artifact/3`、`presentation-draft-state/1`、`presentation-review-state/1`。简写 `delivery-proof/2` 不是实际 wire ID。

## 3. 操作映射

| 现有 author operation / 消费者 | 新领域序列 | 返回产品信息 |
| --- | --- | --- |
| describe | capabilities + input/output/profile schemas | 版本、资源要求、可编辑/播放能力、限制；编译 profile 不再冒充旧 HTML 引擎 |
| template | template.describe/query | 版本固定的参数、约束、页/对象/时间目标 |
| read_content | authorized resource read/query | 复用产品授权 ContentRef，不将任意路径开放给内核 |
| begin/open/import/instantiate | create/open/template.instantiate → typed revision | draft/新模型ref、来源ref、ID/默认值/能力转换报告 |
| read | document.query/diff | 固定版本、分页结果与源身份映射 |
| edit | document.transact | 原子新 revision/changeSet；旧 AST 编辑与新 typed 编辑严格分版本 |
| replace_html | 旧版受限 HTML→typed 转换；仅历史入口保留 | 保留原输入和转换报告；新工具使用 text/shape/table/chart 等类型操作 |
| insert_template | template.instantiate + slide/object insert transaction | 母版、字体、资源、动画目标一起复制重编号 |
| abandon | release draft/session according to host retention | 不删已发布 Artifact 或被其他版本引用的资源 |
| compile | pin check → requirements → compile/export/verify → candidate → host Export | Artifact、预览、播放、诊断、真实质量状态；不新增 review/publish |
| legacy review/publish | 历史版本专用适配 | 不允许 @3 或新默认入口倒退成旧二阶段产品流程 |
| source_preview | read-only open + page index + sample | 80页档案、按需预览、源 hash 与能力说明 |
| import_extract | semantic open + import report | 原生可编辑模型、来源保留、确切能力缺口 |
| 模板维护全部操作 | template/model API + 原任务路由 | 后台接续、参数压力验证、草稿/版本/CAS 保持 |

旧底层操作 `document_contract/validate/inspect/edit/parse_html`、`source_page_validate/materialize` 及 `template_parameterize/edit/stress_plan/assemble/inspect/instantiate/validate/page_validate` 均归入上述 typed 领域能力。HTML 适配仅作为历史输入转换，不重新引入 DOM 排版或另一个输出真值。

## 4. 新交付清单与版本边界

公共清单采用 [`musteroffice.bundle/1-draft`](../../contracts/delivery.schema.json)，明确 `document/revision`、PPTX、预览、资源闭包、可选 playback manifest、原生编辑声明、质量证据和完整版本元组。它不包含 tenant、数据库事务、产品 artifactId 或可由 Agent 伪造的权限。

Musterwork 的产品 owner 分别持有 artifact/draft/candidate IDs、来源 invocation、版本序号、ContentRefs、pin/fence 和 public-to-product resource map，不能把任务权限塞入公共 bundle。最初设计时 successor 尚未分配；2026-09-26 已开始实际实现 `presentation-artifact/4` 的 Rust/TS 读取及文件选择，字段与边界见[产品原生清单读取](../../implementation/product-office-manifest.md)。它包含 Artifact 身份、public bundle、资产映射、版本 pins 和五类声明；2026-09-27 已实现 `presentation-draft-state/2`、`presentation-authoring-checkpoint/2` 与 SQLite 同 owner 的原生草稿事务，见[原生草稿提交](../../implementation/product-native-drafts.md)。随后已实现实际 SDK 创建/编辑准备器及 `presentation-authoring-checkpoint/3` 读取集提交，最终原生专项 11 项及相关库 82 项通过，见[真实内核创建与编辑](../../implementation/product-native-mutations.md)。原生 candidate、新工具组合、Invocation 预算与 Artifact 提交仍在实施，不能认为本清单本身已完成生产接入。

| 公共信息 | 产品字段/验证 |
| --- | --- |
| documentId/revision/documentRef | 新草稿的语义谱系与固定 pin；旧草稿转换另存 provenance |
| pptx hash/bytes/type | 产品 output_pptx ContentRef；所有预览与证据引用同一输出 hash |
| preview pageId/imageRef/sample | 新 previews；sample 明确 editor/initial/event-frame，不能以封面代表播放质量 |
| playback manifest | engine/plan/resource hashes、能力、初始状态与入口；不是网页脚本 |
| resources | 与 Content Store 一一映射、内容寻址校验、作用域及生命周期；媒体支持 range read |
| evidence claims | 每项 type/status/profile/subject digest/evidenceRefs；严格区分 structural/visual/editability/playback/target-application |
| engine/schema/rules/font/registry versions | 编译键、候选 binding、恢复与 Viewer 兼容校验共同使用 |

2026-09-27 后续：[原生导出准备器](../../implementation/product-native-export.md)已实际生成固定 worker 候选，并对最终产品存储重新检查后构造 `/4` 清单，保持五类质量声明和完整读取集。新增真实导出与既有草稿/库回归通过；此处的“候选”仍是计算结果，持久恢复、原 Artifact 提交事务、Invocation 保留/配额及生产工具组合继续实施。

播放包可以是可重编译模型或兼容版本的 plan 缓存；相同 bundle 不能依赖某台机器内存中的 handle。Viewer 遇到未知 plan 版本须用兼容内核重编译或明确失败，不猜结构。不能执行包内任意 JS。

2026-09-27 提交后续：[原生候选与 Artifact 提交](../../implementation/product-native-commit.md)已实现 `presentation-candidate-state/3`、`presentation-authoring-checkpoint/4` 和 SQLite v185。候选与旧格式共用原表，旧版 2 MiB 限制保留；新版候选为 8 MiB、恢复封套为 16 MiB。实际候选通过同一 Attempt/草稿/Artifact 校验后，版本、引用、Tool、Ledger 和收据一起提交，草稿仍可编辑。真实重开恢复、旧 writer 拒绝、版本冲突、重试和失败回滚已有测试；Invocation 预写资源与完整配额、生产工具及共享 Home/PostgreSQL 仍未闭环。

2026-09-27 资源后续：[Invocation 预写额度与授权](../../implementation/product-invocation-content.md)已实现独立的 `InvocationContentStore` 和 SQLite v186，绑定原 Attempt/Invocation，先原子预留完整输出集合，再于实际文件写入前后校验执行权。额度不因重启或失败写入重置，普通内容身份与受管输出隔离。当前是存储端能力，准备器、checkpoint、提交保留、终态回收与共享 Home/PostgreSQL 尚未完成接入。

2026-09-27 准备器后续：[Invocation 资源接入准备器](../../implementation/product-owned-preparation.md)已要求创建/编辑和导出使用专用存储端口，在第一笔写入前分别预留完整输出计划；原 Runtime 新增提交元数据准备函数，将 checkpoint 与 Tool result 成对预留，并由原生提交事务正式保留工具结果。实际调度器/journal 与固定 worker 回归通过。旧普通预写及恢复格式保持兼容，不据此宣称完成新版提交来源门禁；终态回收、worker 暂存恢复、生产工具入口及共享 Home/PostgreSQL 继续实施。

## 5. Job、资源与提交状态机

2026-09-27 原生宿主补充：[执行暂存所有权与恢复](../../implementation/execution-spool-recovery.md)在独立原生适配器中引入父进程/worker 的 OS 共享锁、有界孤儿恢复和 `--execution-lease-v1` 启动协议。该库级阶段尚未更新产品固定材料，不能把库级验证视作产品接入完成。旧非受管目录不能按年龄删除；物理暂存清理也不代替产品内容引用和 Invocation 账户的终态回收。

产品后续见[原生宿主与暂存维护](../../implementation/product-execution-spool.md)：实际 Cargo 与 worker 已更新为同一固定 SDK 代际，新增进程级 `OfficeHost`，每个引擎实例仍须注入原 Invocation 的受管存储。Device 的可选受信任配置只指定 worker 路径与摘要，暂存根由宿主决定；启动执行一批恢复，原 retention 循环继续有界维护。取消后的排队操作保留名额且检查取消，不建立第二个任务 owner。此组合不启用新 Agent 工具；完整运行与回归范围以阶段记录为准。

引用准备见[日志内容引用索引](../../implementation/product-ledger-content-roots.md)：SQLite v187 在原 Ledger 行记录解析覆盖状态，同事务派生协议明确声明的 ContentRefs，迁移逐条读取并核对原正文摘要。实际 Tool 附件、恢复引用及两版 Artifact 提交已验证；原输出帧正文回收保持兼容。该目录不是内容读取授权或永久保留指令，自动回收仍须同时核对原调用真正终态、各领域持久引用、嵌入资源闭包和既有正文生命周期；当前尚未启用业务终态删除。

```text
Invocation admitted (route + revision + capability + fence fixed)
  → preparing resources → computing → sealing → verifying
  → immutable candidate → host commit CAS → succeeded
  ↘ waiting_resource / failed / cancellation_requested
```

公共 jobId 映射现有 Invocation 的一次执行，而不是新 Run；重试 attempt/generation 分离。进度事件有单调 sequence、phase、完成/总单位、资源需求，允许无总量的阶段进度，不编造百分比。

ResourceProvider 用授权 ContentRef 转换出的 scoped handle；内核可 readRange，不知道对象存储地址。ResultSink 提供有界上传/spool 和 seal 后读取；验证读取的必须是最终存储 bytes。资源保留至少覆盖候选验证、提交以及已发布 Artifact/活动会话引用，不能 Writer 一结束就释放字体。

提交条件：当前 draft pin、输入摘要、route/generation/fence、engine/profile、输出和所有资源摘要匹配；全部必需检查成功；未有取消获胜；Artifact/Resource/Ledger/成功 receipt 原子写入。任何一项失败都不发布半成品。

取消与提交竞争只允许一个持久终态：取消先 CAS 成功则晚候选废弃；提交先成功则返回已成功与明确取消未生效结果。断线不能自行宣告取消或重路由；模板受控后台接续沿现有机制迁移 fence，旧客户端晚结果不能覆盖。

## 6. 历史、外部编辑与切换

历史 Artifact 保持原 bytes/声明，提供下载与新 Reader 预览；继续编辑创建新谱系版本并保存 `originArtifact/version/sourceHash` 和 ID 转换报告。不能把旧质量声明升级成新内核已验证。若只有旧 AST 而无 PPTX，受限转换器必须覆盖既有支持域；转换失败需补实现，不能删除记录。

用户在 Office/WPS 修改文件后，按 bytes hash 识别新输入；旧预览/QA 失效，重新读取后给出语义差异。保存新产物时使用 expected-file-hash 避免覆盖用户并发修改；创建模板另走明确业务入口。

切换分 read-compatible → developer candidate → authorized corpus comparison → 全门禁 → 新任务默认 → 旧依赖退役；双轨比较只允许一个 owner 提交。rollback 只能切到理解当前模型/能力版本的内核，不能把新 SmartArt/媒体文稿送进不支持的旧静态引擎。

## 7. 产品验收矩阵

| ID | 用户链路/故障 | 必要断言 |
| --- | --- | --- |
| MW01–MW03 | 旧工具、@3直接导出、历史草稿升级 | 旧 schema 不重解释，新功能可继续编辑，无新增发布审批 |
| MW04–MW06 | 大媒体/资源、40页创作、80页源预览 | 上传/存储/编译/解析/播放/下载限额一致，无错误套用40页限制 |
| MW07–MW09 | 新质量证据、候选、Artifact | 无伪造 PDF 证据，无单项 passed 替代全部质量 |
| MW10 | compile 成功与提交故障/重试 | exactly-once 产品副作用由持久 CAS 保证，同请求返回同收据 |
| MW11 | Web Worker、Desktop native、Managed 无客户端 | 同语义/资源输入；同错误/取消语义；客户端关闭不误提交 |
| MW12 | 创建→Agent修改→预览/播放→下载→Office/WPS修改→再编辑 | 原生对象和时间/数据修改完整传递 |
| MW13 | 模板维护、背景任务、设备断线、晚结果 | 原路由策略、受控接续、generation/fence 正确 |
| MW14 | 历史产物/模板、旧 PDF、模型升级与回滚 | 原资产不改写，原质量不冒充新验证，新版本可恢复 |
| MW15 | 干净安装/离线/旧依赖移除 | 不启动旧 DOM/Chromium/PptxGenJS/商业转换/Office/PDF 运行链；共享依赖先查其他用途 |
| MW16 | Skill/MCP/CLI 与第三方宿主 | 共用合同，无 MW 私有源码依赖，实际文件及摘要一致 |

本页最初冻结适配责任和测试要求；当前已开始修改 Musterwork 的原生 Artifact 读取合同，尚未切换产品生产者和用户入口。实施必须继续同步服务端 validator、持久状态 encoder/decoder、共享产品 parser、客户端收集/Viewer、工具/Skill 和发行清单；只改读取侧不构成替换。

来源索引：`MW:apps/agent-runtime/crates/runtime/artifact/src/presentation_tool.rs`、`presentation_authoring.rs`、`presentation_commit_protocol.rs`、`presentation_prepared.rs`、`presentation_state.rs`；`MW:packages/agent-runtime-product-client/src/presentation-html-artifact.ts`；`MW:packages/presentation-client/src/delivery-proof.mjs`。这里只记录观察和接口设计，没有复制私有实现。
