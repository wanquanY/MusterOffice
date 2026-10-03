# 自建形状和单元格的文字命令

状态：计算实现；共享编辑器和产品验收仍在进行；导入文字进展见[保留文字命令](editor-retained-text.md)。

## 目标身份与算法

`TextEditCommand.object` 标识所属对象；自建表格必须另外携带稳定 `cell: CellId`。
形状不能携带 cell，表格不能省略 cell；不能用物理行列序号或段落 ID 猜测目标单元格。
合并区域只能编辑左上角 origin，covered cell 的文字仍为将来拆分保存，但不是交互编辑目标。
输入端点的段落必须同属目标文字体，跨单元格选区被拒绝。

形状和自建单元格使用同一个字素边界、跨片段/跨段落替换、样式 patch、身份派生及选区映射算法。
单元格展开为普通 `EditTable.SetCellText`，只替换目标文字体；形状仍展开为 `SetText`。
其他单元格的内容、合并关系、样式及表格布局保持原声明，资源与全图验证仍由事务核心执行。

`TextRangeChange.cell` 标明单元格上下文，反向映射保留它。形状命令和范围映射省略 cell 时的
规范 JSON、命令摘要及派生身份规则不变。不存在另一个“表格文字状态”或产品侧偏移计算器。

## 没有文字体时初始化

原始空单元格 `text=None` 没有模型 ParagraphId；不能使用排版合成空段落的序号创建选区。
`TextEditAction.initialize` 接受文字和显式 `TextBodySetup`：文字体样式、内边距、换行、溢出、
段落样式和段落默认片段样式。宿主选择所需格式，内核创建全部 ParagraphId/RunId，
复用同一 CRLF/CR 归一化、段落预算、字素检查与确定性身份规则。文字中的制表符转成有独立
RunId 的 InlineContent.Tab，不放进禁止控制符的普通 Text 片段；替换和初始化共用此转换。
默认和显式左制表位的换行、glyph 原点及选区共用原生排版，具体范围见[Tab 排版](editor-tab-layout.md)。

初始化仅适用于没有文字体的自建形状或可见自建单元格。已有文字体必须使用 replace；
重复 initialize 不覆盖现有内容。空字符串也创建一个合法的空文字段落，便于后续输入。
结果返回真实新选区，并省略 rangeChange，因为不存在可映射的旧模型段落。
撤销通过普通历史事务恢复 text=None，不留下虚假的空段落身份。

Setup 是明确的新建格式，不是从屏幕像素推导的继承结果。已有文字的普通编辑不会重新应用 Setup，
不会解析并写回全局字体/主题样式。初始化与替换的非法格式、跨文稿、旧 revision、取消和预算错误均原子拒绝。

## 历史与范围

候选仍包含普通 Transaction、回执、commandDigest、新 Snapshot 与选区；宿主持有历史材料和提交 CAS。
普通历史以既有对象声明为冲突边界，同一表格的后续修改可能保守冲突，不能据此覆盖当前表格。
本增量没有引入单元格并发合并或改变历史冲突规则。

RetainedSource 不属于自建 TextBody；初始化和样式命令不会绕过原生字段保留约束。
[保留文字高层替换与文字能力查询](editor-retained-text.md)复用这些目标前提；原生结构/格式与产品输入/IME 继续实施。

## 验证入口

`mo-presentation-edit/tests/text_cells.rs` 验证合并 origin、覆盖/错误/跨单元格目标、初始化、
Unicode、跨段落样式/合并、空文字、预算/取消/版本及撤销重做。
`tools/verification/editor-cell-text-parity.mjs` 比较实际 Native/WASM/TS 命令、历史及重新渲染结果。
运行程序、依赖、输入/输出摘要与执行范围见[验证证据](../reviews/evidence/2026-10-03-editor-authored-cell-text.json)。
这些检查不替代真实浏览器/Electron、Windows/Linux、Office/WPS、性能或完整产品验收。
