# 原生文字装饰绘制

后续已实现[独立下划线纯色 / noFill](underline-paint.md)。以下保留本阶段范围和冻结证据；原文中的独立填充未实现项已由该后续阶段推进。

2026-09-25，开发态实现。沿用 Rust 核心、HarfBuzz/Skia 原生组件和同一 Rust WASM 主模块；不引入 HTML/CSS 排版或额外运行依赖。承接[来源文字页面接口](text-page-runtime.md)和[资源诊断](text-resource-diagnostics.md)，完整一期范围保持不变。

## 本次实际能力

`a:rPr u="sng"` 和 `strike="sngStrike"` 已进入原生文字页面绘制，可同时使用。单下划线接受跟随文字的默认画笔及 `uLnTx` / `uFillTx`，使用当前纯色或 noFill。颜色继承、来源摘要、显式字体选择、整页预检、Native worker / CLI 原子发布及 WASM 边界沿用同一实现。

线段覆盖排版后的 advance，包括普通空格，不按可见字形轮廓包围盒生成；组合字符不会单独扩大线宽。相邻且位置、颜色、粗细一致的线段合并，未装饰文字和换行分隔线段。实际字簇跨越不一致的装饰声明时，仍整页返回 `glyphPaintConflict`，不擅自切开连字或组合字簇。几何位于对象本地坐标，使用与字形相同的锚点重定位、对象变换、坐标误差检查及 SceneBuilder。

本轮实现了单线的字体度量规则；双线、粗线、点划线、波浪线、words-only、独立 underline fill/stroke、描边、高亮及文本效果仍有显式未实现诊断。原生上标/下标 baseline、竖排、列布局、自动适配等也未因本轮绘制变化而获得支持。上述项目继续属于完整一期目标。

## 度量与资源责任

`PreparedManifest::measure_instances` 从已校验的字体绑定读取明确要求的度量，复用已验证字节和 face。字体/轴/批次数量先检查；装饰所需度量不会加入普通文字的必需行度量。编译器按实际选中的字体和请求轴去重，每字体最多 256 实例一批，每框最多 2048 个装饰实例。普通页面不增加装饰度量组件调用，含 regular/bold 两实例的已测样本只增加一个批次调用。这个调用计数不是耗时或内存性能结论。

位置和厚度取 HarfBuzz 的 `undo` / `unds` / `stro` / `strs`，包含实例轴和 MVAR。按 OpenType [post](https://learn.microsoft.com/en-us/typography/opentype/spec/post) 与 [OS/2](https://learn.microsoft.com/en-us/typography/opentype/spec/os2#ystrikeoutposition) 定义，两种 position 都指线条顶边的 y-up 坐标；转换到页面 y-down 后厚度向下展开，不误作中心线。OOXML 的样式与跟随画笔语义依据固定 ECMA-376 Part 1 §21.1.2.3.12–15；这里的字体度量绘制策略仍是本项目 draft，不能据此认定 Office/WPS 使用相同像素规则。

缺失度量 `None` 与真实零不同。不可用的位置、零或负厚度返回 `decorationMetric`，附对象、段落、run、物理节点、字体绑定、具体度量及值；不凭空替代为字号比例或最细一像素。无装饰/无填充文字不要求这些度量。组件调用、字体上传、请求字数和生成路径均计入已有整页预算，取消或额度失败没有部分图片输出。

字形片段起点只重建一次；advance 前缀用整数设计单位累计，再转换 Q32。相邻片段最多 2 raw Q32 的舍入缝可以并合，坐标证据在原框误差界之外保守增加 8 raw Q32，覆盖起点重建、prefix/度量缩放和合并。这个误差界相对选定字形整数输出及断行拓扑，不包括字体引擎、布局选择、颜色、抗锯齿或应用差异。

## 验证与复现

新增原创 `owned-decorations.ttf` 复用仓库原创轮廓与 MVAR/avar，补充有效装饰度量。两个 Hebrew 码点映射到原创测试轮廓，仅用于双向放置测试，不是希伯来字体设计或真实字体质量样本。原字体不被改写，许可沿用仓库原创语料状态。

主要命令（FontTools 4.61.1、NumPy、Pillow、lxml、jsonschema 使用开发环境已有依赖）：

```sh
python3 tools/verification/decoration-font-fixtures.py
MO_DECORATION_EVIDENCE_DIR="$PWD/.codex-work/text-decorations/cases" cargo test -p mo-harfbuzz-sys --test text_decorations --locked
python3 tools/verification/text-decorations-failures.py
python3 tools/verification/text-decorations-reference.py
node tools/verification/text-decorations-parity.mjs
node tools/verification/text-page-runtime-regressions.mjs .codex-work/text-decorations
```

跨端探针使用本阶段单独生成的 `.codex-work/text-decorations/wasm-node`；构建流程沿用[开发说明](development.md)，不要覆盖旧阶段证据。独立参考从 XML 读取样式和变换，以 FontTools 计算基础度量及 MVAR，以 Fraction 重算已选字形的坐标，再构造参考图。字形整数及断行选择是参考输入，不宣称独立验证全部塑形。像素比较排除距边缘 2 像素内区域，不认证抗锯齿。

当前冻结结果见[文字装饰证据](../reviews/evidence/2026-09-25-text-decorations-verification.json)：474 项 Rust 测试，14 个成功页面、2 个缺失度量失败请求的 Native/WASM 结果一致，16 个 CLI 请求检查；39 个前期文字页面请求和 307 个更早请求重放。前期两个单下划线请求从“属性未实现”推进为“字体厚度为零”，其它旧响应及成功像素保持一致。独立验证覆盖 14 个包、48 个行度量值、23 个装饰矩形、1,589,995 个内部像素，排除 90,005 个边缘像素。

本阶段没有产品耗时、RSS、安装包体积或 Office/WPS 打开—编辑—保存验证。Musterwork 生产宿主、Artifact、完整高级内容及替换验收仍未完成。
