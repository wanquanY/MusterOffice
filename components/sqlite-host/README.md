# 标准宿主持久化组件

状态：ADR 0006 授权范围内的实现选型与验证；不是公开发行批准。固定版本与归档摘要见 [component.json](component.json)，Rust 完整依赖闭包由根目录 `Cargo.lock` 固定。

`mo-standard-host` 使用 rusqlite 0.40.2（关闭默认特性，启用 `bundled`）与其 libsqlite3-sys 0.38.2 中未经修改的 SQLite 3.53.2。SQLite 保存已接收请求、不可变文档版本、当前版本指针和任务结果，用同一事务裁定取消与提交。使用 WAL、FULL synchronous、fullfsync、外键检查和关闭 trusted schema。单元测试读取实际连接参数、SQLite source ID 及编译选项；事务故障、并发连接和进程终止另有集成测试。

SQLite 仅属于独立标准宿主。`mo-operation-service` 仍是无 IO 的计算与合同层；生产 WASM 内核不引入 SQLite。Musterwork EmbeddedHost 应复用产品自身持久化和任务所有者，再调用同一纯计算服务，不能在已有 Runtime 中另嵌一套标准宿主数据库。

新增锁定包的 MIT 许可声明逐份保存在 [licenses](licenses/)。SQLite 本身的 public domain 声明已核对归档中的 amalgamation，另见 [SQLite 官方版权说明](https://www.sqlite.org/copyright.html)。这些记录不改变 MusterOffice 尚未确定的公开许可证。

当前使用上游 bundled 构建，其默认包含 FTS、RTREE 等额外功能，不能宣称是体积极限配置。完整编译选项、宿主二进制和依赖统计进入本阶段证据。后续裁剪需要单独记录构建配置，并重验事务、恢复、兼容性及实际大小，不能仅凭数据库文件体积推算安装包。

复查脚本为 `python3 tools/verification/operation-host-dependencies.py`，从 Cargo 缓存核对归档与锁文件摘要，复制对应许可并通过 `cargo tree --locked --offline` 生成、检查宿主和生产 WASM 依赖树。脚本不下载代码、不使用系统 SQLite，也不修改上游代码。
