# Musterwork 内容文件恢复与所有权

2026-09-26：延续[流式内容存储](product-content-stream.md)，在产品既有 SQLite owner 中增加物理写入登记、操作系统锁和恢复。以下 `MW:` 路径均在 Musterwork 仓库；本仓库只保存相对路径、摘要和验证记录，未迁入产品私有实现。本阶段不改变核心文稿计算、Native/WASM 或已固定的 SDK/worker。

## 同一 owner 下的物理写入

`MW:apps/agent-runtime/crates/infrastructure/sqlite/src/schema_v183.rs` 将数据库从 v182 原子迁移至 v183。新增 `content_material_writes` 记录写入身份、tenant、内容身份、摘要、声明字节数和 writer generation；原有内容、授权、任务和提交表保持其原有职责。这是内容文件的持久登记，不是第二套任务数据库或调度器。

写入最多同时预留 512 MiB、1024 个登记项，沿用产品单内容上限。额度在消费 Source 前检查，并随持久登记一起提交。这个限额只约束在途物理写入，不是 Invocation 总额、已发布内容容量、整个磁盘用量或进程内存上限。完整作业配额仍需由产品 Runtime 持有。

内容根下的 `.content-material-v1` 是专用暂存命名空间。注册锁覆盖逐项锁文件创建到数据库登记完成的窗口；每项使用 canonical UUID、独立的 OS 文件锁和 `.tmp` 文件。恢复只枚举这个受控目录，并从经过验证的 tenant/hash 推导 CAS 路径，不遍历用户目录或任意历史资源路径。

正常路径为：获得注册锁与逐项锁 → 持久化登记 → 释放注册锁 → 有界传输及输入校验 → 文件同步与实际字节回读 → 不覆盖地发布 CAS 文件 → 再次回读并同步目录 → 在原 writer fence 下提交内容元数据。等待 Source 时不持有 SQLite 写事务；回收时的引用检查与文件删除由注册锁及短写事务保护。

普通、共享上下文、共享工具和准入内容写入共用此实现。共享工具候选保留 material lease，直到进入原来的提交事务；准入暂存由已有持久 stage 保护。核心没有获得产品文件路径、授权或发布权。

## 取消、退出与引用保护

恢复以 OS 锁是否仍被持有判断真实写入是否活跃，不根据 PID 或时间阈值猜测。活跃写入跳过；已退出写入的临时文件、无引用 CAS 和登记按可重复执行的顺序清理。已发布内容、准入暂存、其他物理写入均保留对应字节。

阻塞文件操作具有独立生命周期：Future 取消并不会取消已经启动的文件创建、硬链接或删除。因此 `RegistryLease` 将注册锁带入实际阻塞任务，临时文件打开也将 `MaterialLease` 连同文件句柄返回。即使接收方取消，排队中的文件操作仍持有锁，丢弃的结果先关闭句柄再执行清理。生产注册和回收使用同一套锁封装。

删除已发布内容先完成原有墓碑事务，再在新的受保护事务中重新检查引用。先前的可达性检查不能授权后续删除；期间出现的新别名、准入 stage 或写入都必须保护字节。删除与恢复共用同一引用判断。

启动在恢复成功后才进入 Ready；写入和删除也会触发恢复，另提供显式恢复入口。错误不伪装成数据库完整性失败。恢复不能删除仍有 READY 元数据的内容，因此它不等于完整 Artifact/Invocation 孤儿回收。取消后没有后续活动时，已落盘但未发布的 CAS 可能要等显式恢复或重启才清理。

## 平台与依赖

目录同步集中在 `MW:apps/agent-runtime/crates/infrastructure/sqlite/src/content/material/fs/platform.rs`。Unix 使用目录及不跟随末端链接的打开标志。Windows 使用目录句柄所需的 `FILE_FLAG_BACKUP_SEMANTICS`、不跟随重解析点的标志，并申请同步所需的写访问；错误向上传递，不以空操作返回成功。依据为 Microsoft 的[目录句柄说明](https://learn.microsoft.com/en-us/windows/win32/fileio/obtaining-a-handle-to-a-directory)与[FlushFileBuffers 访问要求](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers)。当前实际运行证据来自 macOS；Windows 分支仍须在真实目标文件系统执行验证，不能由编译检查推定兼容或断电安全。

本阶段复用产品已有 fs4 1.1.0（MIT OR Apache-2.0）、uuid 1.24.0（Apache-2.0 OR MIT）、rustix 1.1.4（Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT）及现有 Tokio/摘要依赖。tempfile 3.27.0 返回测试依赖，不再由 SQLite 生产清单直接引入。没有新增外部包版本；完整产品依赖闭包与发行许可仍单独验收。

## 验证范围与后续

真实子进程分别停在未登记暂存、部分输入、CAS 已落盘但元数据未提交、元数据已提交四个边界，然后由父进程实际终止并重开产品存储。前三类未发布材料清理，已发布字节保留。测试中的 ignored 子入口仅防止其作为普通测试单独等待，父测试会显式运行并终止它四次。

独立取消测试占满唯一阻塞工作线程，并确认实际 I/O 已入队后取消调用方，检查注册锁仍被持有；另一用例验证取消的文件打开不会在租约释放后重新创建孤儿暂存。测试用计数器只观察生产封装的任务提交，不替代文件操作。最初的恢复测试停在更早的 SQL 等待点，不能证明删除已入队；已收紧观察边界并保留各次日志。其余验证覆盖容量先行拒绝、准入与别名保护、v182 实际数据迁移、目录目标检查及现有流式读写行为。

最终 SQLite 内容、准入、共享内容、迁移与启动恢复筛选共 41 项通过，另一个 ignored 条目是上述显式子进程夹具，不是跳过四个终止场景。SQLite 生产库严格 Clippy 及所改 Rust 文件格式检查通过。目录同步模块另由 Rust 1.92.0 对 x86_64-pc-windows-msvc 完成实际源码编译检查；没有链接整个产品或在 Windows 执行。最新真实存储回读的 12 个资产和 bundle 共 13 个文件与此前封存字节一致，固定摘要的共享内核 CLI 再次检查后的报告也一致。没有运行整个产品的所有测试，没有构建新的绘制 worker、WASM 或安装包，也没有新的吞吐/RSS 结论。[阶段证据](../reviews/evidence/2026-09-26-product-content-lifecycle-verification.json)绑定代码、全部原始日志和最终产物。

封存期间产品并行开发提交了 `e7d6393bf3f3704d9ddcff3473b1f14609e879ff`，增加其 Artifact application facade 和内部依赖边。原生读取实现与 SDK 保持字节不变，Artifact 主文件的新增导出按原提交核对；这些改动未回退。产品 Cargo.lock 因该工作变化，外部 registry 版本与摘要不变；最终验证在新的依赖状态重新执行，旧日志保留。不能声明整个产品锁文件与上一阶段相同。

进程强制退出不等于硬件断电、内核崩溃、磁盘损坏或跨平台通过。旧命名空间外的临时文件不在本次扫描范围；历史不可达 CAS、墓碑提交后中断的独立删除、已发布但无业务引用的内容仍需完整持久回收设计。不能把物理写入恢复扩大为全部资源生命周期完成。

继续实现作业总额与持久回收、云端流式端口、不可变范围读取，再串联原生草稿、候选、最终存储检查、原子 Artifact 提交和 Viewer/Player。完整高级内容、Office/WPS 往返、历史迁移、跨平台发行及 E0–E3 替换门槛仍未关闭。
