# 导出作业与公开资产提交

状态：本阶段实现与验证完成，源码与产物由[阶段证据](../reviews/evidence/2026-09-26-export-host-verification.json)绑定。标准宿主的导出开发 profile 连接[私有交付计算](presentation-delivery.md)与既有任务 owner，不代表完整 v0.4、MCP 或 Musterwork 替换验收通过。

## 从接受请求到可读取结果

`mo-operation-service` 增加 `DocumentAction::Export`，动作名为 `presentations.export`，profile 为 `presentations-pptx-resource-delivery-v1-draft`。请求包含 `documentId`、`baseRevision` 和 `ExportSettings`；后者提供交付设置、文档资源到已授权资产的绑定、可选字体包资产及预期 renderer 身份。执行路径不接受文件路径、身份或权限参数。

导出绑定指定的不可变历史版本。接受或执行期间文档 head 可以继续变化，导出仍对应所请求的版本，既不覆盖 head，也不将正常编辑误报为导出版本冲突。相同 principal/scope/动作/requestId 的重试返回同一任务；改参数而复用 requestId 仍拒绝。

`compute_export` 是可移植计算入口：验证动作与 profile、快照、完整资源绑定、字体字节身份以及 renderer。它调用同一 `mo-presentation-delivery::build`，不另写一套 PPTX/预览生成器。交付配置摘要计算由 `DeliverySettings::input_digest` 共用。`ExportCandidate` 的私有字段绑定请求摘要和实际交付候选，不能从 JSON 构造成功凭据。

标准宿主调用链为：持久接受 → claim → 指定快照与授权资源读取 → 私有计算 → 同库提交 → 按资产 ID 读取。`outputMode=job` 先返回 queued；当前本地 CLI 的操作员再调用 `run`，它尚无后台常驻调度器。sync/auto 沿同一 owner 在调用内执行，不创建第二套任务。

## 原子提交与资源生命周期

计算期间的所有输出仅属于当前 principal/scope、job、executor、fence 和请求摘要，公开资产查不到候选。提交使用短写事务，重新核对任务、取消/过期、指定版本、候选摘要、全部输出所有权、封存描述和分块完整性，然后同时保存所有资产映射、Published 状态与成功回执。SQL 中途失败整体回滚，不允许一部分资产可见而任务未成功。

数据库 v4 增加 `result_assets`。它指向原有 `result_spools/result_chunks`，不复制全部负载，不将生成结果伪装成上传。发布后的实际字节继续计入作用域配额，取消、任务终态清理及重启不删除已发布资产。私有 reader 则在提交后失效；后续读取必须重新通过授权公开资产入口。

相同作用域中，重复导出已有的相同资产身份和描述时，验证既有 published backing 并复用。重复候选的私有分块在本次任务终态清理中删除。上传资产与导出资产身份冲突明确拒绝，不将任意同名上传作为已校验导出。跨 scope 不能获得资源；同 scope 的读取权限由宿主注入。

数据损坏检查包含封存元数据、分块数量/总长/索引和公开读取时的分块摘要。完整输出字节已经在私有计算中回读核对；提交写锁内不再次复制或重读所有负载。数据库管理者恶意同时改写数据与摘要不在这个完整性模型之内。

## 续租、取消与失败

长导出沿既有取消检查点检查真实任务状态，并在约三分之一租约间隔续租。续租更新同库私有结果期限；实际 I/O 或渲染不持有长写事务。正在使用范围读取器时仍可做短事务续租，无需复制资源来满足可变借用。Native renderer 的 Send 约束仅属于原生宿主配置，不强加给纯核心/WASM 接口。

另一个连接取消任务会回收私有结果；计算停止，续租不能恢复取消或过期任务。最终提交再次检查，避免“最后一次取消检查后取消”发布成功。调用者的取消回调也进入同一失败提交路径。

底层 OPC/PNG/I/O 包装保留宿主的结构化错误原因，包括取消、资源过期、限额与权限。真实页面错误进入 `RENDER_FAILURE`，可附原始诊断。`Failure.detail` 缺省时不增加 JSON 字段；原创建/编辑成功回执保持已有 wire 形态，导出回执包含 `documentId/revision/semanticDigest/bundle`。

存储写入或提交错误不能伪造成功，也不能假定数据库仍可写。此时保留此前原子状态；当前实现可能仍显示 Running，直到取消、租约过期或宿主恢复处理。没有自动重算/续跑同名私有输出或掉电恢复承诺。完整任务调度、强制终止、自动清理策略及全进程内存预算仍需后续实现。

## 本地调用与未来嵌入

可信操作员启动 `mo-host <database> <principal> <scope>`，通过 NDJSON 提交生成合同中的 `HostRequest`。实际渲染需配置 `--preview-worker <path> <sha256>`，再执行 `run <job-id>` 或同步请求。该配置可以省略用于纯文档/资源操作；执行导出而未配置 renderer 会失败。

完整的真实上传、创建、异步导出及二进制读取调用由[独立验证程序](../../tools/verification/export-host-reference.py)构造，使用仓库自有输入，不依赖隐式系统字体。请求和回执的机器合同见[operation-request](../../contracts/generated/operation-request.schema.json)与[host-response](../../contracts/generated/host-response.schema.json)。它们是可运行 draft 合同，与 `docs/contracts/` 中目标设计信封分开登记。

Musterwork 以后通过嵌入宿主映射现有 Invocation/资源/取消/Artifact 提交。标准宿主数据库是独立运行实现，不要求 Musterwork 在其现有任务系统外再部署一套数据库和任务队列。MCP、Skill、Plugin、完整 SDK/Viewer 和实际产品适配仍待实现。

## 验证与边界

[显式宿主集成测试](../../crates/mo-standard-host/tests/exports.rs)使用真实固定摘要的 worker。除所有资产完整发布外，还覆盖重启读取、历史版本与同时编辑、重复导出不重复占用持久字节、跨 scope 拒绝、取消/过期、权限变化、候选错绑、真实页面失败，以及第二个资产写入时的 SQL 故障回滚。新增长任务测试推进显式宿主时钟跨越初始租约，并在已有私有字节时分别取消、过期和触发调用者取消，检查续租不能复活任务。

release CLI 独立检查已执行 31 次调用：12 个实际 JSON 请求和 16 个 JSON 回执通过生成 Schema，其余调用返回原始二进制；两页共 12 个公开资产的字节与数据库块、长度、摘要一致，重试和成功后取消保持结果。两份真实 v3 资源/任务库仅在副本上迁至 v4，原有行、素材字节与原库文件摘要保持不变。它们没有已提交文档；首轮脚本错误假定存在 head 而失败，修正测试入口为实际资源读取后通过，首轮目录保留。

公开交付的独立文件检查涵盖模型和资源引用、PNG 像素、10 个相关 XML 部件的 XSD、15 个原生对象及 8 段文字。两页内容采用自有合成字体，文字是 `A A`；不能将这个输入视为商业文稿视觉保真证明。五类质量声明中，运行时仅 OPC/ZIP/XML 图和摘要结构检查通过；排版、可编辑往返、播放和 Office/WPS 仍为未证明。

本轮全仓 821 项 Rust 测试、格式、严格 Clippy、107 份 Schema/TS 检查通过；普通测试忽略的 2 项私有交付与 7 项宿主导出测试另行用真实 worker 显式执行。新增导出请求/回执与可选错误 detail 影响五份生成 Schema，其余 102 份保持不变。原有 mutation 回执没有加标签或改变 JSON 字段形态。

当前 CLI 和新构建 WASM 完成 72 次文件/编辑配对调用，所有用例结果与上一阶段一致；公开交付中的两页直接经当前 Native/WASM 预览入口绘制，元数据和全部像素相同，并保持上一阶段页面结果。纯操作服务 WASM 构建与依赖检查通过，没有引入 SQLite 或原生进程组件。完整 WASM 导出作业运行另行验收。

复现程序为 `export-host-workspace.py`、`export-host-reference.py <新目录>` 和 `export-host-parity.py <候选目录>`，由 `export-host-evidence.py` 封存并用 `--check` 复核。最终公开文件验证目录为 `.codex-work/export-host/reference-4/`。归档检查发现 Python 的 SQLite 上下文退出未关闭连接，记录后的关闭回写会改变库文件摘要；验证程序已改为显式关闭后记录，在新副本重跑。最终 12 个交付文件与双端重放所用的 reference-3 逐字节相同。旧阶段程序、输入、数据库及证据不覆盖；本轮早期编译、旧迁移数量断言、错误分类和 Clippy 失败记录同样保留。

完整 WASM 导出 owner 尚无执行证据，纯核心 WASM 编译与实际页面双端绘制分别检查。当前也没有完整高级内容、目标应用往返、发行闭包、最终安装包或产品性能验收结论。后续继续完成标准操作/资源生命周期、Agent 接入与 Musterwork 桥接，完整演示能力及 E0–E3 门槛不变。
