# 原生几何公式与坐标求值

2026-09-25 更新：预设展开和显式来源合同已实现，当前状态见[预设展开](preset-expansion.md)。以下保留此前自定义几何阶段的实现和验收记录。

2026-09-24：在[来源几何声明](source-geometry.md)之上实现 17 种 DrawingML 导引公式，并计算自定义几何的路径坐标、调整手柄、连接点和文字区域。Rust 计算通过 Native CLI、WASM 和范围读取 API 接入真实 PPTX；原始文档声明保持不变。[封存证据](../reviews/evidence/2026-09-24-geometry-evaluation-verification.json)记录源码、产物、实际输入/输出和独立核对。

**本节封存时的结果是原生路径空间中的计算值，尚未成为可绘制页面。** 预设声明仍明确返回 `presetNotExpanded`；预设展开、几何继承、路径到形状空间缩放、弧线编译、完整填充和页面编译继续实现。通过数学检查不能代替完整形状或 Musterwork 替换验收。

后续已经接通的局部坐标缩放、椭圆弧和 Q32 路径输出见[原生路径编译](native-paths.md)。本说明及其封存证据保留公式求值阶段的验证范围。

## 入口与计算边界

CLI 为 `pptx-geometry <query.json> <source.pptx>`，WASM 为 `evaluate_pptx_geometry(request, source)`。`mo-kernel-api::evaluate_pptx_geometry` 接收范围读取器、明确长度、来源/计算预算和取消检查；`mo-pptx::source::geometry::evaluate::query` 供已检查的不可变来源索引复用。

本节历史请求使用 `ecma376-2016-ms-guides-draft-v1`；当前接口必须显式选用 `ecma376-2016-ms-presets-draft-v2`，派生来源字段也已升级，见[预设展开](preset-expansion.md)。请求仍含 `expectedSourceSha256`、`surface`、`objects`、`profile`。该配置仍是 draft，没有宣称某个 Office/WPS 版本已通过验证。结果保持请求顺序及重复对象查询，逐对象返回 resolved 或带源节点位置的 unresolved。

resolved 结果包括对象的有效尺寸与其来源、调整值和导引结果、手柄、连接点、文字区域及路径。路径宽高、填充/描边/挤出声明仍保留缺省状态；这些默认值属于后续路径与画笔编译，不能由本阶段随意补齐。未声明文字矩形也不会被替换为猜测框。

目前只计算对象直接声明的自定义几何，尺寸可复用已有位置/尺寸继承的结果。组、缺几何、缺尺寸、保留的未知几何属性和未展开预设分别有明确诊断，不会用矩形或空路径替代。

## 顺序、依赖和数值规则

先按文件顺序计算调整列表，再计算导引列表。公式最多三个参数，使用固定数组和词法迭代，不构造脚本、不递归追逐前向引用。重复的自定义名称读取此前已经定义的值，再更新后续引用；每条输出仍保留自己的源节点、名称、结果与实际引用的导引节点/内建量。手柄的 `gdRef*` 绑定调整列表节点，不能将普通计算导引伪装成可编辑调整值。

依赖顺序依据 ECMA-376 Part 1 §20.1.9.11。内建量按 §20.1.10.56 的明确词汇表，从局部形状宽高计算；不从对象在页面中的绝对位置取得 `l/t/r/b`。对标准列出的内建名称重新赋值暂明确诊断，不以未验证的覆盖优先级继续求值。条件公式的三个参数均先解析，因此未选分支的未知引用也会报告。

工作值采用 binary64，超越函数复用已有固定版本 `libm 0.2.16` 的 `force-soft-floats` 配置；不在中间导引边界量化成整数或 Q32。运算顺序固定，输出前拒绝非有限值并规范化负零。词法原件继续在来源索引和原始文件中保存。

- 17 种操作为 `*/`、`+-`、`+/`、`?:`、`abs`、`at2`、`cat2`、`cos`、`max`、`min`、`mod`、`pin`、`sat2`、`sin`、`sqrt`、`tan`、`val`。
- `mod` 使用三维欧氏长度；`sqrt` 先取绝对值，遵循微软对标准说明的修正。[MS-OE376 gd 说明](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oe376/9aa189a4-c55b-4259-94e6-538ac00c7c55)
- 当前 draft 对反正切采用完整象限，角度单位为 1/60000 度；正弦/余弦保留精确的直角值。零方向和正切极点明确未解析。`cat2` / `sat2` 先缩放方向分量，避免平方导致不必要的溢出。
- 路径坐标可引用已算导引，或使用原生整数/带单位坐标；角度字面量验证原生范围。单位转换包括 mm、cm、in、pt、pc、pi，结果仍处于原生计算阶段。

完整象限、重名处理、奇异点、词法和浮点规则均是本配置的显式解释，仍需目标应用核对。标准公式定义可参见 [ShapeGuide.Formula](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.shapeguide.formula?view=openxml-3.0.1)。数学结果的容差验证尚未转化为整页像素误差上界，后续路径编译须承担该检查。

## 资源与失败

默认每批至多 256 次对象查询、1,000,000 个计算步骤、8 MiB 累计词法处理量、262,144 个值/命令记录。查找、公式、参数、坐标和元数据都计入同一个批次预算，重复查询不会刷新预算。引用查找借用原始名称，普通坐标计算不分配临时依赖字符串；只有导引结果需要持有的依赖边才生成输出记录。

除零、前向或缺失引用、参数数量错误、未知操作、无定义方向、正切极点、数值溢出和非法元数据均返回定位诊断，不合成零。取消、来源摘要冲突、非法请求和预算超限使整批失败，不返回半批计算结果。纯计算核心不持有文件路径、网络或隐式字体。

## 真实验证与数据问题

全仓 **292 项 Rust 测试**通过，本阶段新增 7 项。严格 Clippy、rustfmt、55 份 Schema 和 TS 检查通过。新增 `pptx-geometry-query` 与 `pptx-geometry-response`，此前 53 份 Schema 和生成 TS 字节不变。新增代码按类型、数学、内建量、会话、记录转换和查询分离；没有扩大 `mo-geometry` 的通用路径职责。

602 份实际 PPTX 输入包含标准附带 187 条定义在横向、纵向和正方形尺寸下的 561 个变体，以及全部公式、象限、单位/坐标、元数据和失败边界探针。560 份成功求值，42 份明确未解析。560 份又执行了真实文字编辑，Native/WASM 候选文件字节相同，编辑后几何结果不变。

新增 **1168 批 Native/WASM 检查**，包含源摘要、未知对象/表面、预算、非法 profile 和未知字段的失败请求。独立实现从 ZIP/XML 重建记录和物理序号，使用 80 位 Decimal、Machin 公式及独立级数核对反正切/三角函数，共检查 **38698 个数值**；不以另一入口调用同一个 libm 充当独立数学参考。

预设容差为 `|actual − expected| ≤ 1e−7 + 2e−11 × max(|actual|, |expected|)`。所有数值满足该检查，最大容差占比约 0.0023284。最大绝对差 0.000625 出现在约 9.64×10¹² 的中间导引值；接近零处最大相对差为 1，对应绝对差约 2.33×10⁻¹⁰。两项均保留定位信息，不隐藏相对误差，也不能把混合量纲的导引容差当成页面像素误差。

3611 个表面部件通过官方 XSD。一个故意含未知属性的保留探针单独排除。560 份编辑候选仅改变授权文字，其余 XML 与部件相同。旧 4039 批主内核检查在新产物上重新执行，响应、编辑文件和帧不变。另有下述 9 份应用探针的真实跨端查询及 54 个 XSD 部件检查；合计本阶段 **611 份输入、1177 批新增检查**，累计 **5216 批主内核 Native/WASM 检查**。独立 Decimal 数值计数对应前述主语料，不包含这 9 份应用探针。

标准附带示例在此阶段暴露了额外问题：循环箭头的部分公式含多余参数；若干记录引用了文档明确内建列表以外的 `cd3`、`hd10`、`wd12`、`wd32`。本轮没有删除参数或猜测导引值。Apache POI 的公开[内建导引实现](https://raw.githubusercontent.com/apache/poi/trunk/poi/src/main/java/org/apache/poi/sl/draw/geom/BuiltInGuide.java)包含 `wd32`，但本项目尚未完成这些额外名称和标准示例修正的目标应用验证，也没有复制其代码。此前发现的重复 `upDownArrow` / 缺 `upArrow` 仍记录在[来源几何说明](source-geometry.md)。这些都是完整预设实现后续必须关闭的项。

### 应用实际观察

LibreOffice 26.2.0.3 已分别打开另外 9 份自有 PPTX 并导出 PDF。每份第一页包含一个由导引控制宽度的红色矩形；下表记录 PDF 路径边界，单位为 pt。公式、文件摘要、PDF 摘要及全部边界保存在封存证据中。

| 探针 | 标准规则或待核假设对应宽度 | LibreOffice 实测 | 当前内核查询 |
| --- | ---: | ---: | --- |
| 一英寸字面量对照 | 72 | 72 | 72 |
| `wd32`，假设为宽度 / 32 | 22.5 | 0.0560 | 未定义引用 |
| `hd10`，假设为高度 / 10 | 28.8 | 0.0560 | 未定义引用 |
| `wd12`，假设为宽度 / 12 | 60 | 0.1410 | 未定义引用 |
| `cd3`，假设为一圈 / 3 | 240 | 0.6520 | 未定义引用 |
| `+-` 含第四参数，假设忽略额外参数 | 144 | 144 | 参数数量错误 |
| 负数开方，微软规定先取绝对值 | 72 | 0 | 72 |
| `at2 -1 1`，以原生角度数值作为坐标 | 637.795276 | 637.795029 | 637.795276 |
| 除零 | 无定义 | 0 | 除零诊断 |

字面量对照一致；反正切探针支持完整象限的解释，其差约 0.000247 pt。负数开方与微软说明存在实际差异；四个额外导引名的观察也不支持直接按名称猜测除数。极窄路径边界不能用于反推应用的精确导引值。多余参数和除零观察不构成静默修正文件的依据。当前保留这些差异及 draft 状态，继续核对目标 Office/WPS；不为了匹配单个应用而修改标准解释。

同一批 9 份 PPTX 已通过 Native/WASM 查询字节一致检查，3 份求值成功、6 份明确未解析，并通过全部 55 份合同 Schema 检查。这是应用边界观察，不是内核渲染像素对照或应用编辑往返；本轮没有 Office/WPS 运行。

现有几何参考与 197 批 Skia 组件/sanitizer、61445 个参考像素按未变帧/组件摘要复用，本轮未重跑该组件测试。原生工作进程摘要发生变化，其主内核文字/渲染回归已在新产物上重跑。

没有新增外部依赖版本；Cargo.lock 只增加 `mo-pptx → 已有 libm` 依赖边，pnpm 锁文件不变。当前开发产物的实际字节数与增量见封存证据，不代表完整内核或 Musterwork 安装包，也没有端到端速度/内存验收结论。

## 复现和后续

按[开发说明](development.md)构建 Native/WASM，并准备来源几何语料和固定摘要的官方标准文件。Python 的 lxml、Decimal、jsonschema 仅参与开发验证；应用观察另使用显式指定的 LibreOffice 和 PyMuPDF。

```sh
python3 tools/verification/geometry-eval-fixtures.py
node tools/verification/geometry-eval-parity.mjs
python3 tools/verification/geometry-eval-independent.py
python3 tools/verification/contracts.py --geometry-eval-report .codex-work/geometry-eval/parity.json
python3 tools/verification/geometry-formula-observations.py --soffice /path/to/soffice
node tools/verification/geometry-application-parity.mjs
python3 tools/verification/geometry-application-check.py
python3 tools/verification/contracts.py --geometry-eval-report .codex-work/geometry-eval/application-probes/parity.json
```

`geometry-eval-evidence.py` 核对真实产物、回归、输入/输出、锁文件和历史摘要，默认只验证，`--seal` 独占创建新证据。下一步继续确定预设数据修正与解释规则，完成路径空间转换和弧线编译，再把来源几何、填充及线条接入页面编译。完整文稿、高级对象、播放、Agent 协议和 Musterwork E0–E3 仍属于原目标。
