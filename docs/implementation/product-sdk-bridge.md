# Musterwork SDK 构建与内容桥接

2026-09-27：在[产品验证范围读取](product-content-range.md)之后，将真实 SDK 加入产品 Cargo 工作区，并接通输入资源、输出流和最终存储检查。以下 `MW:` 路径属于产品仓库；本仓库只保留相对路径、摘要和验证记录，没有迁入产品私有源码。

## 可从源码检出构建

此前 SDK 只存在于被忽略的开发缓存中。Cargo 会在运行 build script 之前解析路径依赖，包括 optional 依赖，因此不能让产品工作区的解析依赖缓存恰好存在，也不能要求编译 Linux/Windows 库时必须已有对应平台 worker。

SDK 现在作为产品源码依赖放在 `MW:apps/agent-runtime/vendor/musteroffice/<manifest-sha256>`，由现有工作区的相对 Cargo 依赖引用，纳入应随产品保存的源码文件范围。产品明确排除 `vendor/musteroffice` 子树；SDK 生成器同时在每个库的 package 中声明自己的 workspace。最初用通配排除目录和隐式继承时出现工作区归属错误，日志保留。最终真实产品解析与嵌套源码构建均通过，没有往产品全局依赖表复制内核内部模块。

本次固定 SDK 清单 SHA-256 为 `f3d44dca6f1f2632832eb3327b71d38915c354ab89143d578ec5618b0470471d`。22 个生产库的 Rust 与所需数据保持上一 SDK 字节；22 份库 Cargo 清单新增显式 workspace，来源记录相应更新。两次独立打包归档逐字节一致。所有变更都发生在新目录，旧封存材料没有改写。

`MW:scripts/prepare-musteroffice-development.mjs` 增加 `--sdk-only`；它在没有平台 worker 的环境中验证或显式导入固定 SDK，不下载、不执行程序，也不覆盖损坏的已存在目录。完整模式继续准备独立固定的 worker。目前仍只有 macOS ARM64 worker；原有 `generated/sdk` 目录不再参与产品编译。根包增加 `office:sdk:verify` 和 `office:dev:prepare` 命令。现有 Docker 对整个 `apps/agent-runtime` 的源码 COPY 包含新的 vendor 子树，本次没有声称完成 Docker 镜像构建。

[源码布局验证驱动](../../tools/verification/product-sdk-build-layout.py)在产品自己的忽略目录建立只含可进入版本控制源码的副本：包括 4,654 个文件，不包含 SDK 缓存、worker 或 MusterOffice 核心检出目录。真实 Cargo metadata 核对 22 个 SDK 库都从副本内部解析，且不是产品工作区成员；随后实际编译适配器生产库。使用本机已有 registry 和编译缓存，不等于空 registry、Linux 容器或整个产品启动验收。私有复制文件和编译日志留在产品仓库，驱动退出后清理自己的临时副本。

## 同一内容所有权

`MW:apps/agent-runtime/crates/infrastructure/musteroffice` 是已登记的 infrastructure 边界，不拥有任务数据库、队列、权限或结果提交权，也没有继承旧 HTML/PDF 成功合同。

- `OfficeContentReader` 从原 `ContentStore::open_verified()` 取得精确身份材料，再实现 SDK 的 `ReaderAt`。保留跨块读取、溢出检查与错误分类；没有文件路径或整文件 Vec 回退。
- 输入绑定使用 SDK `AssetId`；最终交付绑定保留 SDK `RequestId`，不互相伪装为同一种身份。两者共用同一泛型材料准入：先检查整批重复、ContentRef 冲突、数量、单项及总字节上限，再打开内容。相同 ContentRef 的别名复用一个读取器，但每个别名仍计入预算。
- 描述信息从已验证的产品材料生成。未绑定资源直接拒绝；取消后不返回半个资源集合。调用方继续使用 `RecordingContentStore`，在原 Runtime 提交中重新核对读取集。
- `OfficeOutputSource` 将候选的一个输出资源适配为产品顺序流。同步文件读取在阻塞执行器运行，每块最多 64 KiB。读取 Future 被取消后，source 保留原来的 pending 任务，重试不另排一项；任务实际完成前仍持有候选。恢复时允许调用方改用较小缓冲，不丢失或重复字节。
- 写回后的交付文件通过 `OfficeDeliverySource` 重新打开产品实际存储，并执行同一 SDK 的交付检查。计算候选、通过检查的私有内容和产品成功提交仍是三个独立阶段。

这里的准入限制仅约束一个材料集合；输出块大小仅约束适配器内部块。完整 Invocation 并发、文件句柄、磁盘与渲染内存预算、宿主崩溃后的暂存清理仍须由原 Runtime 实现，不能将这些接口当作已完成的全局资源管理。

## 实际验证

最终产品桥接 5 项、共享内容库 26 项、Artifact 库与集成 52 项测试通过；一项需要显式材料的真实 worker 测试另外运行并通过。该测试将原有自有图片/合成字体写入实际 SQLite 内容存储，经新桥接调用固定 worker，再用新输出流写回产品存储。最终检查通过，12 份文件及回执与前一 SDK 消费基准逐字节一致，两页检查报告不变。

输出取消验证占住唯一阻塞线程，实际入队后连续取消 16 次读取 Future，观察始终只有一个任务持有候选；释放线程后依次使用 1、3、17、31 字节缓冲恢复，完整字节一致且候选引用释放。另验证跨块和逆序读取、超界、精确租户/身份、别名复用、准入前拒绝、取消以及读取后原地损坏。损坏继续通过 SDK I/O 边界报告资源完整性失败，没有被改成可重试的存储失败。

产品准备脚本最终 15 项、原有 SDK 验证器 6 项通过。新适配器 all-targets 的 `Clippy --no-deps -D warnings` 和格式检查通过。产品全工作区 all-targets 编译通过，但有原有测试代码警告；包含依赖的 Clippy 首次受原有 `capability/src/browser.rs` 告警阻断，不宣称全产品严格 lint 通过。新增测试的 slice lint 已修正后重跑。

扩大架构回归为 27 项通过、2 项失败、1 项原有忽略：原有 `device-interaction` 工作区成员缺少边界登记，以及 PostgreSQL `resource.rs` 仍有直接事务入口。已核对 HEAD 中对应源码及相同静态断言，这两项不是本次新增；没有改写测试、放宽规则或宣称整体架构验收通过。记录包含源码依据，未伪称运行过另一个基线测试程序。

桥接与 SDK 生产库另通过 Windows MSVC 目标编译，使用已有 Rust 1.92.0 stable 目标。此证据不包含 SQLite 的 Windows 文件行为、原生 worker Windows 执行或整个产品运行。

## 依赖、证据与后续

产品 Cargo.lock 保留原有依赖并新增 SDK 图，更新其要求的 serde、serde_json、thiserror、crc32fast 等版本；没有更新内核锁文件或 worker。实际产品 SDK 图包含 50 个 registry 包，记录具体版本、许可声明和统一后的 features。其中若干传递版本与 SDK 独立构建锁不同，且产品 feature 合并引入两项额外依赖；以实际宿主图和本轮结果为证据，不把独立 SDK 的 48 包图冒充最终产品图。完整发行许可核查、性能、RSS 和安装包体积仍未验收。

[阶段证据](../reviews/evidence/2026-09-27-product-sdk-bridge-verification.json)绑定当前源码、产品私有文件摘要、实际宿主依赖、失败与最终日志、源码布局检查及最终导出字节。此次未改变内核计算源码、WASM 或固定 worker，未切换产品默认 PPT 路由。

下一步在既有 Runtime 中实现明确的 MusterOffice 草稿与生成请求、作业预算和暂存恢复，将候选、最终存储检查及 Artifact/Ledger/成功回执接入原子提交，再接通 Viewer/Player 与历史兼容。完整高级内容、Office/WPS 编辑往返、跨平台发行和 E0–E3 仍开放。
