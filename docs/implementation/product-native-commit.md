# Musterwork 原生候选恢复与 Artifact 原子提交

2026-09-27：接续[原生导出准备](product-native-export.md)，将实际计算候选接入 Musterwork 原有提交 owner。本页中的 `MW:` 表示产品相对路径，私有产品源码仍在产品仓库。本阶段不开放新 Agent 工具或切换默认 PPT 路由。

## 提交语义

`MW:apps/agent-runtime/crates/runtime/artifact/src/presentation_office_export/publication.rs` 负责唯一的候选到 Artifact 投影。它从候选的文稿身份、原 Artifact 基准版本、实际清单引用、完整依赖、Invocation 和创建时间构造版本；提交者必须来自原 RunContext。恢复与正常提交共用这一投影，拒绝调用方单独替换标题、版本、依赖或身份。

`OfficePresentationOperation::Export` 进入既有 `CommitPreparedPresentationTool`。同一个 SQLite 写事务校验当前 Attempt、取消状态、草稿完整状态、模型修订和 Artifact 基准版本，写入候选、Artifact 版本、依赖、草稿头、内容保留、Ledger、Tool 成功和幂等收据。没有第二个数据库、队列或提交 owner，也不增加“导出后再发布一次”的产品步骤。

首次导出创建 Artifact；继续编辑后导出更新同一 Artifact。导出不增加模型修订，草稿继续处于 Open，基准 Artifact 版本前进。晚到的旧模型候选及旧 Artifact 基准候选都返回冲突。原请求重试返回原提交收据，即使草稿随后已继续编辑和导出，也不写第二份副作用。

准备阶段实际读取的请求、模型、图片、字体与导出的文件、收据、检查报告、清单及 checkpoint 在同一事务重新授权和保留。任一撤销、过期执行权、取消或 SQL 故障都使候选、版本、日志和成功状态整体回滚。这里的提交保留不能代替准备阶段尚未接通的 Invocation 预写生命周期。

## 版本化恢复与共同存储

新候选为 `presentation-candidate-state/3`，恢复封套为 `presentation-authoring-checkpoint/4`。候选保存原草稿、Invocation、时间、清单的精确 JSON、清单引用和实际计算读取集。封套再绑定草稿 pin、ArtifactVersionId 和提交者。解码后重新编码并比较完整原始字节，拒绝重复/未知字段、额外状态、非规范数值和错版本；这些状态本身不授予发布权限。

原候选 `/1`、`/2` 和 checkpoint `/1`–`/3` 保留原解析及 2 MiB 上限。新候选上限 8 MiB，新 checkpoint 上限 16 MiB，包含嵌套 JSON 转义成本。这是持久元数据边界，不是整项任务的内存或磁盘预算。

候选共用原 `presentation_candidates` 表，按明确 discriminator 选择解码器。读取时核对原始字节摘要，以及冗余存储的候选/草稿身份、修订、文档摘要、候选绑定摘要、Invocation 和时间。原生读取接口不会隐式转换 HTML 候选，旧接口读取原生候选返回 IncompatibleFormat。

## SQLite v185 迁移

旧表的物理 CHECK 上限也是 2 MiB，仅扩大 Rust 解码器无法完成接入。v185 在事务中重建原候选表，保持所有历史行、唯一键、外键、不可变触发器和评审引用。较大的元数据仅允许明确 `/3` 标识；缺少 schema、null、旧格式、未知格式、非 JSON 和超过 8 MiB 的内容不能使用扩展容量。SQL 层通过并不表示候选语义有效，生产 codec 仍须完整验证。

重建使用从 v177 提取的共同连接守卫：只在持有的连接关闭外键检查，提交前执行 foreign_key_check，成功或失败后恢复并核对设置。如果 Future 被取消，连接关闭而不是带着关闭的外键设置返回池。迁移失败保留旧 schema 及历史图，不修改历史迁移文件。

## 验证状态

最终三项真实 worker 提交测试通过。测试重新打开产品存储，从已落盘 checkpoint 重建提交器；原 Device writer 被拒绝，新实例可继续提交。首次导出、真正编辑标题后再次导出、两个版本的实际文件回读、完整内容保留、旧请求重试、并发旧候选冲突、七类提交故障和候选元数据损坏均有断言。两个版本各 12 份实际交付文件另存用于独立核对，首版保持历史基准字节，第二版 PPTX 的真实标题与新模型一致。两版都保留五类质量声明，仅结构为 passed。

旧创作回归 50 项通过、6 项明确跳过：三项旧外部环境测试未执行，另外三项新真实 worker 测试已按显式固定材料单独执行。v183/v184/v185 的五项迁移测试和 v177 共同连接守卫的三项回归通过；历史图、容量分版本、非法外键回滚、末尾 schema 故障回滚和取消后的连接隔离分别验证。这些迁移夹具使用历史 DDL 与合成关系，不是用户数据库升级验收。

相关库单元测试 71 项、内容桥接集成 5 项、Artifact 应用集成 6 项通过。三个生产库的严格 `--no-deps` Clippy 和全工作区 all-targets 编译通过，后者保留既有测试未使用导入警告。通用检查结束后只修改了默认 ignored 的真实导出测试夹具，并重新编译、显式执行这三项测试；生产源码和普通创作测试摘要保持不变。没有运行完整产品测试套件。

恢复验证早期曾因夹具继续使用旧 Device writer 而正确得到 StaleFence；最终夹具完整重建存储、journal、reader 和提交器，并反向断言旧 writer 仍被拒绝。多次真实计算之间使用原 Runtime heartbeat 续租，不手工改长正例租约、不允许过期 Attempt 复活。早期编译失败和全部失败日志保留，未通过放松生产校验取得成功。

源码、实际文件、命令、退出码和验证范围记录于[阶段证据](../reviews/evidence/2026-09-27-product-native-commit-verification.json)。证据读取器提取为共用模块；三项反例测试要求拒绝不完整/矛盾的测试结果和缺失的文档链接，没有复制历史脚本再替换源码执行。

## 继续实施

下一步是原 Invocation 持有的预写资源登记、保留和预算，以及 worker 暂存的崩溃恢复；不能盲删确定性 ContentId，因为重试和其他合法版本可能仍在引用。完成这些边界后再组合生产工具入口。共享 Home/PostgreSQL、Viewer/Player、历史文稿/模板迁移、完整高级对象、Office/WPS 编辑往返、跨平台、性能、完整安装包体积和 E0–E3 仍待完成。

本阶段使用冻结 SDK 与 worker；当前核心仓库的其他改动没有因产品测试而得到重新验证。作者与导入文稿的统一语义/渲染链路问题仍见[评审整改 F2](review-corrections.md#f2-必须继续完成的架构闭环)，不能用产品事务闭环替代该架构工作。
