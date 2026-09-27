# Agent Skill 与 Plugin 开发包

2026-09-27 · [K3](kernel-boundary-roadmap.md) 的离线开发包装已实现，独立客户端已从搬迁后的包完成真实计算。当前没有公开发行、安装到用户 Agent 客户端或启用 Musterwork 新入口；完整 PPT 和产品替换目标保持不变。

## 单一内容源与规范

[canonical Skill](../../integrations/skills/musteroffice-presentations/SKILL.md) 提供发现能力/Schema、创建/导入、版本化原子编辑、导出、检查和交付流程；资源、字体和附件由宿主提供。四份参考文件按需读取，不在 Skill 中复制完整合同或维护另一套演示引擎。高级内容按已实现能力操作，不把保留原始部件、静态预览或降级图片冒充完整创建、编辑与播放。

[打包器](../../tools/agent-package/README.md)消费这份 Skill、[身份清单](../../integrations/plugin.json)和两个显式原生程序，生成同一目录：

| 文件 | 作用 |
| --- | --- |
| `plugin.json`、`mcp.json` | Agent Plugins 1.0.0 便携身份和 stdio 配置 |
| `skills/musteroffice-presentations/` | 唯一 Skill 副本；含 Codex 发现元数据 |
| `.codex-plugin/plugin.json`、`.mcp.json` | 从同一数据生成的 Codex 兼容清单；仍引用同一 Skill 和程序 |
| `bin/mo-mcp`、`bin/mo-export-worker` | 同平台 MCP 与配套计算 worker；Windows 使用 `.exe` |
| `runtime.json` | 平台、架构与 worker 实际 SHA-256 |
| `bundle-manifest.json` | 文件长度/摘要/执行标志清单；明确 `releaseCleared: false` |
| `README.md`、`notices/` | 宿主配置说明及仓库已记录的上游许可材料 |

本阶段依据官方 [Agent Plugins 1.0.0](https://agent-plugins.org/specification)、[Agent Skills](https://agentskills.io/specification)和 [OpenAI 插件文档](https://developers.openai.com/plugins/build/plugins)。便携清单用 `./bin/mo-mcp`，由客户端按插件根目录解析；参数中的 `${PLUGIN_ROOT}`、`${PLUGIN_DATA}` 只展开一次。兼容配置显式指定根目录为工作目录。MCP 协议版本、工具合同、Skill 版本、Plugin 规范和平台安装能力仍是不同对象。

官方两个 JSON Schema 以 URL 和 SHA 固定；[开发验证器](../../tools/agent-package/validate.py)只接受对应本地副本，禁止自动远程解析。OpenAI 提供的插件验证器与 Skill 验证器另行执行。清单验证不能证明客户端已正确注册、选择或执行 Skill。

## 运行和宿主边界

宿主创建自己的输入、输出、临时目录，通过它管理的 `PLUGIN_DATA/caller-files.json` 传入绝对路径和有界并发数，具体例子见[包说明](../../integrations/package-README.md)。这只是宿主配置文件，不是 MusterOffice 文稿库。Agent 仍需宿主提供已授权文件/附件工具；当前 MCP 不会因安装插件而自动获得任意附件或网络资源。

`mo-mcp --package <runtime.json> <caller-files.json>` 绑定当前包内 worker，拒绝调用方另行指定 `exportWorker`，也拒绝错误平台、损坏摘要和重定向的包文件。配置和结果都保持宿主所有；内核不创建账号、数据库、持久 Job 或产品页面。计算仍共用原 SDK/文件桥接，输出保持 `productCommitted: false`，由产品负责保存和提交。

替换包时不需要把 worker 路径写回调用方配置；已有进程退出后，宿主可以用新目录启动。已验证的是**同一版本/构建换目录后的配置复用**，尚未声称未来版本之间的二进制/合同兼容。宿主不得在进程运行期间改写包目录。卸载不能删除宿主文稿，文稿也不得放入客户端可能随插件清理的 `PLUGIN_DATA`。

运行只使用两个原生程序及对应系统库；Python/jsonschema 是开发验证依赖，Node/Python 不属于此 stdio 包的运行前提。未捆绑用户文稿、字体、图片或 Musterwork 源码，也未添加安装脚本、后台服务和运行时下载。HTTP 仍是[独立可选入口](http-mcp.md)，不进入默认本地包。

## 已执行验证

[阶段证据](../reviews/evidence/2026-09-27-agent-package-verification.json)绑定源码、实际二进制、归档、官方 Schema 和独立调用结果。

- macOS arm64 实际二进制组装、ZIP 解压到含空格及字面量 `${PLUGIN_DATA}` 的路径后运行；包内程序使用空 PATH，工作目录为包根，无 Cargo/组件路径环境依赖。两个独立组装归档逐字节一致，执行位和完整清单保持。
- 两代 stdio 分别完成创建、原子文字修改、两版真实导出及两次重新导入；每代 24 份资产经 MCP Resources 回读，合计 48 份与历史未打包基准逐字节相同，收据、索引、检查报告也一致。夹具仅两页自有文稿/合成字体，不代表商业文稿验收。
- 两代各 18 类无效启动拒绝，包括平台/架构、未知字段、重复键、超限/无效 UTF-8、相对/缺失路径、worker 覆盖、损坏、缺失、目录与符号链接；宿主配置和已有结果保持原样。两代分别通过兼容清单启动第二个包，并读回同一宿主结果。
- 5 项包装边界测试、3 项验证驱动测试、官方便携 Schema/插件/Skill 检查通过。11 步 MCP 编译/测试/严格 Clippy 通过：默认 17、旧宿主 feature 18、HTTP feature 22 项存在重叠，不累加为独立测试数。
- 原普通 stdio 入口另外重跑两代实际协议、1 MiB 分块读取、8 次并发导出，以及两代取消/EOF 共四组真实子进程停止和暂存释放。包装入口没有改变计算输出或持久化责任。

初次 Rust Clippy 暴露 `PathBuf` 借用错误，修复后完成构建与所有上述门禁；首次 Skill 校验器不接受其 `compatibility` 字段，改为正文需求说明后通过。失败记录保留，没有改写为成功。本机 Python 默认 CA 配置不能读取官方 Schema，后续以系统 curl 正常 TLS 验证获取，未关闭证书校验。

本轮未改动计算核心、根锁文件、SDK/WASM、Musterwork 或用户客户端安装配置；没有重跑未变化的完整核心/WASM/HTTP 外部流程。已有外部验证继续保留原输入范围。

## 尚未完成

开发包不是发行包。实际客户端安装/启用/更新/卸载、LLM 选择 Skill 的真实行为、Windows/Linux 执行、签名/公证、完整传递许可闭包、远程附件通道和 2025 HTTP 兼容仍需逐项验收。现有 NOTICE 汇总不等于发行许可已完成；本阶段 ZIP 来自 debug 构建，不能用来预估 Musterwork 最终体积或证明性能收益。

Musterwork 接入继续使用公共 SDK/MCP 与自己的任务、文件和 Artifact 通道；它无需引入一个办公业务平台。完整高层渲染/播放、全部高级内容、Office/WPS 编辑往返、用户链路与同条件性能/体积门槛仍按 [K4/K5](kernel-boundary-roadmap.md)推进，包装通过不等于可立即替换原 PPT。
