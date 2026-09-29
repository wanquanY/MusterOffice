# 原生文稿标题投影与保留式写回

2026-09-28。Musterwork 的原生模板导出验证发现：原生导入没有读取包内文稿标题，而来源编辑器禁止 `SetTitle`。产品导出按既有规则要求有效 Artifact 标题，因此真实模板草稿被拒绝。修复落在内核的文件语义层，产品校验保持原样。

## OPC 元数据

`mo-opc::read_core_properties` 从包级关系定位 core properties，不假定输入一定采用 `docProps/core.xml`。关系类型采用 [Microsoft 对 ISO/IEC 29500 的实现说明](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/28beaa8f-42ce-41e6-820d-4d7af34457a5) 指定的 package metadata URI；校验关系、内容类型和命名空间后读取直接 `dc:title` 文本。

`RewritePlan::set_core_title` 是类型化、固定来源摘要的元数据修改。已有标题只替换其文本；没有标题时添加独立命名空间的标题元素；没有 core properties 时创建唯一部件并补充必要的关系与内容类型声明。未知元数据、注释、属性、其他内容及 UTF-8/UTF-16 编码保留。未改变标题的写回保留原包字节。

新增 XML 子元素操作会验证插入前后的展开名称，防止父节点的默认命名空间改变子元素语义。容量、取消、部件冲突和签名策略仍由原 OPC 边界检查。封存后重新读取真实结果并核对标题，不将计划中的字符串当作已落盘结果。

## 来源版本

新导入使用 `presentationml-retained-fields-v2-draft`，标题进入正常 Document 语义摘要和原子编辑。原生来源的 `SetTitle` 与文字、二维变换走同一个候选写回计划。

已有 V1 快照中的空标题表示当时未投影该字段，不能解释为删除源文件标题。因此 V1 保持原有投影及编辑约束；无修改导出仍保留原包，不自动变更旧快照及 revision。需要新版标题语义的来源通过显式重新导入建立新快照。

## 验证状态

专项覆盖实际原生导入、原子编辑、导出再导入、非标准部件名称、缺失元数据、编码保留、旧快照、容量和输入冲突。全仓 1,048 项 Rust 测试及严格 Clippy、格式、Schema、TS 和原生导出检查通过；普通测试中另有 25 项忽略，不全部计为已执行。

30 组文稿计算与六份 Schema 的原生/WASM 对比通过，包括 V2 标题修改和 V1 继续禁止标题编辑。两代 MCP 实际修改标题并导出后，只有 core properties 部件改变，页面预览保持原样；13 组模板完整结果/诊断在 SDK 所用计算、CLI、WASM、MCP 间一致。相关 33 项协议门禁全部通过。

产品新 SDK/worker 上的原生模板实例化、标题编辑和真实导出通过；此前失败暴露的限制没有在产品出口绕过。原始命令、材料、失败与修复后结果见[阶段证据](../reviews/evidence/2026-09-28-product-native-templates-verification.json)。这里不宣称 Office/WPS 外部应用或完整替换验收通过。
