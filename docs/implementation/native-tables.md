# 原生表格模型、原子编辑与写出

2026-09-29，F04.01–F04.05 的进行中实现。此阶段补齐此前对象模型没有表格的问题；**不是完整表格验收，也没有提升为 Musterwork 的固定 SDK**。完整一期和替换目标保持进行中。

## 领域模型和编辑

[Table 模型](../../crates/mo-presentation-model/src/table.rs)以稳定行、列、单元格 ID 表达网格；这些 ID 在表内唯一，段落和 run ID 仍在文稿内唯一。每行保留每列对应的物理单元格。合并起点持有矩形跨度，其余单元格指向起点；网格校验拒绝悬空覆盖、缺格、重叠、越界、重复身份和坐标溢出。

合并只改变可见区域的归属，保留被覆盖单元格的文字、样式和身份，不隐式拼接文字。拆分任意成员可恢复整个区域。删除行列只删除其物理内容；若原起点被删除，剩余左上角单元格以自己的内容成为起点。上述是内核事务语义，尚未证明外部应用编辑、回存也保留覆盖单元格的内容。

[EditTable](../../crates/mo-presentation-edit/src/table.rs)通过既有文稿事务完成替换、文字与样式修改、行高列宽、合并拆分、行列增删及完整重排。插入穿过合并区域时扩展跨度；重排必须保持矩形和原起点，否则整次事务拒绝。普通 `SpliceText` 可按原段落/run ID 编辑单元格，沿用 Unicode scalar 和 AnchorMap。取消或失败均不发布半成品。

网格的精确 EMU 总宽高同时是表格外框尺寸。行列变更在同一事务同步外框；只修改外框、使其与网格不一致的操作无法提交。位置、旋转和组变换仍使用既有对象模型，不新增宿主页面逻辑。未来整表比例缩放需作为明确的网格尺寸操作实现，不能留下两套互相矛盾的尺寸。

验证预算按整份文稿累计物理单元格，包括被覆盖单元格，默认上限为 1,000,000。文字及字体仍走原验证和依赖闭包；被覆盖文字也不能绕过字体引用、重复身份和文字预算检查。索引使用借用和整数前缀和，不复制文字；这不是性能验收结论。

## 原生写出

作者计划将表格降到 [SourceTable 声明](../../crates/mo-presentation-source/src/source/table.rs)，复用同一文字目录、颜色及描边转换。[Writer](../../crates/mo-pptx/src/native/table.rs)输出 `p:graphicFrame / a:tbl`、原生行列、物理 `a:tc` 和 `a:txBody`，没有转为矩形组或图片。

合并写出每行/列的 `rowSpan / gridSpan / hMerge / vMerge`，保留覆盖单元格文字。边距写入 `tcPr`，不重复写入文字框内边距；直接填充、六向边框、竖向对齐及文字方向/水平溢出写入对应原生字段。表格自身的可见填充/描边尚无定义，不会静默丢弃；显式无填充/无描边可归一为空的图形框声明。

参考依据包括 [Microsoft 对表格合并的补充规范](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oe376/de4562ba-7901-48df-9803-414ded131f2b)及仓库已锁定 ECMA-376 XSD。没有迁入第三方表格实现、字体或模板。`mo-presentation-model` 增加对工作区既有 `thiserror` 的直接依赖；没有增加新的外部组件版本或 C/C++ 库。

## 验证与可复现输入

416 项相关 Rust 测试通过，其中新增 12 项表格测试；严格 Clippy、完整工作区编译、Schema 同步及 TypeScript 类型检查通过。初次相关回归发现共用描边帮助函数改变旧组对象错误信息，已保持原错误行为并重新通过；没有修改测试来接受回归。

- [模型测试](../../crates/mo-presentation-model/tests/tables.rs)：精确矩形、错误合并、身份、限额、溢出、取消和序列化。
- [事务测试](../../crates/mo-presentation-edit/tests/tables.rs)：504 种合并矩形与插删位置组合、重排、覆盖文字、Unicode 锚点、所有已观察取消检查点、尺寸同步、无效样式回滚、全局文字身份及字体依赖。
- [原生写出测试](../../crates/mo-pptx/tests/native_tables.rs)：物理单元格、原生合并标记、文字及属性、确定性、输入不变和拒绝不可表达外观。
- [自有输入](../../fixtures/presentations/native-tables/request.json)与[独立检查程序](../../tools/verification/native-table-export.py)：实际 CLI 导出，PresentationML 部件及 DrawingML 表格 XSD 验证，独立 python-pptx 读取表格结构和全部单元格文字。
- [Native/WASM 对照](../../tools/verification/native-table-parity.mjs)：14 次初始化、事务或错误请求返回完全一致；11 份原生表格导出逐字节一致。[独立编辑后结构读取](../../tools/verification/native-table-readback.py)进一步检查全部 11 份文件的 119 个物理单元格、合并关系、文字、行列尺寸和外框，以及 66 次 XSD 校验。这些不包括像素或外部应用操作。

复现结构检查：构建当前 `mo-cli` 后，用 `python3 tools/verification/native-table-export.py --cli target/debug/mo-cli --request fixtures/presentations/native-tables/request.json --schema-dir <已锁定XSD目录> --output <新的诊断目录>`。程序拒绝覆盖旧结果。命令、日志、材料摘要和实际验证结果保存于忽略的 `.codex-work/native-tables/`；汇总见[阶段证据](../reviews/evidence/2026-09-29-native-tables-verification.json)。

## 尚未完成及接入边界

此阶段覆盖作者模型和写出基础，F04 的所有适用维度尚未完成。原生表格 Reader、来源绑定与局部回写、共享排版/绘制、边框冲突及表格样式级联、文字驱动行高、整表缩放和单元格动画仍需实施。现有有限形状绘制 API 对表格明确返回未支持；不能以写出成功生成空预览。

WPS/Office 打开、编辑与回存、Native/WASM 像素一致、实际产品链路、性能和安装包体积均未由此阶段验收。5,998 字节样本只是该自有文稿的文件大小，不能用来推算内核或桌面安装包。

Musterwork 对接继续使用[既有独立 worktree](product-worktree.md)，原主工作区留给用户。当前产品固定 SDK/Worker/WASM 仍以前一[路径效果身份阶段](flat-motion-presets.md)为准；表格垂直链路完成后再统一升级和验证。没有切换默认引擎、部署或发布。

## 后续来源链路

2026-09-29 后续已实现[原生表格来源与文字编辑](table-source.md)，包括单一文字所有权、来源局部回写和历史投影兼容。上文“Reader 尚未实现”描述本基础阶段的历史状态；共享表格渲染、有效样式和完整替换继续实施。
