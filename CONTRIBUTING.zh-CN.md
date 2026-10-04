# 参与 MusterOffice

[English](CONTRIBUTING.md) · 简体中文

先阅读 [README](README.zh-CN.md)、[架构](docs/architecture/overview.md)和
[已确认决定](docs/decisions/README.md)。使用问题在
[Discussions](https://github.com/wanquanY/MusterOffice/discussions) 讨论，
缺陷和功能建议通过 [Issue](https://github.com/wanquanY/MusterOffice/issues) 模板提交。
漏洞请走[私密安全报告](SECURITY.md)。

## 提交、审查与合并

1. 重要架构、格式及依赖变更先说明问题、方案、兼容影响和验收标准。
2. 外部贡献者使用 Fork；有写权限的维护者在当前工作区建立功能分支。
   未经明确要求，不另建 worktree。
3. 提交职责明确的修改，补齐相关测试与文档，保留他人已有改动。
4. 向 `main` 提交 PR，填写变更原因、验证结果和未验证项；可先创建 Draft。
5. 解决审查讨论，在最新提交上通过全部必要检查，并保持分支与主分支同步。
6. 外部贡献由维护者审查批准后 squash 合并，仓库内已合并的功能分支自动清理。

禁止直接推送 `main`、强推和删除受保护分支。维护者也必须经过 PR、CI 和安全检查。
单维护者阶段的审批安排及 GitHub 规则详见[协作治理](docs/governance/github-collaboration.md)。

提交说明应准确描述改动；PR 标题将作为 squash 提交标题。
不同目的的改动分开提交 PR，不擅自重写他人使用中的分支。
欢迎签名提交，目前不把提交签名作为强制门禁。

## 工程质量与验证

遵循 [AGENTS.md](AGENTS.md) 的职责边界、质量和文件组织要求。
计算核心不依赖账号权限、网络、存储或产品 UI；根因由所属模块解决，不能交给消费方补偿。

本地入口和原生组件准备见[当前验证](docs/implementation/current-verification.md)。
GitHub CI 从公开上游取得固定输入，校验摘要并构建原生组件，再运行同一验证入口：
Rust 格式、Clippy、测试、合同、TS 客户端、真实原生 worker、MCP 及文档 Native/WASM 一致性。
另检查 README 命令、模板文件、CodeQL 和新增依赖漏洞。

不得把失败改为跳过，不得放宽精度、内容保留、可编辑性或资源限制以通过测试。
Office/WPS、完整渲染一致性、产品链路和发行性能各自独立验收，CI 通过不代表全部验收。
提供可公开分发的最小复现；移除凭据和私有内容。渲染改动注明字体、平台、组件版本并附预览。
合同变动通过正式生成器同步 Schema 与 TypeScript 类型。

## 权利与行为规范

原创内容采用 [Apache-2.0](LICENSE)，贡献按同一许可证提交。
迁入代码、字体、模板、素材前核查来源和再分发权利，保留第三方声明。
当前不另设 CLA 或强制 DCO 签署，详见[许可治理](docs/governance/licensing.md)。
协作请遵守[行为规范](CODE_OF_CONDUCT.md)。

版本标签及正式发行由维护者负责。合并 PR 不等于授权发布软件包、部署 Musterwork
或声明完整兼容能力。
