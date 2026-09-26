# 原生线条继承与显式规则查询

2026-09-24：真实 PPTX 已可通过 Rust/Native/WASM 查询对象的派生线条样式，逐属性保留对象、主题或规则默认值的来源。文字编辑后重新查询保持相同语义；原始声明不补值、不改写。实现沿既定 Rust＋TS＋精选 C/C++ 路线推进。

本阶段将[来源声明](source-lines.md)连接到明确命名的 `ms-oi29500-lines-2024-draft-v1`。这是微软公开规则的一种待目标应用核对的实现；`resolved` 表示原生线条属性已取得声明或规则默认值，**不表示颜色已算成 RGBA、页面已绘制或 Office/WPS 保真已通过**。实际产物和检查见[封存证据](../reviews/evidence/2026-09-24-line-style-verification.json)。

## 入口与边界

`mo-pptx::source::line::resolve::query` 对不可变来源索引执行纯查询；`mo-kernel-api` 先读取调用方提供的真实 PPTX 字节，再验证请求的来源摘要。Native CLI 使用 `pptx-lines <query.json> <source.pptx>`，WASM 暴露 `resolve_pptx_lines(request, source)`，两者返回相同序列化结果。两份新增合同是 `pptx-line-query` 和 `pptx-line-response`，TS 类型由 Rust 同源生成。

请求必须提供 `expectedSourceSha256`、`surface`、原生对象 ID 数组 `objects` 及上述 `profile`。对象结果遵循请求顺序并保留重复 ID；空数组有效。来源冲突、无效对象/部件、预算超限和取消使整批失败。未建模的对象样式则给出逐对象 `unresolved`，不返回伪造的完整样式。

`EffectiveLine` 覆盖线宽、cap、compound、alignment、填充、虚线、连接及 head/tail 箭头。来源记录区分：

- 对象：部件、原生 ID、物理元素序号。
- 主题：有效主题或覆盖部件、声明序号、引用对象、引用序号和 style index。
- 默认：本次请求显式选择的规则。

来源索引、作者 Document 和 PPTX 的 XML 都不被查询修改。声明缺省仍是缺省；派生结果单独存在。查询未接入来源页面编译，也未开放线条属性修改。

## 属性选择

实现按当前对象直接线条、当前对象引用的主题线条、匹配版式对象及其引用、匹配母版对象及其引用的顺序寻找缺省属性；全部适用声明之后才使用本 profile 的默认值。原有占位符匹配负责 slide index 和 layout/master type 关系。歧义、未匹配或不支持的继承上下文在仍需继承时返回诊断。父链验证限定 slide → layout → master，不允许公开索引被修改后产生循环。

主题线条总是使用被查询页面的有效 format scheme，因此页面级格式覆盖也作用于继承来的 style index。主题引用从 1 开始；超出实际列表范围返回 `styleIndexOutOfRange`。本 draft 将 `idx=0` 解释为不执行主题列表查询，继续允许父声明继承，不凭空插入 `noFill`。该解释及引用优先级还需 Office/WPS 实测。

选择节点只在相同类型间补齐子属性。例如选中 `solidFill` 后，下层 `noFill` 不替换它；缺色可以由下层 `solidFill` 补足。已选 miter 可继续寻找下层 miter 的限值，下层 round 不改换连接类型。head/tail 的类型、宽度、长度分别继承。自定义虚线作为整体选择，**显式空列表同样阻止继承旧列表**。这些多遍查找与同类型合并原则来自 [MS-OI29500 DrawingML 默认属性说明](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/c0c046ec-a61d-405d-88fe-74d8487a37d7)。

`lnRef` 查找的是 `lnStyleLst`。早期标准关于它引用 fill list 的文字存在勘误，按 [MS-OE376 对 lnRef 的说明](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oe376/52cfd9cd-2a39-4f20-9364-70510270ffb9)处理。主题 `objectDefaults` 记录的是插入对象时的初始格式，本实现不把它当作现有对象的渲染继承层；来源声明仍保留，参见 [ObjectDefaults](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.objectdefaults?view=openxml-3.0.1)。

本规则的派生默认值如下，均不写回作者数据：

| 属性 | 规则默认值 |
| --- | --- |
| 线宽 / cap | 9525 EMU（0.75 pt）/ flat |
| compound / alignment | sng / ctr |
| 未声明填充 / 虚线 / 连接 | noFill / solid / round |
| 已选择 miter 但没有限值 | 800000（800%） |
| 箭头类型 / 宽度 / 长度 | none / med / med |
| 已选择 solidFill 但没有颜色 | bg1 原生主题表达式 |

这组值依照上述微软规则组织；本阶段未证明任何具体 Office/WPS 版本完全采用相同路径，不替换已有作者渲染规则。

## 颜色与保留内容

纯色结果保留原生六种颜色表示和有序变换。`phClr` 相关的引用颜色作为独立表达式和来源上下文携带；不在继承时采样或中途量化成 RGBA8。显式 `0`、带符号整数百分比及 `800.123456789%` 等原始百分比词法不会丢失。后续颜色计算需要显式主题、系统色和占位色上下文，并验证各层上下文的应用优先级。

当前绑定优先使用所选声明所属对象的 `lnRef` 颜色，缺失时保留继承链第一个引用颜色上下文。这是 draft 解释的一部分；LibreOffice 实测存在不同结果，不能视为已确认的跨应用语义。`resolved` 中也可能含仍待数值求解的主题/系统颜色。

适用声明或引用含未知保留属性/扩展时，返回 `retainedContent`；选中渐变/图案填充返回 `unsupportedFill`。高优先级已选择 noFill 或 solidFill 时，未被选择的下层渐变不会改变该选择。没有把未建模内容降级为黑色、空线条或位图。

## 资源约束

查询默认最多 256 个对象请求、100 万步、262144 项遍历值及 4 MiB 词法/来源预算。同批次重复请求共享预算。进入克隆前检查变换、虚线列表、百分比字符串和来源长度；扫描对象、逐级求值均检查取消。来源读取仍先执行已有 OPC、XML、MCE 和声明预算。

这些限制约束本查询；不代表整个产品的生产实例池、并发调度、浏览器 Worker 或峰值内存门禁已经完成。

## 实际验证与差异

本阶段全仓 269 项 Rust 测试、严格 Clippy、rustfmt 和 51 份 Schema/TS 检查通过。新增 8 项 Rust 测试；此前 49 份 Schema 与 TS 合同均未修改。

新增 28 份自有 PPTX，覆盖逐属性继承、页面主题覆盖、直接与主题引用优先级、空列表/空节点、限值缺省/0/精确小数、颜色顺序及保留内容。Native/WASM 共 144 批新查询，包含 28 批真实文字编辑后重新查询、77 批既有线条声明输入和 11 批协议边界。候选文件字节、响应字节一致。

另一套 Python ZIP/lxml/MCE 参考从文件关系与 XML 构建层次，再按属性列选择结果，不使用内核 SourceIndex；独立核对 118 批成功查询、124 个对象的完整结果和来源。28 份编辑候选除指定文字之外的 XML 和部件均保持不变。224 个投影部件通过官方 XSD；刻意加入未知属性的一个部件单独记为非法，不混入正向合规统计。参考只覆盖这批自有唯一占位符语料，不宣称任意第三方 PPTX 的兼容性。

此前 2825 批主内核用例在当前产物重新执行，旧响应、像素及编辑输出保持不变；累计 **2969 批**。197 批 Skia 组件/sanitizer 和 61445 个组件参考像素沿用相同组件源码与二进制摘要的历史证据，本轮没有重新执行；旧几何参考同样按实际帧摘要核对后复用。

使用 LibreOffice 26.2.0.3 的隔离临时配置，另取得 24 份实际 PDF 并读取第一页线条属性。观察范围和原始 PDF 摘要独立保存：

| 自有用例 | 当前 draft 查询 | LibreOffice PDF 实际观察 |
| --- | --- | --- |
| 本地主题引用与父对象宽度同时存在 | 选择主题宽度 4 pt | 约 3.99679 pt，支持此样本的顺序 |
| idx=0，父对象直接线条 9 pt | 继续继承父线条 | 约 9.01403 pt，未将 0 解释为清空父线条 |
| 空 custom dash + 主题虚线 | 保留空列表 | 与主题用例输出相同非空 dash 数组 |
| 父对象直接宽度 + 父对象主题引用 | 补齐主题填充等缺省项 | 第一页未观察到 stroke 路径 |
| 页面覆盖 + 继承来的主题引用 | 选择覆盖中的 10 pt 线条 | 第一页未观察到 stroke 路径 |
| 主题 solidFill 的宽度和颜色缺省 | 派生 0.75 pt 与 bg1 表达式 | PDF width=0，颜色约为 RGB(52,101,164) |

另有直接 `phClr` 继承引用颜色的样本输出白色线条。上述差异均保持开放；PDF width=0 只是原始绘图参数，不能推断视觉上完全没有线宽。未对这些样本运行 WPS/PowerPoint，也没有做应用内编辑往返或内核像素对照。LibreOffice 是验证工具，不是运行依赖，更不是代替目标应用验收的标准答案。

没有新增外部依赖或锁文件变动。未压缩 CLI 为 4478944 字节（增加 75072），Rust WASM 为 3990220 字节（增加 68192），JS glue 增加 744 字节；raster worker、两个 C/C++ WASM 组件及薄适配器字节未变。文字 worker 大小保持 2375376 字节但摘要变化，已在新产物上重跑相关回归。这些均为未完成内核的开发产物，不能作为 Musterwork 安装包或性能收益估计。

## 复现与继续推进

先按[开发说明](development.md)构建同版本 Native/WASM 并准备既有来源语料及官方 XSD 缓存。Python 使用已记录的 lxml、PyMuPDF 和 jsonschema，仅供开发验证。

```sh
python3 tools/verification/line-style-fixtures.py
node tools/verification/line-style-parity.mjs
python3 tools/verification/line-style-independent.py
python3 tools/verification/line-style-observations.py --soffice /explicit/path/to/soffice
python3 tools/verification/contracts.py --line-style-report .codex-work/line-style/parity.json
```

`line-style-evidence.py` 要求完整回归和检查日志，核对实际产物、输入/输出及历史摘要；默认验证，`--seal` 独占创建新证据，不覆盖旧记录。完整复现还需运行各历史实现文档中的回归入口，不能只替换旧报告的 artifact digest。

后续已接通[原生线条颜色计算](line-colors.md)，来源页面编译及目标应用证据继续推进。完整填充/效果、页面文字/图片、可编辑高级对象、播放和 Agent/Musterwork E0–E3 继续按原方案实现；本阶段仍不具备替换 Musterwork 的条件。
