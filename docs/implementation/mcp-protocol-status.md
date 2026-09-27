# MCP 版本、Plugin 与接入验收状态

2026-09-27 · 用户要求核实实际采用的协议是否最新且完善。本页记录本日核查结果，版本是否最新需要在后续发行时重新核对；不把版本号当作完整接入验收。

## 官方规范与实际支持

官方 [`latest`](https://modelcontextprotocol.io/specification/latest) 当前指向 [2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28)。默认 MCP 入口明确支持 `2026-07-28`，并兼容 `2025-11-25`；两代协议的实际 stdio 调用分别验证。工具发现、计算 Schema、结构化结果、资源读取、取消和错误处理使用官方 SDK，不建立另一套名为 MCP 的私有通信协议。

可选 [Streamable HTTP 入口](http-mcp.md)已实现并验证 `2026-07-28`，采用请求级元数据，关闭响应流取消实际计算；当前仍需调用方提供挂载文件通道。独立客户端已验证协议头、Origin、导出和取消隔离。这不代表任意客户端附件、公开部署或所有可选扩展已完成。[2025-11-25 HTTP](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports)具有不同的初始化/会话/取消语义，其兼容仍待实施；不能用 stdio 的两代通过代替 HTTP 两代验收。

Tools 与 Resources 是当前开放能力。Prompts、Tasks、UI 等没有被当前适配器声明为已支持；可选扩展需要按用途显式实现和协商。接入产品仍拥有认证、权限、持久任务、文件保存和页面，协议版本升级不改变 [ADR 0007](../decisions/0007-kernel-only-integration-boundary.md)。

## Rust SDK 补丁版本

核查时仓库锁定 `rmcp 3.4.0`，官方最新 release 为 [3.4.1](https://github.com/modelcontextprotocol/rust-sdk/releases/tag/rmcp-v3.4.1)。此前补丁阶段改为精确锁定 `3.4.1`，只改变独立 MCP workspace 的这一个 registry 包。后续 HTTP feature 的新增依赖另见[HTTP 组件记录](../../components/http-runtime/component.json)；纯计算、嵌入 SDK 和根 workspace 不新增网络依赖。

下载归档摘要与 Cargo registry 锁校验和一致。官方发布源码相对 3.4.0 的生产 Rust 差异只在 HTTP 客户端发现/回退处理，当前启用的 server/transport-io 模块未改变；上游还调整宏测试与开发测试依赖，MusterOffice 没有启用这些宏。新提交的完整许可文本与此前保留的文本逐字节一致，仍保留 MIT→Apache 过渡声明，不能只依赖 Cargo 的许可证字段。详见独立[3.4.1 组件记录](../../components/rmcp/versions/3.4.1/component.json)，旧组件和历史测试证据不覆盖。

新版实际二进制已通过 7 步 MCP 检查：默认 17 项、显式旧宿主 feature 18 项测试，其中共同 17 项不重复计数；两代 stdio 均完成实际创建/编辑/导入/导出与分块读取，8 次并发导出及 4 组取消/EOF 验证通过。两代协议各两版结果的 48 份资产、结果文件和索引与 3.4.0 阶段逐字节一致。完整输入、实际二进制与结果见[升级证据](../reviews/evidence/2026-09-27-mcp-sdk-341-verification.json)。

本轮没有重跑未变化的核心/WASM 或旧宿主 198 次完整协议流程，旧记录保持此前范围；本次检查不能代替完整 MCP 一致性认证或发行许可闭包。

## Plugin 与工具合同

Plugin 已形成[离线开发包](agent-package.md)：一份 canonical Skill、Agent Plugins 1.0.0 便携清单与生成的 Codex 兼容清单，配套原生 MCP/worker。官方便携 Schema、插件/Skill 校验器与独立客户端从搬迁后的包执行真实导出已通过；尚未安装或启用到用户客户端，跨版本升级和跨平台发行亦未验收。平台清单、实际工具激活和 Agent 使用流程分别验证，不能把便携规范或 JSON 校验当作所有平台已可用。

MCP 协议版本、官方 SDK 版本、MusterOffice 工具合同版本和平台 Plugin 清单是四个独立对象。`musteroffice.computation/1-draft` 等标记描述本产品尚未冻结发行的计算接口，不是对官方 MCP 标准的修改。完整一期目标仍要求完成实际能力与兼容、安装/升级、远程资源通道和独立客户端验证；未完成项继续记录于[实施目标](kernel-boundary-roadmap.md)。
