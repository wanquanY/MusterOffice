# 可选 Streamable HTTP 计算入口

2026-09-27 · `mo-mcp/http` 已通过官方 `rmcp 3.4.1` 实现 MCP `2026-07-28` 无会话 HTTP，复用[本地薄 MCP](thin-mcp.md)的 SDK、调用方文件桥接和固定导出 worker。源码开发交付尚不代表通用远程附件接入或完整 PPT 替换验收。

## 接入边界

接入产品可以将 `mo_mcp::http::Endpoint` 挂到自己的网关：先认证每个请求，再选择调用方对应的 Endpoint。每个实例只打开宿主明确提供的输入、输出和临时目录；不同权限范围不能共用同一文件目录实例。产品拥有附件落地、文件生命周期、任务持久化、结果提交、TLS、CORS 和连接层限额。

MusterOffice 不增加租户注册表、OAuth、上传存储、持久 Job 或页面。有界并发、取消信号和临时导出文件是计算状态。客户端仍需调用方文件/附件通道；仅填写 HTTP URL 不能访问其本地文件。`mo-mcp-http` 仅绑定回环地址用于开发，不是公开部署网关。[使用说明](../../tools/mo-mcp/HTTP.md)包含启动与协议示例；HTTP feature 默认关闭。

## 协议与生命周期

按官方 [2026-07-28 Streamable HTTP](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http)处理发现、Tools、Resources、请求元数据与 SSE。每个 POST 独立执行，检查版本/方法/名称头与正文一致性，不保留协议会话或跨请求任务表。GET/DELETE 返回 405，历史会话/事件 ID 不能绑定当前调用。

HTTP 只声明 `2026-07-28`。旧式 `initialize` 不返回成功结果：SDK 沿用旧握手的 HTTP 状态语义，返回 HTTP 200 内的 `-32022` JSON-RPC 错误及支持版本列表；带逐请求元数据的旧版调用返回 HTTP 400、`-32022`。检查分别断言实际协议行为，不把 HTTP 200 单独视为成功。2025 HTTP 兼容继续待办，stdio 两代兼容单独验证。

默认拒绝浏览器 Origin；宿主可配置明确白名单。Host 匹配显式配置，不接受通配配置或重复关键头。严格 JSON 解码拒绝重复键和非法 UTF-8。适配器保留输入错误原因：正文超时 408、超限 413、无效输入 400，不被 SDK 的通用 Body I/O 错误误报为内部故障。

断开请求会取消其实际计算。阻塞计算继续持有计算槽直到停止并完成有界清理；相同 JSON-RPC ID 的并行请求也独立取消。宿主关闭时调用 `Endpoint::shutdown()`，保留 Tokio runtime 以等待阻塞计算和清理。操作系统文件 I/O 不是硬实时取消；强制杀进程后的最终文件由调用方处理。

## 资源与依赖

每个 Endpoint 最多 8 个在途 HTTP 请求，响应消费/丢弃前保持准入，满额立即 429。正文最多 4 MiB、读取期限 10 秒；请求和响应流共享 180 秒截止时间；最多 64 个头、16 KiB 头数据；响应最多 16 MiB。超限响应终止并报错，不返回伪造的截断成功。响应禁缓存，SSE 禁代理缓冲。

这些不是进程 RSS、TCP 连接或磁盘的全局预算。网关负责解析前的连接/头读取限制、代理超时及不再消费的流。计算与资源读取沿用独立 1–8 槽和既有文件预算。SDK 会缓存未知工具名称，因此按请求创建轻量协议路由，避免长期无界缓存；不可变工具定义、文件桥接和计算槽继续共享。没有 fork 或修改第三方源码。

独立 MCP workspace 新增 26 个锁定 registry 包，既有版本和归档摘要保持不变。[组件记录](../../components/http-runtime/component.json)保留新增包版本、校验和、完整根许可证/声明，完整发行许可闭包仍待验收。当前主机 normal 依赖图确认 HTTP 依赖不进入默认 stdio 或嵌入 SDK；这不是安装包尺寸/性能结论。

## 验证

11 步 MCP 构建/格式/严格检查通过；默认 17 项、旧兼容 18 项、HTTP 22 项单元测试通过，共有 17 项不重复累计。新增 5 项覆盖响应名额、字节预算、截止时间、输入错误和正文读取前的 Origin 检查；全 feature 组合严格检查通过。

独立 Python HTTP/SSE 客户端完成六份 Schema、创建/原子编辑、两版真实 PPTX 导出、24 份资源回读、重新导入、1 MiB 分块、文件域隔离和重启读取；另有 23 项协议/输入拒绝检查和 2 次并行真实导出。TCP 测试覆盖断线、关闭服务、两个同 ID 的 worker 独立取消、8 个慢请求准入和正文超时恢复。阻塞测试 worker 只用于验证生命周期。

两代 stdio 的真实调用、8 次并发导出及 4 组取消/EOF 回归通过。HTTP 与两代 stdio 的两版合计 72 份资产、结果与索引保持上一 SDK 阶段字节。真实 export worker 未变化，未重跑核心/WASM 或旧持久宿主完整流程；这些检查不代替 Office/WPS。输入、失败历史及结果见[阶段证据](../reviews/evidence/2026-09-27-http-mcp-verification.json)。

本阶段未改动 Musterwork。附件桥接、Skill/Plugin、旧版 HTTP、产品网关、完整高级内容及 Office/WPS/性能/体积验收继续按[实施目标](kernel-boundary-roadmap.md)推进。
