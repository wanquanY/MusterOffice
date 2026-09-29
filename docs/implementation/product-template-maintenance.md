# 原维护 Agent 的原生模板定义阶段

2026-09-28，Musterwork 独立工作区将[原生模板计算端口](product-template-engine.md)接入原 `PresentationTemplateLoopPolicy`、工具日志、内容库和维护 Run。本阶段完成整份来源的 `native_define`；后续[压力任务](product-template-stress-maintenance.md)已接通有限案例计算与提交证明。自动上传协调器尚未选择这条流程，最终质量与目录发布仍在实施。这不是完整模板维护或 PPT 替换验收。

## 明确的阶段与策略版本

原来按页重建 HTML 的阶段语义保持。新增 `presentation.compilation@3`、`presentation-compilation-policy/3` 和 `presentation-template-stage-output/3`，编译档案固定为 `presentation.office-template@1`。原 `presentation-template-compilation/1` 请求封套继续承载明确的策略与阶段，不改变历史请求字节。

| 阶段 | 当前状态 | 文稿范围 |
| --- | --- | --- |
| `native_define` | 已实现原维护 Agent、真实计算、独立阶段验证与原任务完成 | 完整原始 PPTX |
| `native_stress` | 后续已实现固定计划、逐例计算与独立覆盖验证；完整质量待验收 | 完整原生模板及实例 |
| `native_verify` | 仅有独立阶段身份；计算执行明确拒绝，尚未实现 | 实际压力、交付与来源证据 |

Python 请求模型和 Rust 快照分别拒绝原生阶段、旧策略、旧编译档案的混用。原生阶段的 `page_index` 在调用侧是 `-1`，在快照中明确缺省，不用第 0 页代替整份文稿。设置摘要作为新策略的必填材料进入原请求身份和封存清单。签名前重新校验可变的 Python 调用模型，避免模型构建后的改动绕过阶段约束。

## 模型调用与计算责任

原生策略使用原工具名称及独立的 `operation@3 / submit@3` 身份。新增模型指令明确其整份来源语义；模型可进行以下操作：

1. `describe` 返回已安装 SDK 生成的公共 `TemplateDefinition` Schema 文件，模型沿原有界 `read` 分页读取。
2. `prepare_input` 从封存清单取出精确来源、文稿身份和设置摘要，通过同一个 OfficeHost 渲染完整来源并返回真实快照、资产与页面预览。
3. `define_template` 只接收显式参数定义；来源文件、身份和环境由宿主绑定，核心检验来源版本、语义摘要及可编辑目标。
4. `prepare_stage` 接收已计算的模板引用，根据实际产生它的工具记录和文件字节构造阶段回执。
5. 原 `submit` 工具独立重建并比较证明，才允许原 Agent Loop 和 SQLite Run 完成。

计算入口增加 `office_template_contract`，读取公共 SDK 的同源 Schema，不复制另一套参数规范。读取器明确支持其 Schema 媒体类型和既有原生文稿媒体类型；仍保持原字节界限、分页及内容授权规则。工具结果中的句柄只来自宿主结果封套，读取到的任意 JSON 不产生新的引用权利。

没有新增模型循环、业务任务、数据库或目录提交入口。MusterOffice 仍只提供文稿计算；上面提到的封存、权限、工具日志和任务完成全部属于接入产品 Musterwork。

## 阶段完成证明

`native_define` 通过的能力范围仅为 `source-and-parameter-definition`。验证器要求：

- 同一封存请求、整份来源、编译档案、实际字体和设置摘要；来源字节与 ContentRef 一致。
- 模板必须来自真实 `define_template` 结果，包摘要与 SDK 描述一致，模板快照等于实际来源渲染快照。
- 来源文档 ID、revision、semanticDigest 与定义一致；资源映射保留原始 PPTX。
- 全部 SDK 交付资产都映射到实际存储文件，逐项核对摘要、长度和媒体类型；保留逻辑资产身份与无损去重。
- 提交内容必须来自真实 `prepare_stage`；回执全部字段与独立重建值一致。

缺少计算记录、把 `read` 结果称为计算结果、替换环境或来源、添加自称通过的质量字段均不能完成阶段。定义阶段不宣称布局、压力、动画播放、Office/WPS 编辑往返或模板目录发布通过。来源交付的五类声明原样保留，当前自有样本只有有限结构检查通过，其余仍为 `not_proven`。

## 实际验证与剩余工作

真实专项运行原模型循环、SQLite 工具日志、内容存储及实际 native worker。提供方是脚本测试替身，用来验证工具协议和任务执行；它不代表外部模型质量或生产会话验收。一次完成包含 11 次脚本模型请求、3 次原生计算，覆盖公共 Schema/来源回执的分页读取、参数定义、阶段生成和提交，提前提交被拒绝。另有 7 组证明变造/缺失检查被拒绝。

实际来源为自有两页 PPTX，10735 字节、21 个 ZIP 部件。12 个 SDK 资产均验证实际字节，模板快照与来源快照一致，两页 PNG 与前阶段基准逐字节一致。保存的阶段证明不能据此提升质量声明。

Python 与 Rust 使用同一份新增原生阶段已知结果材料，分别构建、校验签名和恢复规范快照；历史已知结果文件保持原样。详细命令、源码摘要、失败修复与范围见[阶段证据](../reviews/evidence/2026-09-28-product-template-maintenance-verification.json)。

自动上传入口、原 CompilationJournal 的原生阶段推进、参数压力与质量验证、原目录版本事务、Cloud/Web 路由及真实模型会话仍待实现。页面组装、HTML 历史迁移、完整高级内容和 P00/E0–E3 继续按总目标推进。固定 SDK/worker、候选 `device.v54 / author@7 / Skill v10` 与默认 stable 选择均未改变。
