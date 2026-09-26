# 演示文稿语义与编译边界整改

2026-09-27。对应架构评审 F2，实施状态逐项记录；拆分 crate 本身不等于语义合流完成。

## 已实施的依赖边界

来源声明、原生继承求值、来源绑定与时间读取由 [`mo-presentation-source`](../../crates/mo-presentation-source/src/lib.rs) 持有。该模块依赖模型、OPC/XML 和纯计算基础库，不依赖作者 PPTX 生成器、编译器、渲染器或宿主；原生字段保留式重写与其私有来源绑定放在同一适配层。原始包仍通过已验证的只读 `PackageRead` 提供，未知内容保留规则和摘要校验不变。

`mo-presentation-compile` 的生产依赖改为该来源模块；`mo-pptx` 只作为编译器测试的文件生成工具。`mo-pptx` 持有格式写出，并转导既有 `source`、`timing` 读取入口及错误类型，现有调用方无须为文件移动改变行为。这样后续 writer 消费编译计划时不会产生 compiler → writer → compiler 的包级循环。

没有增加外部组件或改变已有依赖版本。嵌入式 SDK 按生产依赖图选取库，新增内部模块由同一闭包算法纳入。历史验证脚本及摘要保留原始路径；当前门禁使用实际 workspace，当前文档源码链接更新到新位置。

## 共享作者计划与格式投影

[`AuthorPlan`](../../crates/mo-presentation-source/src/author.rs) 验证作者文档后，直接构造与来源读取相同的类型化声明，调用同一个母版/版式继承及主题选择实现。文字声明、颜色、几何、原生单位换算、对象/图片关系绑定由计划持有。计划借用不可变作者文档，保留对象语义及可编辑性；不将文档压平成绘图指令。

[`export_plan_to`](../../crates/mo-pptx/src/write.rs) 序列化这份计划。writer 原有的独立文字默认值、颜色和几何转换已经移除。现有 `export`/`export_to` 是先构造计划的便捷入口。确定性导出样例保持原始字节摘要；直接计划与实际导出回读的声明图做独立等价校验，物理 XML ordinal 与计划地址分别重绑定后比较。

图片解析通过封闭的 [`ImageInput`](../../crates/mo-presentation-source/src/source/images/input.rs) 供给关系和资源。来源适配器读取真实 `PackageRead`；作者适配器绑定计划中的资源 ID、摘要、类型和宿主授权的只读字节。二者共用图片继承、引用策略、布局和绘制准备，不伪造 OPC 包，也不读取任意路径。作者图片读入后先验摘要，再进入解码器。

`AuthorPlan::declarations()` 的地址及 `source_sha256` 是共享声明引擎内部的逻辑绑定：该摘要由带域标记的作者文档与默认值生成，**不是导出文件 SHA-256**；`byte_length=0` 不构成包证明。对外交付必须单独保留计划身份和实际 PPTX 资产摘要，不能混用。

## 直接交付与证据身份

交付构造一次 `AuthorPlan`，writer 与预览共用该计划。原生 `--preview-author-document` 接收一次作者文档、图片字节和字体；worker 构造一次类型化计划、资源关系与字体清单，逐页调用共享资源页面编译。该路径不调用 PPTX writer、`Package::open` 或 `inspect_source`。原有 `--preview-document` 来源批次与单页入口继续可用。

导出仍执行实际存储字节的封装校验和文件回读覆盖检查；这属于文件验证，不为作者预览提供语义输入。`preview-evidence/2-draft` 同时保存 `authorPlanSha256` 与 `pptxSha256`，接收端从已验证的模型和默认值重建计划身份后分别核对。旧 `preview-evidence/1-draft` 只接受原有来源文件绑定规则，不把两个身份互换。

真实 worker 回归覆盖两页文字、图片、主题/继承、隐藏页和完整交付：直接作者路径与实际 PPTX 来源路径的像素、绘图元数据一致（只重绑定预期不同的输入摘要）；两种批次的输入字节均按 ReaderAt 计数，图片/来源及字体各传输一次。覆盖跨文档身份、错误资源范围、取消、消费者拒绝以及计划/文件摘要混淆的原子失败。这是正确性回归，不是发行性能或 Office/WPS 视觉验收。

## 来源文档闭环

版本化导入、保留来源的模型、共享事务及直接来源计划已经接通，详见[统一来源文档](source-backed-document.md)。`PresentationPlan` 为 writer 与 renderer 提供各自需要的投影，PPTX 回读只作文件验证。当前交付输出升级为 `preview-evidence/3-draft`，使用通用 `planSha256` 区分逻辑输入与 `pptxSha256`；v1/v2 的已有验证规则继续保留。

来源字段和结构编辑的明确边界、原始字节保留及数据回滚兼容性见上述实现文档。静态页面依赖和缓存范围见[增量编译](incremental-layout.md)。最终验收结果见[整改记录](review-corrections.md)。
