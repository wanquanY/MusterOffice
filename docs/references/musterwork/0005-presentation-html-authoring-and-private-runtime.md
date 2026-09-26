> Musterwork 历史设计快照 · 迁入日期：2026-09-24。来源：`docs/architecture/agent-runtime/adr/0005-presentation-html-authoring-and-private-runtime.md`。
>
> 原文中的“当前”“已采纳”和商业 SDK 选择只代表来源项目当时的状态，不是 MusterOffice 的决定或验收结论。内核以[现行设计](../../design/presentations.md)为准；来源、摘要和链接转换见[迁移清单](README.md)。

# ADR 0005：HTML 演示文稿事实源与私有 Runtime 执行

> 状态：Normative
>
> 版本：v1.1（2026-09-22）

## 背景

固定模板槽位无法表达用户对整页结构、布局、图片及图表的自由修改。用户已明确选择 Web/Desktop
只提供预览、由 Agent 编辑、WPS/Office 手动编辑，并授权完整设计落地到开发环境。

## 决策

规范化声明式 HTML AST 是唯一可变文稿。模板实例拥有完整节点树与稳定页面/节点 ID，模板引用仅保留
精确来源，不限制实例后续布局。HTML 渲染、测量场景、对象映射、原生 PPTX 和页图均由指定修订派生；
不持久化两个可以分别修改的 HTML/JSON 源。

引擎编译受支持的原生文本、形状、表格、图表及图片。最终产品预览必须来自实际交付 PPTX；HTML 预览
仅用于创作验证。不能用整页截图宣称原生可编辑，也不能承诺未知 PPTX 对象无损转换。

Backend 模板域拥有上传意图、权限、不可变目录版本、逐页 checkpoint、租约和目录发布。模板编译
是同一 Runtime 的私有执行配置，复用 AgentLoop、Provider 适配、模型/工具 Journal、使用量事实、
Content Store、完整工具轮次恢复和输入容量估计。它们不创建持久 Agent、不接入 Memory Dream 队列，
也不由 Python 再执行一套模型循环。业务策略只选择任务材料、阶段工具和可验证提交结果。

普通 Cloud/Device 与私有 Run 均按物理模型响应恢复全部工具调用、结果及精确 Provider replay。
续接材料独立内容寻址并核对摘要，不作为公开推理文本；不能为节省输入拆散一个多工具响应。
容量来自已冻结路由及共享估算，包括指令、工具 Schema、文本、图像和续接信息；不能以固定字节上限
冒充模型 token 窗口。历史投影裁剪不删除 Journal，业务产物仍通过授权 ContentRef 读取。

2026-09-14 决策修订：视觉判断由创作 Agent 通过实际页面图片完成；普通创作不依赖独立评审模型。
`artifact.presentation.author@3` 的 compile 在确定性校验通过后同事务保存 Artifact、文件和工具收据，
草稿继续开放，后续改稿再次 compile 推进同一 Artifact。没有模型可调用的 review/publish 交付门槛。
编译成功表示原生输出与文件校验通过，不宣称视觉或 Office/WPS 验收通过。

模板转换 `presentation.compilation@2` 复用真实 Tool Calling：Agent 主动调用读取、编辑、编译、
prepare_stage 和提交工具；提交工具核对来源、压力样本及最终文件闭包。文本/JSON 宣称无法完成阶段。
历史冻结协议仍严格校验原始证明，只用于旧任务恢复；不要求新任务配置独立评审模型。

WPS/Office 修改保留为本地文件，预览读取当前文件字节并撤下旧 QA 状态；新 Agent 版本另存。用户显式
“继续编辑导入”建立新创作基线，“上传为模板”执行参数化，两种意图不能自动互换。

2026-09-22 执行位置修订：文稿、工具收据与 Artifact 仍由所属 Device/Cloud Runtime 唯一提交，
编译与最终文件渲染则通过明确的环境适配器执行。Desktop 目标是本机受管独立进程，Web 目标是通过
功能等价验收的浏览器执行端；未覆盖范围保留现有合格执行方式。按
[客户端渲染方案](presentation-client-rendering.md)逐范围切换，不能用 HTML 截图、
缺失字体或删减模板能力替代原有质量合同。

桌面普通创作、模板维护和工作副本预览应共用受管宿主及依赖组件。组件包含版本、完整哈希、许可证、
隔离执行与取消边界，不依赖客户机 PATH、Homebrew 或 Electron 启动时偶然继承的环境。后台服务配置
固定组件代次；请求和产物通过受限暂存文件交接。选择本地执行后的故障不能静默转为上传用户素材。
组件发行、统一预览入口和 Web 执行桥是否完成以实施记录为准，不能把此决策视作上线证明。

## 规范与验证归属

- PRD FR-12（`Musterwork:docs/prd/agent_runtime_prd.md`）拥有产品行为。
- [HTML 演示文稿设计](presentation-html-authoring-and-template-compilation.md)拥有机制与 H01–H35 验收场景。
- 字段合同 §5.10（`Musterwork:docs/architecture/agent-runtime/rust-agent-runtime-field-level-contracts.md`）指向 Rust、Protobuf、引擎及产品 API 唯一 Schema。
- 发行合同（`Musterwork:docs/architecture/agent-runtime/builtin-capability-release.md`）拥有不可变 Tool/Skill successor 与历史回放。
- 实施记录（`Musterwork:docs/plans/agent-runtime/presentation-html-authoring-implementation.md`）拥有当前进度、实际执行证据和未通过项。
- 客户端执行记录（`Musterwork:docs/plans/agent-runtime/presentation-client-implementation.md`）拥有本地宿主、发行与浏览器迁移的实施证据。

本决策采纳设计，不声明全部模板样本和 Office/WPS 场景已验收，不授权生产部署或产品打包。
