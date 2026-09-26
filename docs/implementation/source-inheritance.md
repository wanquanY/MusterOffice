# 来源占位符与继承关系

2026-09-24 · 已接入真实 PPTX 来源读取和文本修改后的候选校验。当前解析位置/尺寸与主题部件关系，主题声明与分项覆盖进一步见[来源主题](source-themes.md)，颜色的按需求值进一步见[颜色计算](source-colors.md)，字体求值、填充、文本样式与完整文稿导入仍待完成。

## 来源记录与派生结果

`SourceObject.placeholder` 记录 `p:ph` 的类型、索引、方向、大小和自定义提示标记。每个属性保留是否显式出现，不能把省略改写成显式默认。匹配时才应用 `type=obj`、`idx=0` 等标准默认；枚举和无符号索引在读取时验证。对象原有 `transform` 继续保存显式局部变换。

`SourceSurface.links` 从 OPC 关系取得 slide→layout、layout→master、master→theme、layout/slide→themeOverride。父关系和主题关系要求内部整部件目标、正确内容类型和单一指向；母版指向多个版式是正常反向关系，不作为继承父边。`effectiveTheme` 记录主题基部件与按母版到页面排序的覆盖部件摘要，这里只负责部件关系；主题内容由共享目录读取，`themeSelection` 按颜色/字体/格式分别绑定来源，仍不表示颜色、字体或效果已完成求值。

`SourceObject.resolution` 是独立派生记录。`origin`、`size` 各自包含有效值和最终显式声明的部件/原生对象 ID。一个对象的位置可以来自母版、尺寸来自版式，局部显式零值仍覆盖继承值。解析不会补写 `transform`、改写源 XML，或把多个层级合并为作者值。

## 匹配规则和边界

幻灯片占位符按 `idx` 匹配关联版式；这个行为由 [Microsoft 的 OOXML 实现说明](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/3ec954b2-37a6-41da-8973-04a592c91fb2)明确描述。版式到母版按占位符类型族匹配：居中标题归到标题，内容类归到正文，日期/页码/页脚分别匹配。类型族表与 [python-pptx 的占位符实现和实验记录](https://python-pptx.readthedocs.io/en/latest/dev/analysis/placeholders/layout-placeholders.html)交叉核对；目标 Office/WPS 版本仍须实测，不能以第三方库一致代替验收。

| 匹配结果 | 当前含义 |
| --- | --- |
| `notPlaceholder` / `master` | 普通对象或继承链顶层，只解析自身显式几何 |
| `matched` | 找到唯一父占位符，返回父引用和匹配规则；属性逐项继续继承 |
| `unmatched` | 缺少关系或匹配对象；缺失的几何保持未解析，不补成零 |
| `detached` | 页面上的 `idx=0xffffffff` 不参与版式匹配 |
| `ambiguous` | 多个候选，返回所在部件及候选数，不任意采用第一个 |
| `unsupportedContext` | 组合内占位符、组合自身占位符、非页面语义的 header/slideImage 或不合法的母版/版式脱离索引；不从该上下文向子级传播几何 |

位置与尺寸以成对的 OOXML `off` / `ext` 声明为单位解析。这里的坐标仍是对象所属局部坐标，不是组合变换后的世界坐标。旋转、翻转、child transform、占位符文字/提示、样式和背景的继承尚未纳入该解析结果，不能拿它直接宣称布局完成。

实现先建立部件内索引与类型索引，再按母版、版式、页面三个固定层级解析。实际 OPC 中母版/版式的双向链接不会造成递归；每个对象检查取消，原有包/XML/对象限额继续生效。共享主题及父引用均绑定本次不可变来源索引，而不是文件名或显示名称。

## 验证与复现

5 项新增 Rust 集成测试覆盖真实包读取、逐属性来源、显式零值、缺失/脱离/歧义、非法声明、未支持父上下文传播和修改后不压平。该阶段全仓 75 项 Rust 测试通过；后续主题阶段的当前计数见[实现进度](progress.md)。

```sh
python3 tools/verification/pptx-source-fixtures.py .codex-work/pptx-output/native.pptx .codex-work/pptx-source-fixtures
python3 tools/verification/pptx-inheritance-fixtures.py .codex-work/pptx-source-fixtures/manifest.json .codex-work/pptx-inheritance-fixtures
node tools/verification/pptx-source-parity.mjs target/release/mo-cli .codex-work/wasm-node/mo_wasm.js .codex-work/pptx-inheritance-fixtures/manifest.json > .codex-work/pptx-inheritance-parity.json
python3 tools/verification/pptx-inheritance-independent.py .codex-work/pptx-inheritance-parity.json
python3 tools/verification/mce_reference.py .codex-work/pptx-inheritance-parity.json
python3 tools/verification/pptx-source-independent.py .codex-work/pptx-inheritance-parity.json
```

加上既有 LibreOffice 重存样本后，来源 Native/WASM 共 106 组检查。12 份继承专项输入通过独立关系/主题栈检查，346 项几何属性与 python-pptx 对比；歧义与不支持上下文由显式用例断言，报告列出第三方库不能用作判据的排除对象。MCE 投影只用于独立库的内存验证副本，不进入实际编辑输出。

全部来源语料中，29 份成功输入经独立 MCE/对象/文本投影核对，56 份实际编辑/复制结果经字节保留校验，检查 1165 个未修改压缩条目。可见尺寸的继承文稿修改后已由 LibreOffice 26.2.0.3 导出两页 PDF并提取到新中文；这不证明像素、世界坐标、内核渲染或 Office/WPS 编辑等价。

该阶段源码、构建、输入、输出和排除项见[继承验证证据](../reviews/evidence/2026-09-24-pptx-inheritance-verification.json)。主题声明已继续实现，后续推进属性族继承、完整来源绑定和领域模型映射，不能把本轮来源几何投影当作完整导入。
