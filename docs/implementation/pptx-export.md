# 原生 PPTX 导出实现边界

2026-09-24 · 当前实现，不代替 v0.4 完整一期设计或兼容验收。

## 路径与职责

`mo-pptx` 将已验证的演示文稿记录映射为原生 PresentationML/DrawingML，使用 `mo-opc` 生成 ZIP/OPC 后重新读取实际字节。文本、形状、图片、组合和连接线分别写为原生对象；对象名保留 MusterOffice ID 以便验证。输出不是每页截图，也不调用 LibreOffice、Python 或浏览器生成文件。

核心只接收 `Document`、显式 `ExportDefaults`、按资源 ID 提供 `ReaderAt` 的 `Resources`、预算和取消回调。`mo-kernel-api` 提供开发 JSON/二进制桥接，Native CLI 和 WASM 调用同一实现。CLI 拥有文件 I/O：排他创建暂存文件、写入与同步、重新检查实际文件、核对计算结果，再以不覆盖目标的方式发布；不支持硬链接的文件系统明确失败。生产 ResultSink、授权句柄、任务持久化、CAS 提交和崩溃恢复仍待宿主层实现。

## 已映射范围

| 内容 | 当前实现 |
| --- | --- |
| 文稿 | 页顺序、尺寸、隐藏页、标题；主题/母版/版式及关系 |
| 文本 | Unicode 段落和 run、换行/Tab、明确字号/颜色/字体家族、粗斜体/下划线、语言、对齐/RTL/两种竖排方向、段前后距、文本框内边距/换行和 overflow 属性 |
| 几何 | 矩形/圆角矩形/椭圆、自定义路径（move/line/quadratic/cubic/close）、旋转/翻转、组坐标和嵌套对象 |
| 外观 | 无填充/纯色/主题色、8 位 RGBA 透明度、描边线宽与显式端点/连接 |
| 图片 | PNG/JPEG 原生图片部件、摘要校验、裁切/变换；当前只识别文件头，不是完整解码验证 |
| 连接线 | 自由端点及已支持形状的连接位点引用；保留原生对象引用 |
| 校验 | 文稿语义前置校验、数值可表示性、XML 字符/部件预算、OPC 重读、实际文件摘要、原生/WASM 逐字节一致 |

默认字体家族、文字大小/颜色、背景、12 个主题色槽由调用者显式提供，核心不读取系统字体。当前 `FontDelivery::ReferenceOnly` 仅输出可编辑字体引用，未做字体选择、字形塑形、嵌入、布局或缺字检测。FontFace 的字重/变体解析仍须在字体引擎中实现，当前外观依赖显式 CharacterStyle 粗斜体和目标应用字体。

## 精确性和未实现边界

静态旋转写为一圈内的非负等价角度，作者值和来源保留编辑仍保留原始负号/圈数；本规则不用于动画。该选择解决了组变换样本中的 LibreOffice 角度差异，已核对 WPS 画布像素不变，见[静态旋转导出](static-rotation-export.md)。

导出对原生数值域进行检查。例如字号/段距需要精确表示为百分之一磅，裁切需要精确转换为 OOXML 单位；无法表示时返回错误。主题色的 8 位 alpha 映射到 OOXML 百分比后，可还原原 8 位 alpha，但不表示任意高精度色彩模型已实现。

带 `SourcePackage` 的导入文稿明确拒绝走新建导出器，必须等待来源保留计划；否则会丢失模型尚不认识的原生内容。尚未映射的装饰性辅助属性、版式默认文本样式、部分主题默认样式、组描边、混合段落书写模式等返回 `Unsupported`。目前完整预设几何、效果、占位符/样式继承、表格/图表、SmartArt、公式、动画/转场、媒体等仍未实现，不降低一期范围。

当前 writer 直接消费领域记录，还没有接入最终设计的 `CompiledPlan`、字体/布局及完整语义 Reader 验证。新增[来源 Reader](pptx-source.md)只解析部分原生对象/变换/文本。因此 `OverflowPolicy::Report` 只写无自动适配的原生文本属性，尚不能报告真实布局溢出；`GrowShape` 交由目标应用处理。XML 长文本/路径内部的取消粒度、定义数量的提前预算、资源去重、流式导出和完整稳定错误合同继续在对应层完善。导出成功不构成视觉质量、可编辑往返或性能保证。

## 验证与当前结果

自有合成语料含两页、15 个页/母版/版式原生对象及一个程序生成的棋盘格 PNG。17 组 Native/WASM 用例包括 7 组成功文件与 10 组预期拒绝；成功文件逐字节相同，失败不发布目标文件，已有文件不会被覆盖。独立 `python-pptx`/`lxml` 从真实 ZIP 中读取对象，并使用官方 ECMA-376 Part 4 第五版 Transitional XSD 验证 70 个 PML/DML 部件实例。

LibreOffice 26.2.0.3 能打开初始 PPTX、生成两页 PDF 并重存 PPTX；原 15 个对象的类型、组内层级、文本、图片和连接引用仍存在。重存后有 60 处坐标/旋转表示差异，包含组变换重写。后续[来源 Reader](pptx-source.md)还确认新增两个占位对象，并有一个版式对象移入母版；对象存在不代表全部所属关系保持。必须完成世界坐标、外观和编辑语义对照，才能区分表示变化与内容漂移；当前不判定往返通过。PDF 来自 LibreOffice，不是 MusterOffice 渲染能力。

初次导出验证时，WPS 守护进程权限不足，未执行打开/编辑验证。随后通过调用者已有权限取得 WPS 12.1.22553 的[组合专项实际画面](group-placement-compatibility.md)，基础两页文稿的 WPS 编辑往返仍未验收；没有 Microsoft Office 互操作结果。当前证据不能启动 Musterwork 产品切换。

复现命令见[开发说明](development.md)，绑定源码/构建/输入/输出的证据见 [PPTX 验证记录](../reviews/evidence/2026-09-24-pptx-export-verification.json)和 [LibreOffice 差异记录](../reviews/evidence/2026-09-24-libreoffice-roundtrip-differences.json)。官方校验输入来自 [ECMA-376 标准页](https://ecma-international.org/publications-and-standards/standards/ecma-376/)，下载器固定 URL 及归档 SHA-256，缓存不进入运行依赖。

显式端点、连接与尖角参数随后接入原生写出，49 份实际可编辑文件通过独立检查；WPS/LibreOffice 的尖角差异仍未关闭，见[作者描边](stroke-author.md)。
