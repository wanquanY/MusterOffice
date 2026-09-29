# Musterwork Agent 打开原生历史版本

2026-09-28，在独立产品 worktree 将[原生发布版本恢复端口](product-native-reopen.md)接入 Agent。新增 `author@6 / device.v53` 与匹配的 Skill v9，实际工具调用已能从已发布原生文稿恢复新草稿、编辑页面文字、再次导出同一个 Artifact。此阶段没有切换默认 PPT 引擎，也不代表旧 HTML 历史或完整替换验收完成。

## 不可变合同与宿主责任

新动作参数为 `{"operation":"open","parameters":{"artifact_id":"<UUID>","expected_version":"1"}}`。Artifact ID 和版本使用规范表示；版本不是浮点数，也不能由模型猜测。宿主按照原调用的租户、会话读取指定已发布版本，复用既有 `OpenPublished` 和 checkpoint /5。准备阶段与最后提交分别验证来源；原事务再次核对最新 Artifact 版本与依赖有效性。

来源是发布时的原生快照，原草稿后来未发布的修改或放弃状态不改变来源。新草稿保留原内核文稿、资源和基线，新的产品草稿身份由调用身份确定。后续 `apply` 同时需要产品 `expected` 与内核 `baseRevision`，`export` 沿用已发现的不可变环境。不会重新导入 PPTX 以冒充无损历史恢复。

终态调用通过原 Tool journal 重放已提交结果；准备处理器只接受运行中的调用。重复准备复用输出账户，重复原事务提交返回首次收据。没有给 MusterOffice SDK 新增权限、历史数据库或业务任务职责，也没有改变固定 SDK/worker。

`author@4/@5` 的输入合同继续拒绝 `open`，旧 HTML `author@3` 的历史处理器保留。`artifact_engine_mismatch` 明确表示需要原引擎或后续显式迁移；新动作不能悄悄丢弃旧 AST。旧版本另存副本、历史回退和外部改稿仍需各自明确的产品语义。

## Skill 与发行选择

隔离的 `native-office-preview` 配置现在共同选择 Device v53 和 Skill release v27 中的 `musterwork-presentation-authoring@9`。Skill 解释原生历史打开、两个修订条件、资源恢复和再次导出，并明确未完成的模板与 HTML 迁移。旧 Skill v8 和工具 v51/v52 内容继续保留，默认 stable 仍为 Device v50 / Skill v7。

Rust 工具/Skill 构建、Python Authority 声明与 Skill loader 共用原有闭合配置。选择候选配置不能授予能力，实际会话仍需精确 `author@6` 授权。候选 Skill 仅在 Desktop 可用；即使授予全部所需能力，也不得被 Web、API 或 Channel 准入。没有部署后端、发布软件包或修改用户正式会话。

## 实际结果与验证范围

原生 Agent 回归 15 项通过，其中 3 项为新打开动作，12 项覆盖已有创建、导入、编辑、环境、取消和恢复。使用真实 SQLite、产品原子处理器、公共 SDK 和固定 worker；这是受控调用测试，还不是外部模型驱动的桌面完整会话。

新专项在数据库重新打开后恢复已放弃草稿对应的已发布版本，将第一页标题文字由 `A A` 改为 `AA A`，随后发布同一 Artifact 的版本 2。两份 PPTX 均通过 ZIP CRC 和全部 XML 解析，文字仍是 DrawingML 文本，唯一变化的 ZIP 部件为 `ppt/slides/slide1.xml`；第二页 XML 与预览保持相同。输入使用自有合成字体，不能据此推断中文排版或商业成稿质量。原 @4/@5 四份回归 PPTX 与前阶段逐字节一致。

首次专项错误地把终态调用再次直接交给准备处理器，被运行状态检查拒绝；测试改为核对原 journal 与重复事务收据，没有放宽生产授权。发行回归还暴露了旧测试对固定 Skill v7 和全平台范围的假设；测试现在分别验证两种构建，并将已超过 1200 行的 Skill 库测试移入独立文件。原始失败和每次源码摘要均保留。

发行目录及相关库在 native/stable 两种配置下各通过 70 项；真实 Skill 包审计 3 项、Device 组合 3 项、两种配置各 3 项会话准入、Python 发行加载 5 项均通过。连同 Agent 回归，共 99 个独立测试，在两种配置下累计 172 次最终成功执行；六个选定生产库的严格 Clippy 通过。Agent 回归之后仅改动了四个文件中的测试组织与断言，后续 Rust 门禁共用同一份最终源码摘要，未重新计算或替换 Agent 产物。全部命令、早期失败及文件摘要见[阶段证据](../reviews/evidence/2026-09-28-product-native-open-agent-verification.json)。

下一步继续原生模板、旧 HTML 历史迁移、真实模型/桌面完整会话和共享 Home 接入。字体分发、播放、高级内容、Office/WPS、Native/WASM 一致性及完整 P00/E0–E3 质量、性能和体积门禁保持开放；完整一期目标没有缩减。
