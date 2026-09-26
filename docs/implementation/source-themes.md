# 来源主题声明与分项覆盖

2026-09-24 · 已接入真实 PPTX 来源索引和文本修改后的候选重读。此阶段读取作者声明并确定其来源；后续[颜色计算](source-colors.md)已实现按需求值，字体选择、效果计算和渲染仍未完成。

## 记录和覆盖

`SourceIndex.themes` 按部件建立共享目录，保存不可变来源摘要、主题/覆盖类型、MCE 处理摘要及三个独立方案。每个声明绑定源 XML 的物理元素序号；兼容选支不会改写这个序号或源文件。母版、版式、页面继续保留各自的关系和 `effectiveTheme` 栈。

`SourceSurface.themeSelection` 分别指出颜色、字体和格式方案的来源部件与元素序号。依次应用基主题、版式覆盖和页面覆盖；覆盖部件未声明的方案继续使用前一层。例如版式只覆盖颜色、页面只覆盖字体时，颜色来自版式，字体来自页面，格式仍来自基主题。空的覆盖部件也不会清空先前方案。三个方案独立覆盖的依据见 [Microsoft ThemeOverride 定义](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.themeoverride?view=openxml-3.0.1)。

| 声明 | 已读取内容 | 后续计算 |
| --- | --- | --- |
| 颜色方案 | 12 个色槽；sRGB、scRGB、HSL、系统色、方案色、预设色六种表示；按顺序保留 28 种变换及重复变换 | 已接入[按需颜色计算](source-colors.md)；目标软件兼容 profile 仍待验收 |
| 主/次字体方案 | Latin、East Asian、Complex Script，Panose、pitchFamily、charset；补充脚本字体保持原顺序与重复声明 | 主题字体标记、语言/脚本匹配、明确资源绑定、替代策略、字形塑形 |
| 格式方案 | 填充、线条、效果、背景填充四个列表中的原生类型和物理来源序号；线条属性见[来源线条](source-lines.md) | 其他原生属性读取、样式索引、placeholder color、填充/线条/效果求值 |

百分比采用精确原生词法记录：整数形式为千分之一百分比，带 `%` 的小数形式保留全部位数，不在读取时转换成浮点数。整数范围和各变换的正值/固定区间分别校验；已声明的 `+0001`、`50.123456789%` 不被误写成另一种作者表示。系统颜色的 `lastColor` 仅记录文件中的缓存值，不访问本机主题。空的 EA/CS 字体名保持为空，不暗中使用系统字体。

`objectDefaults`、额外颜色方案、其他根声明及扩展内容仍留在源部件中。未解析的根声明返回提示，且其整个子树不进入当前方案状态；其中嵌套的熟悉节点名不能注入有效颜色方案。格式中的线条声明后续已接入共享解析器，见[来源线条](source-lines.md)；其他条目仍绑定源内容，不把保留等同于已经实现效果。

## 有界读取与编辑闭环

每个唯一主题部件只解析一次。默认主题部件总数不超过 4096，所有主题合计不超过 100 万物理元素、32 MiB 属性值；原有 XML 深度、字节和 OPC 限额同时生效。计数包括未选择分支和扩展区，避免通过兼容包装绕过预算。读取沿用取消回调，不创建宿主字体或网络依赖。

来源文本修改仍只改目标叶节点。候选包重新读取后必须保有相同的主题声明、分项选择与来源绑定；其他部件复用原始压缩内容。非法颜色、缺失/重复声明和超限输入明确失败。这里不是完整 XSD 验证器：未建模声明可保留并报告，成功读取不承诺整份文件符合标准或已能完整导入。

## 验证与复现

主题声明阶段新增 4 项 Rust 集成测试，当时全仓 79 项通过；当前计数见[实现进度](progress.md)。独立生产器增加 20 份真实 PPTX，包括六种颜色模型、全部变换、高精度百分比、重复脚本字体、UTF-16、分项覆盖和 15 份非法声明。该阶段来源语料有 56 份合成文件，另有 1 份此前由 LibreOffice 重存的自有文稿；Native/WASM 来源检查与修改共 136 组，连同其他文件/文档计算共 196 组。

独立 ZIP/lxml 处理器核对 34 个成功输入的 74 个主题部件实例、204 个表面的分项选择、852 个颜色声明及物理元素序号。190 个预设色、30 个系统色、17 个方案色枚举与官方 ECMA XSD 对照。73 个经过 MCE 投影的主题部件实例通过 XSD；另 1 个故意包含未知根声明的保留探针明确排除，不能计入标准合规。完整来源语料还有独立对象/文本检查，以及 66 份实际输出、1378 个未修改压缩条目的保留检查。

构建入口见[开发说明](development.md)。在已有来源与继承语料后执行：

```sh
python3 tools/verification/pptx-theme-fixtures.py .codex-work/pptx-inheritance-fixtures/manifest.json .codex-work/pptx-theme-fixtures
node tools/verification/pptx-source-parity.mjs target/release/mo-cli .codex-work/wasm-node/mo_wasm.js .codex-work/pptx-theme-fixtures/manifest.json > .codex-work/pptx-theme-parity.json
python3 tools/verification/pptx-theme-independent.py .codex-work/pptx-theme-parity.json --xsd-dir .codex-work/ecma376/xsd
python3 tools/verification/mce_reference.py .codex-work/pptx-theme-parity.json
python3 tools/verification/pptx-source-independent.py .codex-work/pptx-theme-parity.json
```

如需复现本轮完整计数，应在 parity 命令末尾加上同一份有摘要记录的 LibreOffice 重存样本；不传该文件时少 3 组检查。源码、产物摘要、输入/输出和排除项见[主题验证证据](../reviews/evidence/2026-09-24-pptx-theme-verification.json)。没有新增运行时依赖。

角色到主题槽的来源映射已继续实现，见[颜色映射](source-color-mapping.md)。下一步将这些声明接入颜色/字体和各属性族计算，再映射到完整 Document/SourceBinding、字体文本引擎与布局。尚无完整来源页面渲染、播放或 Office/WPS 等价证据，不能据此启动 Musterwork 正式替换。
