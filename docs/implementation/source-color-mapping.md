# 来源颜色映射

2026-09-24 · 已接入来源索引、继承解析和文本修改后的实际候选重读。该模块把颜色角色绑定到主题槽；RGB 求值、颜色变换和绘制尚未完成。

## 声明与来源

`SourceSurface.colorMapping` 保留三种状态：未声明、`a:masterClrMapping` 标记、显式颜色表。显式表来自母版的 `p:clrMap` 或页面/版式的 `a:overrideClrMapping`，包含背景、文字、六个强调色及两种链接色的 12 项映射。目标值只能是 12 个主题色槽，不能把 `bg1`、`phClr` 等角色误当主题槽。所有必需项校验后才产生记录。

`resolvedColorMapping` 保存最终显式表的部件和物理元素序号。显式表整体覆盖父表；没有显式表时，页面取关联版式的有效表，版式再取关联母版的表。作者的省略与 master 标记仍分别保留，解析结果不会写回源文件。找不到显式表时保持 `null`，不补成任意固定默认颜色。

主题方案和颜色映射是两层不同数据：`themeSelection.colors` 指向生效的主题颜色方案，`resolvedColorMapping` 指向角色到槽的表。版式改变颜色映射不意味着修改主题部件，也不能通过把页面文字改成固定 RGB 来冒充继承。

原生结构依据见 [Microsoft ColorMap](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.presentation.colormap?view=openxml-3.0.1) 与 [MasterColorMapping](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.mastercolormapping?view=openxml-3.0.1)。slide→layout→master 的有效表传播与 [Apache POI 的 XSLFSheet 实现](https://github.com/apache/poi/blob/trunk/poi-ooxml/src/main/java/org/apache/poi/xslf/usermodel/XSLFSheet.java)交叉检查，并用下述 LibreOffice 实验验证；Office/WPS 各目标版本仍待验收。没有复制第三方实现或增加运行依赖。

## XML 和编辑边界

映射读取复用表面原有 MCE 扫描，不另读整份 XML。只有正确结构位置的声明参与解析；选中分支使用原始物理序号，未选分支及扩展区保留，不允许其嵌套同名节点覆盖有效状态。未知 choice、空/重复 choice、错误层级、缺少必需映射或非法枚举均失败。

继承沿三个已验证的领域层级处理，引用共享显式表，避免给每张页面复制一份表。来源修改后的候选索引会同时比较作者声明和有效来源，确保普通文本编辑不会改写颜色关系。这里只覆盖演示页面、版式和幻灯片母版；备注/讲义与对象属性族仍需实现。后续[颜色计算](source-colors.md)已增加显式 `phClr` 上下文、主题引用循环诊断和数值求值；完整样式继承与目标应用互操作仍待完成。

## 当前验证

颜色映射阶段新增 3 项 Rust 集成测试，当时全仓 82 项通过（当前计数见[实现进度](progress.md)），严格 Clippy 和 TS 类型检查通过。新增 16 份实际 PPTX：8 份覆盖继承、显式覆盖、省略、MCE、扩展保留、UTF-16 和缺少母版映射，8 份验证非法结构/属性被拒绝。来源 Native/WASM 共 168 组，连同既有文档/OPC/导出共 228 组。

独立 lxml/OPC 关系处理器核对 42 个成功输入的 252 个表面声明、90 张显式表和 1080 项映射属性，以及有效表来源。新语料的 47 个经 MCE 投影的表面通过官方 PML XSD；1 个故意缺少母版映射的探针明确排除，它只验证返回未解析状态。完整来源语料的 82 份实际输出、1706 个未改动压缩条目通过保留检查。

四个纯色探针经 LibreOffice 26.2.0.3 打开并生成两页 PDF。PyMuPDF 1.28.0 对指定原生形状内部各采样 25 个像素，结果全部等于预期的蓝、蓝、绿、黄：分别对应显式 master 标记继承版式、省略标记继承版式、页面覆盖和母版映射。这只验证这四个样本的外部可见颜色，不代表整个文稿的像素等价、内核渲染、Office/WPS 或完整往返已通过。

源码、构建、输入/输出、外部采样和排除项见[颜色映射验证证据](../reviews/evidence/2026-09-24-pptx-color-map-verification.json)。前一主题声明阶段的历史记录见[来源主题](source-themes.md)。

```sh
python3 tools/verification/pptx-color-map-fixtures.py .codex-work/pptx-theme-fixtures/manifest.json .codex-work/pptx-color-map-fixtures
node tools/verification/pptx-source-parity.mjs target/release/mo-cli .codex-work/wasm-node/mo_wasm.js .codex-work/pptx-color-map-fixtures/manifest.json > .codex-work/pptx-color-map-parity.json
python3 tools/verification/pptx-color-map-independent.py .codex-work/pptx-color-map-parity.json --xsd-dir .codex-work/ecma376/xsd
```

完整计数另包含既有且有摘要记录的 LibreOffice 重存样本，在 parity 命令末尾提供它。外部纯色观察器为 `tools/verification/pptx-color-map-external.py`，明确传入 `--libreoffice` 和新的 `--output-dir`；它使用独立临时应用配置、拒绝覆盖已有 PDF，依赖 PyMuPDF 的开发环境，不进入发行闭包。当前进度及后续颜色/字体/样式计算见[实现进度](progress.md)。
