# Musterwork 原生 Skill 与隔离验收发行

后续版本：当前隔离候选已推进到 v53 / Skill release v27 / Skill v9，见[Agent 打开原生历史版本](product-native-open-agent.md)。本页保留 v52 / Skill v8 首次隔离发行的历史验证，默认 stable 没有切换。

2026-09-28，在独立产品 worktree 中继续接入。前一阶段 `author@5` 虽已能实际导出，默认目录、官方 Skill 和后端 Authority 仍各自选择旧发行，不能据此声称 Agent 已能正式发现原生工具。本阶段将原生 Skill 纳入产品现有发行与准入机制，并提供明确的开发验收配置。

## 同一次选择

产品新增代码持有的 `native-office-preview` profile：同一份闭合 JSON 声明选择 `device.v52` 与 Skill release v26；后者沿原不可变谱系增加 `musterwork-presentation-authoring@8`。Rust Tool、领域 Skill、Skill 包构建与 Python 官方 Skill/Authority loader 使用这份声明。当前默认仍为 `stable`，沿用 `device.v50` 与原 Skill v7；没有修改默认 active 指针或部署真实后端。

使用变量 `MUSTERWORK_BUILTIN_RELEASE_PROFILE=native-office-preview` 明确选择验收构建。Cargo 对该变量和清单建立重建依赖，Python 从相同字段推导代码目录；不接受任意路径、未知 profile 或空值，错误不能偷偷回退为 stable。配置只选择已存在的代码与材料，不能授予调用权限。后端发行摘要增加该 profile、选中 Skill 的完整前驱清单和源码材料，因此两种选择具有不同可复现身份。

此 profile 明确限定为 **Device private 开发验收**。原生 Skill v8 当前只声明 Desktop；它不构成正式 Cloud/Web 发行，也不覆盖 Shared Home。测试使用独立临时数据库，没有读取用户会话、发布 Authority 策略、启动生产后端或调用付费模型。正式多部署迁移仍须完成完整接入和门禁，不将该临时验收范围作为一期完成标准。

## Skill 和授权链路

Skill 继续采用 Musterwork 原有控制字段与包格式。原生 v8 从草案进入带材料摘要的 successor，使用与已有官方包/用户包相同的实际审计器，不靠删除控制字段来通过通用 Codex 校验。内容包括环境发现、公共 SDK 类型查询、创建/导入/原子编辑、两个修订条件、真实导出、诊断和质量状态；没有声明未实现的原生模板或历史 Artifact 重开能力。

ProductSnapshotBuilder 使用选中的实际工具目录和 Skill，按精确 Execution Grant entitlement 形成不可变准入快照。专项同时验证：选中原生 Skill 时存在且仅存在正确版本的创作工具；冻结的 Skill discovery 信息与工具版本匹配；移除所需的原生 capability 后选择同一 Skill 必须失败。Skill 不能自行创建权限，宿主权限仍在 Musterwork。

Device 接入补齐 v50 作为演示文稿的回放前驱。验收模式对新请求只公开 @5，同时保留 @3 原 HTML 处理器和 @4 原生前驱的精确合同。原历史请求不会被重新解释成新 SDK 动作。

## 验证边界与后续

具体命令、源码摘要、正反例结果与文件字节见[阶段证据](../reviews/evidence/2026-09-28-product-native-release-verification.json)。包审计和实际快照准入通过不等于外部模型已经使用该 Skill 完成任务；原生专项里的模型请求为受控测试输入，SDK、worker、SQLite 和 Artifact 提交实际运行。

本阶段 9 条命令完成：Python 发行选择 5 项、原生 Skill 包审计 3 项、验收及 stable 两种构建各 3 项准入、原生 Agent 专项 12 项、Device 合同 3 项、stable 三个目录库共 70 项通过。两种准入运行的是同一组测试，不累计为独立用例。四个生产库严格 Clippy 通过；未知 profile 的编译按预期失败，明确保留 exit 101 和诊断。四份实际 PPTX（@4、@5 各两版）与上一阶段逐字节相同，ZIP CRC 和 20 个 XML 部件检查通过；不据此推断视觉或完整编辑兼容。

下一步继续将已验证的发行组合接入完整测试会话与桌面进程，再完善原生模板、历史 Artifact 的继续编辑、字体交付和播放。当前正式后端签发/策略发布、真实模型调用及用户桌面完整验收仍未验证。返回 stable 必须使用自己的运行目录，不能把不包含 Skill v8 的旧二进制用于恢复原生验收数据库。

MusterOffice 内核没有增加业务发行、权限、存储或页面依赖。完整高级内容、Office/WPS 编辑往返、Native/WASM、性能/体积及 P00/E0–E3 继续开放；该阶段不能标记完整替换目标完成。
