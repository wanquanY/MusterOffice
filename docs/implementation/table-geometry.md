# 原生表格网格与文字范围

2026-09-29，接续[来源读取与单元格文字编辑](table-source.md)，实现来源网格的合并拓扑、声明尺寸计算和借用式文字范围。此阶段为共享排版提供输入，尚未完成表格页面渲染，也不改变 Musterwork 的固定计算材料。

## 所有权与有界计算

[网格计算](../../crates/mo-presentation-source/src/source/table/grid.rs)按物理单元格解释 `rowSpan/gridSpan/hMerge/vMerge`，保留覆盖格数据，通过矩形归属检查拒绝缺失连接、空洞、矛盾跨度与重复 ID。行列地址、原始单元格和合并区域分别表达；读取某格的归属不需要重新扫描整表。单元格数、遍历步骤、ID 字节均有预算，遍历检查取消。

[文字绑定](../../crates/mo-presentation-source/src/source/text/binding.rs)验证原生对象、单元格地址和文字根，借用原对象的物理段落范围，不复制或合成另一份文字。表格准备时一次核对完整段落区间；后续单格绑定通过有序根索引查找。读取器使用增量根身份集合检查重复，避免每读一格就重新扫描已有全部文字根。普通形状也复用此绑定检查。

[声明几何](../../crates/mo-presentation-compile/src/source_table/geometry.rs)计算实际网格中的物理格与合并矩形，支持 RTL，并保留零尺寸。EMU 和十进制单位复用[精确数值转换](../../crates/mo-presentation-compile/src/source_number.rs)，逐边累计 Q96 区间后转换为 Q32，同时输出误差上界。只有一个高精度累计值常驻，不为每个边界保存大整数；RTL 从另一端累计。负尺寸、坐标范围、词法与累计预算失败均明确返回。

这里计算的是声明行高与列宽。文字引起的行高增长、表格框与网格尺寸不一致的最终语义、单元格内边距和裁剪、表格样式及边框竞争仍需在共享页面编译器中完成，不能把这些矩形直接作为完整表格视觉结果。

## 验证与接入

[拓扑测试](../../crates/mo-presentation-source/src/source/table/grid/tests.rs)穷举 3×3 网格的 322 种矩形划分，分别使用省略与冗余覆盖跨度；另测损坏合并、预算边界及取消。[来源几何测试](../../crates/mo-presentation-compile/tests/source_table_geometry.rs)对照作者声明与实际 PPTX 读回，检查精确单位、RTL、合并、累计舍入、文字所有权、缺失文字体与 4,096 格输入。真实字体后端及已有来源接口另作回归，最终结果记录在[阶段证据](../reviews/evidence/2026-09-29-table-geometry-verification.json)。

403 项来源、格式和编译测试（新增 11 项）及 10 项真实文字后端测试通过，严格 Clippy、全工作区编译、Schema/TS 同步通过。11 份既有表格完成 55 次 Native/WASM 来源对照及独立 119 格、66 次 XSD、55 次 JSON Schema 验证；作者事务的 14 次请求和 11 份导出也重新通过。全部原导出和文字编辑后的文件摘要与上一阶段相同。

新网格几何当前是 Rust 库计算接口，尚未进入公共页面输出；不能把来源读写的 Native/WASM 对照说成新表格渲染跨端验收。本轮不进行 Office/WPS 视觉验收、性能或安装包体积结论。Musterwork 继续在[原独立接入 worktree](product-worktree.md)推进；完整表格链路验证后才统一升级产品材料，完整一期目标继续有效。

后续[表格样式来源与声明选择](table-styles.md)已接通共享/内联声明和逐项选择，文字编译与页面渲染仍按上述边界继续实施。
