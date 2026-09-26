# 原生 MCP SDK 选型验证

状态：原生接入的开发实验已通过本页限定验证，尚未采用为生产适配器，详见[实验记录](../reviews/evidence/2026-09-26-mcp-sdk-evaluation.json)。统一操作服务和宿主继续沿用已有实现；不能把协议探针计为完整 MCP、E0-6、P07 或 Musterwork 接入通过。

## 候选与可复现输入

本次评估官方 [Rust SDK rmcp 3.4.0](https://github.com/modelcontextprotocol/rust-sdk/tree/rmcp-v3.4.0)，发布归档、源码提交和上游许可原文见[组件记录](../../components/rmcp/component.json)。独立[实验包](../../tools/experiments/mcp-sdk-probe/README.md)固定 Rust 依赖锁，使用 `server` 和 `transport-io`，关闭默认 features。它不进入计算内核、WASM 或主 workspace 的依赖图。

Cargo 声明为 Apache-2.0，但上游 LICENSE 还明确记录从 MIT 迁移的历史贡献，并包含两份许可文本。因此保存完整上游 notice，不将 manifest 中单个字符串当成完整发行结论。正式采用前仍需传递依赖、归属声明与发行闭包核查。

## 检查范围

实验实现一个 `mo_probe_schema` 工具，实际调用 `mo-operation-service` 的同源 Schema 计算。独立 Python 客户端启动真实 stdio 进程，分别覆盖 2025-11-25 initialize 会话和 2026-07-28 的逐请求元数据/discover，读取十份真实 Schema 并与已封存的 Native/WASM 结果比较，同时核对 outputSchema、结构化结果及文本镜像。非法 ID、额外身份字段和未知工具分别检查协议错误；两代结果的 `resultType` 差异不混用。

源码检查发现默认 async stdio transport 的完整行缓冲没有配置字节上限。本实验采用 SDK 自带的有界 framed codec，输入限制 4,096 字节，超限即关闭探针连接并返回非零退出；生产适配需要按文稿请求预算设定限额，并另行实现输出、在途消息、并发执行与总内存限制。协议库可复用，不意味着默认配置已满足本产品资源合同。

## 验证结果

最终 release 探针在 `aarch64-apple-darwin` 上执行 32 次实际协议请求，完成 20 次 Schema 对照、六次非法工具查询、两次非法版本/元数据检查，以及独立超限进程检查；格式与严格 Clippy 通过。首轮新增缺失元数据负例误写为 `-32600`；核对[官方基本协议](https://modelcontextprotocol.io/specification/2026-07-28/basic)明确要求 `-32602` 后修正测试，保留失败目录和 SDK 实际错误响应。初次 Clippy 的构造方式提示也已修正，未屏蔽检查。

原计算工作区源码、锁文件与生成合同保持上一阶段身份，不为独立实验重复计算旧测试数量。实验保存完整本机依赖图和许可声明，二进制大小只代表带实际 Schema 计算的选择探针，不能推导正式 MCP、完整办公内核或 Musterwork 安装包成本。

## 进入正式适配前的实现要求

现有 `StandardHost` 的 job 模式是 `explicitRun`。正式 MCP 需先把持久队列与执行调度接通，使 accepted 后的作业有实际执行者；业务 Job 仍只存于同一宿主，协议请求 ID 不代替幂等键。长导出不能阻塞状态查询和取消，连接断开与显式业务取消分别处理，重启时处理遗留租约。

随后接入权限过滤后的正式工具 Schema、结构化结果与资源引用，并分别验证两代协议的工具/资源调用、Streamable HTTP、可信身份、独立客户端及 E0-6 纵向交付。本实验没有创建第二个数据库 owner，也未把同步 SQLite 调用直接放入正式异步服务。
