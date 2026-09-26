# 持久化文档操作宿主

状态：P07 首个实际文档操作链路已实现并完成本阶段验证。完整目标仍为 v0.4 一期内核及 Musterwork 替换条件；本页不是 E0-6、E1 或 E3 通过声明。源代码、依赖、实际进程结果和检查记录见[阶段证据](../reviews/evidence/2026-09-26-operation-host-verification.json)。

## 职责与实际范围

`mo-operation-service` 提供严格类型化请求、可信调用上下文和不可变修改候选。它调用既有文档模型与事务准备，不持有路径、数据库、网络或系统时钟；新增可取消事务入口仍共用原有编辑算法。`mo-standard-host` 持有 SQLite、身份作用域、持久幂等记录、执行租约及最终版本提交。`mo-host` 是安装者配置数据库路径、principal、scope 的本地 NDJSON 命令入口，不是认证过的远程服务，也不是 MCP 协议端点。

目前可执行的操作是创建现有作者模型文档、按当前或历史 revision 读取、提交现有原子编辑事务、查询和取消任务。已有图形、文字、资源引用及时间树随同一文档版本保存。资源引用并不表示本宿主已经保存对应字体或媒体字节。公开 draft profile 为 `presentations-author-model-v01-draft`，不接受假定完整能力已经可用的 profile。

标准宿主只服务独立接入部署。未来 Musterwork EmbeddedHost 复用自身任务与 Artifact 存储，调用同一计算服务并实现相同提交约束；不能把本 SQLite owner 叠加到产品 Runtime 中。尚未增加跨仓库源码依赖。

## 一次逻辑操作

可信 `CallContext` 不实现 JSON 反序列化；操作参数不能指定 principal、scope、权限、数据库路径或执行代次。文档由 scope 隔离，获得该 scope 文档权限的成员可共同使用文档；任务回执同时由 principal 和 scope 隔离。标识符及内容摘要都不是访问凭证。修改权限允许执行和重试该修改，独立的文档查询、任务查询与取消权限分别检查。

幂等键为 `(scope, principal, operation, requestId)`，请求摘要绑定合同版本、profile、完整操作及 requestId。`outputMode` 只决定交付方式，不改变逻辑操作身份。重复提交先查已持久化请求；相同参数返回同一任务及回执，即使文档已经到达新版本也不会再次应用。不同参数复用同键明确失败。终态回执保持不可变，不受后续时钟回拨影响。

accepted 只在接收事务提交之后返回。作业保存请求、配置对应的 executor 摘要、状态及时间；CLI 的 executor 摘要来自实际可执行文件字节。排队任务只由相同 executor 执行，升级后不悄悄重解释旧请求。这里的 executor 摘要用于当前文档计算版本固定，不是未来含字体、codec 的完整构建锁。

## 计算、提交与恢复

claim 原子地将 queued 变为 running，并记录递增 fence 与 lease 截止时间。不可变基础快照的解码、语义验证、事务计算和输出快照序列化在写事务之外进行。候选保存请求摘要、基础 revision、基础内容摘要、实际新快照及修改回执；只由共享计算构造。

finish 在同一 SQLite 事务中检查调用权限、任务 owner、executor、fence、租约、取消标志、请求身份及当前文档版本。基础内容摘要还须与已保存的基础版本一致，不能用同一个 revision 标签替换计算输入。新 revision、head 和成功回执一起提交；任意写入失败全部回滚。两次修改基于相同旧版本时最多一个提交成功，另一个返回版本冲突，不覆盖赢家。

queued 取消可立即终结；running 取消先记录 `cancelRequested`，不声称计算进程已退出。取消先持久提交则后到候选不能发布；结果先提交则取消返回已有成功回执。取消之后不能续租。同步检查点取消同样不产生半份文稿。

进程退出后保留 running 租约。后续查询或执行请求发现租约到期，将任务持久终结为 `EXECUTION_INTERRUPTED`；已经请求取消的任务终结为 cancelled。迟到候选无法提交，原 requestId 仍返回原终态；需要重新执行时使用新 requestId。当前没有自动重执行或无限续租，也没有宣称断电耐久已通过物理设备测试。

租约到期禁止提交，不证明旧进程已经释放内存。独立调度器、worker 强终止、全进程资源回收及生命周期清理仍需后续实现；不能把这项 fencing 验证当成完整取消验收。

## 接口与有界行为

Rust → JSON Schema → TS 同源生成 `host-request`、`host-response`、`operation-request`、`operation-job`。结果 outcome 统一为 accepted/succeeded/failed；成功负载显式区分任务回执与文档快照。未知字段、重复 JSON 键、未知 profile 和非规范整数失败。时间与 fence 使用最大 i64 范围的十进制字符串，避免 JavaScript 整数精度损失。

当前请求及输出快照上限为 32 MiB，编辑操作数量与文档对象限制沿用纯内核验证。宿主设置每 principal 的任务回执数量、每 scope 文档数量及租约时长上限，不自动删除文档或已完成回执。完整存储字节配额、分页查询、临时资源保留期及清理器尚未实现，不能将这些数量限制视为完整资源预算通过。

`outputMode=job` 持久排队，由操作者显式执行 `mo-host <db> <principal> <scope> run <job-id>`；auto/sync 在当前进程执行已有有界文档计算。没有后台自动调度，也没有硬性响应时限承诺。此 CLI 用于真实接入底座验证，后续协议适配必须复用服务，不能另写一套业务规则。

## 验证与依赖

测试使用真实磁盘 SQLite 与多个独立连接，覆盖幂等竞争、两个编辑者的版本竞争、取消与提交竞争、撤权、跨 scope/principal 隔离、内容来源、executor 固定、时钟回拨、租约过期、配额、数据损坏及部分写入回滚。测试子进程持久 claim 并计算后由父进程终止，重新打开数据库确认没有 revision，并验证最终恢复状态。测试钩子只存在于测试二进制。

CLI 测试每次调用都启动实际新进程，验证 queued → 执行 → 创建 → 修改 → 历史查询 → 原请求重试。release 二进制另经 9 次进程调用，独立 SQLite 读取确认完整性、两份不可变版本、两份成功回执和一个当前文档。纯计算测试遍历创建与多操作编辑的每一个取消检查点，并验证恢复后的快照与回执相同。

全仓 763 项 Rust 测试通过，比上阶段增加 25 项；一个标记 ignored 的测试入口由父测试作为真实子进程启动并终止，不是尚未执行的功能测试。严格 Clippy、格式、Native/release 构建、100 份 Schema/TS 合同、纯操作服务 WASM 编译均通过。22 个既有编辑请求经过实际 Native/WASM 配对，并与上阶段固定 WASM 的返回值逐字节比较。此前 96 份内核 Schema 不变；这不是完整渲染回放，也不是新操作服务的 WASM 传输运行验收。

组件版本、归档摘要、许可证和构建边界见 [SQLite 宿主组件](../../components/sqlite-host/README.md)。SQLite 不进入生产 WASM；当前 bundled 配置还未进行体积裁剪。当前 arm64 macOS release `mo-host` 为 4,729,696 bytes，仅含本阶段文档操作与存储能力。Rust WASM 为 7,824,721 bytes（比上阶段增加 2,160），C++ WASM 保持 2,370,691 bytes。本阶段没有测量完整内核速度、RSS、Musterwork 安装包或目标应用保真度，不把这些独立产物相加当作发行大小。

## 仍需完成

统一服务还需连接资源注册与传输、PPTX 导入/导出、预览/播放、public bundle 与质量证据；接通调度与强终止、MCP stdio/HTTP、SDK、Skill/Plugin。Musterwork 的 Artifact 提交、Viewer 和历史迁移尚未实现。完整动画/转场/媒体及可编辑高级对象继续属于原目标，不因该宿主链路可用而降级。
