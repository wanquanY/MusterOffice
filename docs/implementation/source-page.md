# 原生 PPTX 静态页面编译

2026-09-25 · 实现中的受限草案 `drawingml-static-solid-page-v1-draft`。这是直接消费真实 PPTX 的首条页面链路，仍未达到 Musterwork 接入或替换验收条件。

## 已连接的链路

`PPTX bytes → OPC/XML/MCE → 不可变来源索引 → 母版/版式/页合成 → 原生几何、位置、填充和线条求值 → 共享路径场景 → Native/WASM CPU RGBA`。原生来源不转换成会丢失信息的作者文稿。该入口不修改文件，不读取系统字体、路径或网络。

页请求携带源包摘要、演示文稿中的 slide 部件、显式 profile、颜色上下文和 viewport。viewport 必须从原点以均匀比例精确覆盖整页。来源检查与摘要核对由内核完成，不接受调用方伪造的来源索引。隐藏幻灯片仍可被显式请求预览，响应保留其隐藏标记；播放中的跳页规则不属于这个静态入口。

当前合成按母版、版式、页面的顺序绘制普通对象，结合 `showMasterSp` 和对象/父组合的 `cNvPr@hidden`。模板中的普通占位对象提供继承，避免重复绘制提示形状；日期、页码、页脚等特殊占位语义尚未接通，会产生明确诊断。显示层、候选对象、隐藏对象和模板占位符分开记录；候选对象可能没有实际墨迹，不等于已绘制对象数。

普通母版/版式图形使用其物理 drawing surface 的主题与颜色映射。页面占位符即使继承了母版声明，也使用页面的 drawing context。`useBgFill` 单独跳转到最终页面背景的上下文；它不能把同批普通图形或线条的颜色上下文一起改掉。原始声明 owner 保持不变，跨上下文背景结果通过可选 `contextOverride` 显式记录。颜色预算在同批上下文切换时继续累计。

这些 draft 规则依据原生属性职责实现，并用自有文件观察，不能据此宣称已证明 Office/WPS 的全部继承行为。[Microsoft 的 slide 说明](https://learn.microsoft.com/en-us/office/open-xml/presentation/working-with-presentation-slides)解释颜色覆盖所属的父元素；[Shape 的 `useBgFill` 定义](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.presentation.shape?view=openxml-3.0.1)要求使用形状后方背景区域。

## 绘制与失败边界

本次接通已有 187 种预设展开和自定义几何计算能力；并不意味着每种预设在当前受限画笔下都可完整绘制。支持纯色/无填充及中心、单线、实线、无箭头、圆角或斜角连接的纯色描边。路径填充模式支持普通/无填充；需要 lighten/darken 等修饰的可见填充返回待实现诊断。背景支持当前继承链中的纯色/无填充，viewport 的显式背景色是最终清屏色。

当前以下可见内容会阻止整页成功：文字体（包括尚未接入的空文字体）、图片、graphicFrame、高级对象、非空效果、效果引用、空间渐变/图案/图片画笔、虚线/箭头/复合线、原生 miter 策略及未完成的几何/占位继承。底层已有渐变或尖角原语，不代表其原生属性映射已接入页面。3D、未知视觉节点/属性等保留物理序号并返回诊断。没有先绘制一部分再把错误包装成成功。

隐藏对象及隐藏组合的子对象不贡献墨迹；不可见模板层仍可能提供颜色、背景或继承，不能直接免除相关语义检查。根组合的效果单独检查。结构之外的非空白正文被拒绝；未实现可见节点保留为覆盖缺口。此目录是受限静态页面的保守检查，不是完整 OOXML 视觉语义覆盖证明。

本 profile 为彩色静态输出，未提供黑白/灰度输出模式；原生 `bwMode` 的完整求值尚未实现。动画、转场、动作和播放时间图仍保留在来源文件中，但不由这个静态入口执行或验收。后续完整功能目标没有缩减。

任何编译、后端、预算或取消错误都不发布部分像素。后端调用后的取消遵循既有实例失效规则，防止复用不确定状态。当前编译仍会为每个请求重新检查/解析源包；生产缓存、增量编译、资源句柄及进程池尚未完成。

## 坐标、来源与资源预算

原生几何坐标在 shape extent 内求值，编译后的路径先减去精确 anchor，再应用已经解析的世界矩阵一次。组合缩放、翻转和旋转复用[来源坐标](source-placement.md)，没有重复应用对象位置。

页面先根据所有对象矩阵的最大绝对行和及其不确定度计算保守局部路径容差，为设备坐标预算分配至多四分之一。整个页面共享一个原生路径编译器及其命令、弧片段和工作预算。路径、仿射变换可复用；作者页面与来源页面共用场景构造、坐标误差计算，不维护两套同义实现。

`downstreamCoordinateErrorBound` 包含位置矩阵、路径近似/Q32 和设备转换的下游坐标误差。它以已求出的 binary64 导引值为起点，**不包括上游公式求值误差，也不是抗锯齿、颜色、视觉相似度或 Office/WPS 保真保证**。

每个实际绘制实例绑定物理部件/对象、drawing surface、填充/线条结果、位置及文档/预设路径来源。直接映射缺口可定位对象及节点；底层通用数值错误仍使用相应错误合同，不保证每种错误都有同等细粒度的对象定位。

可见层中被遍历的对象（含隐藏对象和组合）累计上限为 8192。每层的位置、几何、填充和线条查询继续各自受既有预算约束；最多三层加一份背景查询，不能把这些分层预算描述为一个全页共享查询额度。场景另限制生成命令 1048576、唯一命令 262144、唯一路径 4096、变换 8192；设备执行还有既有像素/工作/内存上限。生产请求预算统一和细粒度取消延迟仍需后续验收。

## 开发接口与宿主边界

- Rust：`mo_presentation_compile::source_page`，以及 `mo_kernel_api::compile_pptx_page` / `render_pptx_page` 的真实范围读取入口。
- WASM：`compile_pptx_page(requestJson, sourceBytes)`；`render_pptx_page(requestJson, sourceBytes, component)` 返回 metadata 和独立像素字节。
- CLI：`compile-pptx-page <request.json> <source.pptx>`；`render-pptx-page <request.json> <source.pptx> <new-output.rgba>`。
- 生成合同：`pptx-page-request`、`pptx-page-compile-response`、`pptx-page-raster-response` 三份 Schema 和对应 TS。来源响应新增可见性/文字体/视觉缺口元数据；填充颜色结果新增可选上下文记录。

Native 绘制复用隔离 `mo-raster-worker --pptx-page`。输入为两个 little-endian u32（JSON 长度、源文件长度），后接 UTF-8 JSON 与源字节；32 MiB JSON 和 128 MiB 源文件上限在分配前检查。其他 worker 模式保留原帧格式。输出继续使用 metadata 长度、像素长度和独立字节；CLI 核对 profile、长度、摘要，并在暂存重读后发布新名字，拒绝覆盖现有文件。逻辑错误可以返回成功进程状态加 `status:error`，宿主必须检查响应状态。

这些是开发接入点，不是已发行 SDK 或完整 Agent 服务。MCP/Skill/Plugin 将按[既定集成设计](../architecture/agent-integration.md)调用统一操作服务；Musterwork 的资源授权、Artifact 提交、取消、任务恢复、历史迁移及 E0–E3 仍需完成。

## 验证证据

本阶段新增 13 项 Rust 测试，累计 384 项；新增 100 批主内核 Native/WASM 检查，重跑旧 9401 批后累计 9501 批。新增用例含 28 份自有 PPTX、36 次编译、36 次绘制请求；21 份成功页面的实际 RGBA 像素逐字节一致，其余保留明确诊断。另验证两种 worker 帧拒绝、实际 CLI 新文件发布/拒绝覆盖/三种失败不落盘。

独立 XML 检查覆盖 168 个索引表面、238 个对象、85 个文字体和可见性/视觉缺口的物理绑定。84 个修改后的 PML 部件中 82 个通过官方 XSD；另两个是故意不完整的 `scene3d` 负例，不算合规成功。没有把整包基础语料的来源读取当作所有 XML 都通过 XSD。

独立整数像素参考检查 10 个不透明轴对齐矩形页面、3461792 个内部像素，排除距离分析边缘两像素以内的区域，零差异。它不覆盖曲线抗锯齿或一般颜色运算。LibreOffice 26.2.0.3 的对应 PDF 有 9 个观察在内部像素允许 1 码值时匹配；母版 `useBgFill` 探针保留直接红色填充，而内核按当前规则使用最终页面背景。该差异保持未关闭，不能把 9/10 解释为应用兼容率。

本阶段没有执行 Microsoft Office 或 WPS 的页面编辑、保存、重开验收。WPS GUI 验证程序缺少 macOS 辅助功能及录屏权限，已停止，未通过替代截图推断成功。此前来源变换在 WPS 中的零子尺寸与部分占位符差异继续保留，见[来源坐标证据](source-placement.md)。

旧 9401 批只允许来源响应新增元数据；既有数值、错误、编辑候选、帧及像素按原摘要/内容复核。作者页面独立参考再次核对 52 份文稿、8807 个对象、52842 项矩阵系数和 44042 个控制点。当前运行合同共 66 份，严格 Clippy、格式、生成一致性和 TS 检查独立执行。

记录由 [source-page-evidence.py](../../tools/verification/source-page-evidence.py)生成，密封证据见[本阶段记录](../reviews/evidence/2026-09-25-source-page-verification.json)。它引用上阶段证据而不重写历史记录。

## 开发体积

同机 macOS arm64 release 产物的未压缩字节数：CLI 5962448，Rust WASM 5160353，Native 绘制 worker 5499920。相较上阶段分别增加 395552、111051、1796288 字节；worker 现在实际包含 PPTX 读取与原生页面编译，不能把这项功能增长隐藏在体积报告之外。文字 worker 为 2375360 字节，本次仅变化 32 字节。C++ WASM 组件和 TS 适配层摘要不变。

未引入新的外部运行依赖或版本；Cargo 锁文件只增加 worker 对已有 `mo-raster` workspace 包的显式依赖，pnpm 锁文件未变。本阶段没有新的产品延迟/RSS 测量；这些均为**不完整开发产物**，不可直接相加作为压缩安装包大小，更不能据此预估 Musterwork 最终打包收益。

## 重现

先完成[构建说明](development.md)及[来源坐标](source-placement.md)语料准备。Node 和独立 Python/PDF/XML 工具仅用于验证，不是内核运行依赖。

```sh
python3 tools/verification/source-page-fixtures.py
node tools/verification/source-page-parity.mjs
python3 tools/verification/source-page-source-check.py
python3 tools/verification/source-page-cli.py
python3 tools/verification/source-page-reference.py --observe
python3 tools/verification/source-page-regressions.py
python3 tools/verification/source-page-regression-audit.py
python3 tools/verification/page-placement-reference.py
python3 tools/verification/contracts.py --source-page-report .codex-work/source-page/parity.json
```

`source-page-reference.py --observe` 需要独立环境中的 NumPy、Pillow、PyMuPDF 和本机 LibreOffice，用独立临时配置转换拥有的探针；不带参数则重读已有 PDF。回归审计另需在覆盖旧输出之前保存、按上阶段摘要验证的 `previous/` 报告及 `previous-json.zip`；缺少旧字节时必须失败，不能用新产物自证历史不变。密封脚本只创建新证据文件，重现时默认核对，不覆盖已密封历史。

下一步把来源文本样式/字体资源与现有段落计算连接到这个页面编译器，再扩展图片、空间画笔、效果和高级内容；随后分别完成可编辑导出、播放、应用互操作与宿主替换验收。
