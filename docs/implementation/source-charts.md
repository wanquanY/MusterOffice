# 原生图表来源、缓存与工作簿引用

2026-09-30。状态：来源查询已实现并完成局部原生/WASM 验证；图表绘制、作者模型、工作簿
一致性及可编辑往返尚未完成。**不能据此把含图表模板的导入/预览门禁改为通过。**

`mo-presentation-source::source::charts::query` 在已验证 OPC 包和 SourceIndex 上，按来源
摘要与物理页面查询原生 `graphicFrame → c:chart → chart part`。页面使用与 SourceIndex
相同的 MCE 配置；图表部件独立处理其兼容分支，不把扩展区的同名元素当作有效系列。
共享图表部件只读取一次；返回对象 ID、关系 ID、源元素序号、部件摘要与长度。

这一阶段读取传统 DrawingML 图表的系列 `idx/order`、轴 ID 和标题/分类/值/X/Y/气泡大小
数据通道。保留公式、数值与字符串缓存、多级分类的来源层序、稀疏点索引、数量声明及
格式代码。没有 `v`、空 `v`、零和错误字符串是不同值；不盲目转换浮点数、补齐稀疏数组
或假定缓存已经重算。返回的 `dataAuthority=sourceCacheSnapshot` 明示这一限制。
图表标题布局、轴语义、标签、图例和绘制样式仍由原部件持有，查询结果不是完整图表模型。

`externalData` 的关系必须属于 `relationships/package`。内部部件返回类型、摘要、长度，
外部关系仅返回 URI；均不访问网络，也不因读取了关系而宣称工作簿结构/公式/缓存已验证。
`autoUpdate` 保留缺省与显式真假，不触发自动刷新。依据官方
[ExternalData 的标准说明](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.charts.externaldata?view=openxml-3.0.1)。

默认预算：256 个图表、4096 个对象绑定、16384 个系列、100 万缓存点；最多 262144 个
物理 XML 元素、单部件 16 MiB、累计部件 64 MiB、关系查找 100 万步。瞬态 XML 树另设
16 MiB 逻辑元数据预算，包含展开后的命名空间名、属性与文字；不能用短前缀重复很长的
命名空间绕过预算。计量不依赖平台指针大小。取消、错误与超限不返回已计算的前缀结果。

公开开发入口是 Rust `inspect_pptx_charts`、WASM `inspect_pptx_charts` 与
`mo-cli pptx-charts <request.json> <source.pptx>`；请求是
`{"expectedSourceSha256":"…","surface":"/ppt/slides/slide7.xml"}`。
两份 Schema/TS 从 Rust 生成。没有新依赖、权限、存储或产品页面；尚未升级 Musterwork
正在使用的固定组件版本。

验证：9 项来源测试、2 项公共接口测试通过，涵盖关系/内容类型错误、稀疏与空值、MCE、
取消、全部预算和摘要冲突。严格 scoped Clippy 与生成类型检查通过。两份私有平台模板
各含柱状图与环形图：四个实际图表、32 个缓存点及四份嵌入部件摘要经独立 ZIP/XML 核对；
原文件不变。自有稀疏样本、错误摘要与四个实际页面共六份完整 Native/WASM 响应一致。
私有模板未迁入公共源码，自有测试也未声称其中的工作簿占位字节是有效 XLSX。

证据：[2026-09-30 图表来源验证](../reviews/evidence/2026-09-30-source-chart-verification.json)。
后续必须实现工作簿与缓存的一致数据源、图表布局/绘制、原生写入及 Office/WPS 编辑往返，
再分别关闭模板和产品链路门禁。


后续集成：该来源查询已随产品 `f419577e6` 固定 SDK 纳入源码依赖；没有把未完成的
图表绘制接入为已验收能力，也没有发布两份原生图表模板。
