# ADR 0004：标准 Agent 接入与统一办公操作合同

> 2026-09-27：本提案已被 [ADR 0007](0007-kernel-only-integration-boundary.md)部分取代。SDK/MCP/CLI/Skill/Plugin 和同源计算方向保留；MusterOffice 自带持久任务/资源宿主及产品 UI 的安排不再是当前交付要求。权限、存储、持久任务与产品页面归接入方，普通 SDK 无需这些业务设施。以下原文保留用于追踪旧设计与实现。

- 状态：**Proposed（本轮接入架构）；跨 AI 产品便捷接入的目标已由用户确认**。
- 日期：2026-09-24。
- 依据：用户要求明确 Musterwork 的接入形式，采用 Skill、MCP、Plugin 等成熟机制，让其他 AI 产品快速接入独立办公内核。

## 本轮设计选择

1. 以 **MCP Server 作为通用 Agent 调用入口**，提供 stdio 本地和 Streamable HTTP 远程/自托管形态；不依赖 Musterwork 账户、数据库或 Runtime。
2. 提供遵循 Agent Skills 格式的技能包，描述创作、编辑、检查和交付工作流；技能通过 MCP 或 CLI 调用同一能力，不包含第二套生成器。
3. Plugin 是面向具体产品的安装、配置和权限声明封装，组合 Skill、MCP 配置及必要宿主适配；不把某一家 Plugin manifest 冒充跨平台通用协议。
4. SDK/CLI 和原生嵌入作为同等受支持的接入方式。实时播放、文件字节及资源传输使用 SDK/宿主数据通道，不经过模型上下文或逐帧 MCP tool 调用。
5. 所有入口映射同一版本化 Operation Contract。协议适配、技能说明和平台包装不拥有布局、格式及质量规则，也不为每个产品重写引擎。
6. Musterwork 以官方集成包接入：Agent 侧 Skill＋MCP 工具，Runtime/Content Store/Artifact 由明确宿主适配器连接，Web/Desktop 的显示和播放用 SDK。其他产品可只配置通用 MCP 即完成端到端任务。
7. MCP Tasks、Skills over MCP 和 MCP Apps 在声明支持后使用；基线工具调用不依赖客户端全部实现这些扩展。扩展缺失不等于办公能力缺失，必要文件交互与显示限制必须明确。

## 责任与验收

官方提供独立运行的标准宿主，负责本地/自托管资源和任务持久化；嵌入产品可注入自己的宿主，沿用其任务所有者，不建立两个竞争提交者。核心本身继续无文件/网络/身份和模型依赖。

一期至少验证：独立本地 MCP、独立自托管 HTTP MCP、Skill＋CLI、嵌入 SDK、Musterwork 全链路；另用一个独立第三方 MCP 客户端验证，无须修改内核或接入产品私有代码。具体客户端/构建及发行渠道待冻结。

详见[接入总设计](../architecture/agent-integration.md)和[MCP／Skill／Plugin 合同](../design/agent-interfaces.md)。本 ADR 没有创建可安装包、启动服务、注册域名或发布插件。
