# 原生交付复核计算

2026-09-28，Musterwork 独立接入工作区增加 `office_template_inspect`，通过公共 SDK 重新检查已保存的真实交付。该端口为最终模板验证提供可复用计算；它尚未完成 `native_verify` 的维护任务接线、视觉复核和目录发布。

## SDK 与产品职责

SDK `ReceivedDelivery::preview_measurements()` 返回在同一次 `inspect` 中读取并核对的逐页测量；`NativeExportCandidate` 提供相同只读访问。记录包含页面 ID、证据资产 ID 和可选文字容量。原文件缺少测量时仍返回 `None`。原交付回执、字节格式及质量声明没有改写；接收校验不会重新渲染，也不会把生产者的测量转换成独立视觉质量证明。

Musterwork 只把已有内容引用映射到 SDK `DeliverySource`。复核输入是明确的生产结果 JSON 引用、完整角色到内容引用的映射，以及已接受的文稿/版本/语义摘要、设置摘要和 renderer。端口沿用原计算额度、期限和字节预算，检查：

- 固定的编译档案、实际字体和当前渲染环境，与原生产结果一致。
- 每个输入的真实字节、声明的媒体类型和角色，覆盖完整且没有多余输入或角色。
- SDK 回执与独立传入的预期文稿身份一致；随后由原 SDK 检查完整来源、快照、PPTX、页面、预览和质量文件。
- 显式快照与 SDK 实际接收的文稿一致，跨内容库引用变化不改变内部 JSON 文件字节。

结果输出一份 `presentation-native-delivery-inspection/1` 文件。文字容量区分 `overflow`、`unmeasured` 和 `measured-fit`，溢出保留精确页面、证据和对象测量。`quality_proven` 保持 `false`；原 SDK 的各项质量声明保留。这些状态只描述文字容量，不替代布局设计、可编辑性、播放或 Office/WPS 验收。

## 实际验证

31 项相关内核测试、产品计算合同、实际交付复核与维护任务回归通过，见[证据](../reviews/evidence/2026-09-28-product-delivery-inspection-verification.json)。内核及接入库的严格 Clippy 通过。

实际复核使用上一阶段 worker 生成的来源与 8 类压力实例，读取原保存文件后重新赋予外层内容引用，保持文件本身不变。共 9 份交付、18 页、72 个文字框；再次定位 `text-maximum-latin-wide` 第二页的横向溢出。其余样例在有限文字容量这一维度报告 `measured-fit`，全部保持完整质量未证明。

实际端口同时拒绝错误文稿 ID、revision、语义摘要、设置摘要、被改动的输入字节及缺失文件覆盖。测试中的生产文件来自真实渲染，但复核测试本身不重新调用 renderer，也不是外部模型视觉复核。维护任务另行使用新 SDK 和原固定 worker 验证真实生产路径。

## 材料与后续

固定 SDK 更新为 `7ed23b21a12b0cacc4cf563ead1b60133a6b5659c3a235881c910d5ece232471`。worker 继续使用 `52bdebdbebd0ef7feca35e4f66213ba237e7ef48a3f6b248d0ac168c71dfcc50`：本次变更属于调用方接收和读取接口，渲染线协议与输出不变。没有新增依赖或改变正式候选选择。

原 Musterwork 最终模板合同还要求对精确预览进行视觉复核。现有复核候选绑定旧文稿媒体类型及 scene/object-map 等必需材料，不能给原生文稿填入虚构的旧字段来绕过。下一步应在产品现有复核所有权内建立原生候选的明确绑定，再连接 `native_verify` 的逐例计算、模型复核、完整证明和原目录事务。模型“通过”文字、容量成功或 SDK 的 `not_proven` 都不能替代这些证据。

本阶段没有新增模型调用、产品页面、权限或存储系统。MusterOffice 继续只负责文档计算及薄接入，产品工作继续限于[独立 worktree](product-worktree.md)。完整替换仍按[一期目标](progress.md)推进。
