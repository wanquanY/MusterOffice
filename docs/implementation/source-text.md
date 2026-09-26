# 原生文本声明

2026-09-25。本阶段把真实 PPTX 的文本样式接入既有来源索引，供后续继承、字体选择和页面排版使用。原有文字叶节点编辑继续使用相同物理来源校验；没有把 HTML/CSS 引入内核。

这是**声明读取与编辑保留**，尚未完成文本继承或来源页面文字绘制。完整一期、Office/WPS 可编辑互操作及 Musterwork 替换要求不变。

## 数据与责任

`SourceIndex.text` 保存演示文稿默认文本样式，`SourceSurface.text` 保存对象文本体、母版文本样式与对象 `fontRef`。每个目录由 `roots`、`nodes`、`effectNodes` 构成：

- `roots` 保留物理元素序号和部件内对象 ID；文稿/母版默认根没有对象 owner。
- `nodes` 按物理元素序号保存类型化值、父子次序和未解释内容位置，避免递归复制同一来源树。
- 文本字符串仍由既有段落/run 读取和编辑投影持有；目录中的 `t` 节点只绑定来源，不再复制字符串。
- 文字填充、轮廓、下划线画笔和效果共用既有 DrawingML 读取器。文字效果节点位于该文本目录，不能与形状效果目录混用。
- 字体声明使用从主题读取器提取的公共 `SourceTextFont` 类型和解析器；主题原有公开类型路径仍保留。

源码按[类型](../../crates/mo-presentation-source/src/source/text.rs)、[属性](../../crates/mo-presentation-source/src/source/text/attributes.rs)、[枚举](../../crates/mo-presentation-source/src/source/text/names.rs)、[文法](../../crates/mo-presentation-source/src/source/text/grammar.rs)和[流式读取](../../crates/mo-presentation-source/src/source/text/read.rs)分工。使用已有 CLI `pptx-inspect` / WASM `inspect_pptx` 响应，不新增另一套源文件读取链路。

## 已解释范围

| 属性族 | 当前声明范围 |
| --- | --- |
| 文本框 | 19 个原生属性；旋转、边距、分栏、竖排、溢出、换行、锚点、三类 autofit，以及精确缩放/行距减少值 |
| 段落/列表 | 11 个段落属性；段前/后和行距、默认与九级列表样式、最多 32 个制表位、项目符号颜色/字号/字体/follow/no/字符、41 种原生自动编号名称 |
| 字符 | 19 个属性；语言、字号、粗斜体、下划线/删除线、大小写、字距、基线、校对和书签数据；Latin/EA/CS/symbol 字体元数据 |
| 结构/样式 | 普通 run、break、field、段落结束和默认字符属性；`fontRef` none/major/minor 与颜色；高亮、下划线、填充、线条及效果声明 |

13 类原生枚举、103 个取值与固定 ECMA `dml-main.xsd` 核对；事实文件摘要为 `6978ba7e889070b0c3cb5b546b23e5a6c3516134afc53b87a21f482ca33f3858`。缺省、显式 false/zero/none、空字体名保持区别。坐标保留精确词法；字距区分百分之一点整数和 universal measure，不先转浮点。

百分比 union 的整数成员按对应范围检查，百分数字符串按原生词法读取，不提前变成有效布局值或执行隐式钳制。字段 ID/type、超链接关系/action 仅为来源数据，不执行、解析关系或生成交互。`xml:space` 当前保留为未解释属性，原始文本不做空白归一化。

WordArt/3D、图片项目符号、扩展、超链接声音及未知属性/子树保留来源位置和原始包字节；不把它们解释成已实现视觉语义。已识别子节点检查顺序、互斥、基数和数值；未知内容保留也不意味着该内容符合 XSD。

## 预算与合同

文本样式预算跨演示文稿主部件和全部已索引表面累计：默认最多 100 万元素、32 MiB 属性值 UTF-8 字节；未知和委派子树也计入。委派填充/线条仍消耗各自预算。现有 XML/OPC 限制和取消检查继续生效。

Rust → Schema → TS 仍为唯一生成链。总计 66 份运行时 Schema，本阶段只扩展 `pptx-source-response`。其 TS 文件超过工程行数上限，因此生成器按完整 AST 类型声明拆分，原模块路径重新导出全部名称，跨文件依赖使用 type-only import。文档注释保留，禁止切断声明或提高 2000 行上限。

拆分工具使用已有锁定 TypeScript 7.0.2 的 `unstable` AST API，启动时校验版本。升级该开发依赖必须同时验证生成器；没有新增运行依赖或修改 Cargo/pnpm 锁文件。独立测试覆盖 382 个声明、四个模块、循环引用、继承、公开入口、负类型赋值、确定性和拒绝过大/未知声明。

## 实际验证

[冻结记录](../reviews/evidence/2026-09-25-source-text-verification.json)绑定源码、构建产物、输入、响应、修改候选及验证日志；前一份来源页面记录保持不变。

- 392 项 Rust 测试、严格 Clippy、rustfmt、Schema 同源和 TS 编译通过。本阶段新增 8 项 Rust 测试。
- 145 份自有 PPTX：124 份可读取文本声明，21 份预期错误；其中 124 份经过真实叶文字编辑、Native/WASM 候选字节比较和再次读取。新增 269 批跨端检查。
- 旧 9501 批全部重跑：7854 批不变，1647 批只增加文本目录；旧数值、错误、编辑候选和像素不变。当前累计 9770 批，批次不是功能完成百分比。
- 独立 XML/MCE 校验 620 个目录、8437 个节点、4119 个显式属性和 498 个文本叶节点；124 份编辑仅改动指定 `a:t`，另有 2480 个未修改压缩部件逐字节相同。
- 独立 XSD 检查 246 个有效修改前/后 slide 部件、六个额外 presentation/master 样式部件；23 个 slide 检查按预期失败，其中含刻意保留的未知属性及其编辑候选。此类保留样本不声称格式有效，部件检查也不等于完整包认证。

本机 macOS arm64 / Apple M4 Max / Node 23.5.0 的未压缩开发产物：CLI 6071648 字节、Rust WASM 5259609 字节、Native 绘制 worker 5556608 字节，较前一阶段增加 109200/99256/56688 字节。文本 worker 经共同的 `mo-kernel-api` 依赖 PPTX 层并重新构建，摘要变化，大小仍为 2375360 字节；已用于本次回归。C++ WASM、glue 和薄适配器摘要未变。这不是完整内核或 Musterwork 安装包，未新增产品延迟/内存测量。

## 后续闭环

下一步连接原生文本的逐属性继承、主题字体与显式 FontManifest，向已有段落塑形/布局/路径管线提供可追溯有效输入，再合成来源页面。需继续完成自动缩放、段落/列表布局、竖排、文字效果、样式/树编辑、高级内容及应用实测。

例如 `normAutofit` 中的比例和行距减少只是声明；[Microsoft 的元素说明](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.normalautofit?view=openxml-3.0.1)不能代替目标应用的实际换行和适配验证。当前来源页面仍对可见文本整页报未支持；本阶段没有 Office/WPS 打开、编辑、保存验收，也没有完成 MCP 服务、Musterwork Artifact 提交或 E0–E3。
