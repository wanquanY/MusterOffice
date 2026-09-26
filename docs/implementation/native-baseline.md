# 原生文字基线偏移

2026-09-25，开发态实现。在[文字页面运行接口](text-page-runtime.md)和[独立下划线画笔](underline-paint.md)之上，原生 `baseline` 百分比已接入源样式继承、共享文字布局、字形路径及实际页面绘制。

## 来源与布局

按原生字号计算偏移：整数字面量以千分之一百分比为单位，带 `%` 的十进制按百分比解释；正值向上，负值向下。字符、默认字符和段落结束样式保留各自来源，结束样式参与空行和行高，不覆盖已有 run。已有字形、自然行高、精确/最小行距、换行、字体实例、下划线/删除线及页面旋转继续共用一条计算路径。

语义来自固定 ECMA-376 Part 1 §21.1.2.3.9 的 `baseline` 定义及 [Microsoft 对正负值和默认值的说明](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/953e072c-f234-4373-b017-3f1f71eac93b)。本阶段按明确的 `sz` 绘制，不附加猜测的自动缩小比例；这些基线能力不等于 Office/WPS 上下标界面、字体缩小、行距和视觉规则已全部验收。

颜色和字体不因基线偏移改变，原文字串也不被替换为 Unicode 上下标字符。几何样式按照计算值比较：`1000` 和 `1%` 可合并相同计算样式并保留两个原始 run；同一扩展字簇内不同的有效基线仍明确失败，不能将组合字符拆开绘制。

## 精度与合同

百分比以有限长度的任意精度整数分子/分母解析，与字号相乘后一次转为 Q32 EMU。转换按最近值、半值远离零；不先舍入到整数 EMU，也不经 JavaScript Number 或二进制浮点。原生数值最多 256 个字节，超长或超范围在组件调用前返回明确错误，取消不发布部分页面。

共享 `GeometryStyle.baselineShift` 增加精确形式：

```json
{"fontSize":"381000","baselineShift":{"q32":"2147483648"}}
```

上述表示向上偏移 0.5 EMU。原有 `"baselineShift":"1"` 仍表示 1 EMU，输入、计算语义及序列化保持兼容。精确对象只接受 `q32` 字段，值为规范 i128 整数字符串；和既有 Q32 合同一样，范围必须由 Rust 语义校验。整数/精确两种表示按同一数值比较，等值表示不切断塑形。3 份请求 Schema 和对应 TS 类型由 Rust 同源生成，其余运行合同不变。

Rust 库字段类型由 `Emu` 扩展为 `BaselineShift`，构造整数偏移时使用 `BaselineShift::Emu(value)` 或 `value.into()`；上述兼容性指 JSON 运行合同，Rust 调用方需更新构造代码。

原生分数转换的误差不因有效样式合并而消失。`SourceParagraphPlan.baselineConversionError` 保留这一转换是否有分数误差（Q32 raw 单位向上取整为 1）；即使极小值被舍入为零、与普通文字合并，页面仍加入保守的 `2L + 2` raw Q32 误差，其中 L 为选定行数。该误差相对已选字形整数和断行拓扑，不覆盖字体引擎、换行决策或应用差异。

## 已验证范围

[冻结证据](../reviews/evidence/2026-09-25-native-baseline-verification.json)绑定源码、构建、样本和结果：

- 487 项 Rust 测试通过，新增 7 项；严格 Clippy、格式及 70 份 Schema/TS 检查通过，没有新增运行依赖或字体资源。
- 新增 14 个页面 Native/WASM 请求，其中 11 个成功、3 个失败；全部经过 CLI 输出校验。75 个前期文字页面请求、307 个更早请求保持原响应及成功像素。
- 新增 12 个基础布局 Native/WASM 请求，7 个成功、5 个输入失败，覆盖精确正负偏移、整数表示等价、三种行距、范围、未知字段、非法数值及重复键。本轮实际双端执行共 408 个请求。
- 独立 XML / FontTools / Fraction 参考核对 11 个页面、16 个基线来源声明、32 个基线计算值、33 个行度量、58 个字形坐标、12 行和 6 个装饰矩形；比较 1,266,594 个内部像素，排除 53,406 个边缘像素。
- 基础布局独立有理数参考核对 7 份布局、14 个字形和 91 个数值坐标，Q32 计算一致，已测整数输出最大偏差 1/2 EMU。原生转换单元测试另覆盖正负半值边界、极小值、词法及范围限制。

页面样本覆盖混合正负偏移、非整数百分比、继承覆盖、等值字面量、极小偏移合并、空行/结束样式、自动换行、旋转翻转及装饰线。原有两组“非零 baseline 未实现”断言已由真实成功绘制与失败边界用例替代，其它未支持内容仍保持明确诊断。

复现入口：

```sh
MO_BASELINE_EVIDENCE_DIR="$PWD/.codex-work/native-baseline/cases" cargo test --workspace --locked
PYTHONPATH=.codex-work/font-tools-venv/lib/python3.13/site-packages python3 tools/verification/native-baseline-reference.py
node tools/verification/native-baseline-parity.mjs
node tools/verification/native-baseline-geometry.mjs
python3 tools/verification/native-baseline-numerics.py
node tools/verification/text-page-runtime-regressions.mjs .codex-work/native-baseline
python3 tools/verification/native-baseline-evidence.py
```

按[开发说明](development.md)构建当前 Native worker、Rust WASM 和独立 `.codex-work/native-baseline/wasm-node`，保留前期证据。使用现有 FontTools 4.61.1、NumPy、Pillow、lxml 和 jsonschema 开发依赖；参考图来自原创轮廓、独立源属性/度量/变换计算，已选字形和行拓扑仍作为输入。像素验证排除 2 像素边缘，不认证抗锯齿、真实字体或 Office/WPS 视觉质量。

完整字符/段落排版、竖排、自动适配、高级内容、生产 Agent/Artifact/Worker、Musterwork 替换及目标应用往返仍未完成；本轮没有产品级耗时、RSS 或安装包体积测量。

后续[段落百分比间距](paragraph-spacing.md)复用本阶段的精确百分比运算，接入原生行距和段前/段后布局；本页保留基线阶段的历史验证范围。
