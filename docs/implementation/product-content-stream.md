# Musterwork 内容存储流式接入

2026-09-26：延续[嵌入 SDK](embedded-sdk.md)，本阶段直接修改产品的共享内容端口和 SQLite 文件存储。以下 `MW:` 均为产品相对路径，私有实现不迁入本仓库。这一步提供真实存储能力，尚未接通原生生产者、Artifact 原子提交或 Viewer/Player。

## 接口与所有权

`MW:apps/agent-runtime/crates/runtime/content/src/stream.rs`定义顺序 `ContentSource` 和私有 `ContentSink`。来源填入调用方的缓冲，目的端完成当前写入后才请求下一块。共享传输使用一个 64 KiB 缓冲，检查短读、非法返回长度、提前 EOF、额外尾字节和 SHA-256；零长度也做 EOF 检查。沿用产品原有单内容限额，不提高 PPTX、页面或资产集合的准入上限。

`ContentStore::put_stream_at`按调用方已固定的完整 ContentRef 写入；`copy_to`授权后复制到调用方私有暂存，并在完成时重新核对元数据。未实现这两个端口的存储返回明确错误，不调用整块 Vec 接口作为隐藏回退。当前 PostgreSQL/对象存储尚未实现流式端口，不能据此启用云端原生生成路由。

Sink 在传输成功之前不可发布或使用。错误、取消和最终授权失败都要求调用方丢弃完整暂存，即使此前收到过部分字节。RecordingContentStore 仅在底层复制完整成功后登记读取引用，后续仍需 Runtime 对这些引用执行提交时的授权检查。流式读取不授予 Conversation、Artifact 或 Invocation 权限。

## 桌面端实际文件路径

`MW:apps/agent-runtime/crates/infrastructure/sqlite/src/content/stream.rs`复用既有写入锁、数据库 writer fence、身份检查和唯一的内容元数据插入函数。写入前检查身份及 fence，文件传输期间释放 SQLite 写事务，最终插入前再次检查。输入变慢不会长期占用数据库写事务；失去 writer generation 的实例不能因计算已完成而发布元数据。

`stream_file.rs`将输入增量写入同目录的私有临时文件，校验输入并同步落盘，再读取临时文件校验实际字节，使用不可覆盖发布操作，最后再次核对最终路径并同步父目录。目的文件已存在时只验证其字节。普通 `put/put_at`及共用底层写入函数也进入这条路径；去重命中不再仅凭元数据返回，损坏的既有文件不会被自动修复或覆盖。

读路径按完整 ContentRef 验证 tenant、墓碑和 locator，使用既有八个并发读取名额，以实际文件流检查长度和摘要，并在慢 Sink 完成后重新授权。内置只读 Runtime 材料继续使用其已有完整身份校验。文件句柄不向模型暴露；Unix 文件打开拒绝跟随末端 symlink。

临时文件由 RAII 持有，普通错误与 Future 取消会清理尚未发布的暂存。强制终止进程不会执行 RAII，取消也可能发生在已经持久化 CAS 字节之后，因此**本阶段没有完成崩溃暂存回收、不可达 CAS 回收或作业级磁盘总额管理**。这些必须继续纳入产品持久 owner 与维护流程，不能把普通取消测试当成进程崩溃恢复验收。

## 真实文件与验证

自有交付夹具的 12 个资产与 public bundle 共 13 个文件，从文件源流式写入真实 SQLite/文件存储，再通过新读取端口写到独立最终文件。回读结果与封存夹具逐字节一致。之后由[独立核对工具](../../tools/verification/product-content-stream-files.py)调用此前冻结的真实内核 CLI，重新检查模型、资源闭包、PPTX、预览像素及声明；检查报告与原封存导出一致。修改最终内容字节或外部 renderer pin 的两个反例被拒绝。这不代表执行了新的 Office/WPS 或产品提交验收。

存储测试覆盖 8 MiB 加 17 字节的非整块输入、短读、空输入、摘要/长度错误、身份冲突、损坏既有文件、取消后重试、慢传输期间的数据库写事务、读取中删除、记录读取集、写权限代际变更、竞争目的文件、symlink 与内置内容保护。8 MiB 测试验证实际请求/接收块不超过 64 KiB，不作为整个进程 RSS、吞吐或安装包测量。

最终 runtime-content 全部 20 项测试通过；SQLite 内容、准入和共享内容相关筛选共 25 项通过，其中包含 18 项内容读写测试。源码摘要、最终产物和原始日志由[阶段证据](../reviews/evidence/2026-09-26-product-content-stream-verification.json)绑定。初次编译的四处错误来自产品数据库错误类型与 SQLx 错误类型的映射不匹配，已修正；失败日志保留。生产库严格 Clippy 的范围是 runtime-content 与 sqlite，不能扩展为产品全仓质量结论。

## 依赖与剩余接入

共享内容库增加对产品既有 sha2 0.11.0 的直接依赖；SQLite 将已锁定的 tempfile 3.27.0 从测试依赖转为生产依赖。未引入新的外部包版本或修改 MusterOffice 计算代码、Native/WASM 二进制。tempfile 采用其已有的 MIT OR Apache-2.0 许可；这不是完整发行 notice 或包体积评估。

下一步继续补齐存储的持久回收与作业配额、云端流式实现及不可变范围读取适配，再将原生草稿、候选、最终存储验证和现有 Runtime 原子提交串联。需要分别完成普通取消、强制退出、重启恢复、提交竞争与已发布资源保护，不能创建第二套任务数据库。

完整高级内容、播放器、历史转换、Office/WPS 编辑往返、跨平台发行与 E0–E3 替换门槛均保持开放。
