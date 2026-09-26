# 来源颜色计算

2026-09-24 · 在真实 PPTX 的主题与颜色映射基础上增加按需计算。它为后续填充、线条、文字和效果编译提供颜色基础，尚不构成完整渲染器或 Office/WPS 兼容认证。实现与验证摘要见[当前进度](progress.md)。

## 计算与来源分离

`mo-color` 是无 I/O 的共用数学模块，不知道 PPTX、主题或宿主。它在 sRGB、线性 RGB、HSL 三种工作表示间按操作需要转换，使用完整 `f64` 精度。连续 HSL 操作保留色相、饱和度和亮度，包括零饱和度、零亮度和色相上限；采样不会改变工作表示。颜色引用也传递完整工作状态，不能在引用边界先转成 8 位 RGB。

`mo-pptx::source::color` 负责六种来源颜色表示、190 个原生预设色、17 种方案色查询及 28 种有序变换。原始词法、变换顺序与源节点仍由来源索引保存；求值不修改任何声明或文件。计算结果提供 RGBA8/RGBA16、显示色域裁剪标志、主题节点依赖链与上下文来源。

方案色先使用当前表面的有效颜色映射，再读取独立选择的颜色方案。`dk1/lt1/dk2/lt2` 直接定位色槽。递归方案引用检查活动路径，12 个色槽使最大递归深度有界；循环会返回明确诊断。缺少映射、方案、色槽、系统颜色或占位颜色也返回未解析，不伪造黑色或宿主默认值。

系统颜色优先使用显式 `context.systemColors`，否则使用文件中的 `lastClr`，并在结果中区分二者。`phClr` 需要调用方提供该样式的占位颜色上下文。本页的主题色查询接收显式上下文；后续[线条颜色入口](line-colors.md)已接入线条继承和原生占位表达式，仍不能宣称已解析所有对象样式的 `phClr`。

## 数值 profile 与互操作边界

请求必须明确指定 `ecma376-2016-draft-v1`。该名称表示当前可复现的数值解释，**不是已通过验收的 Office/WPS profile**。依据是 [ECMA-376 Part 1 第五版](https://ecma-international.org/publications-and-standards/standards/ecma-376/)的 20.1.2.3 与 20.1.10.50；标准 PDF 的获取位置和摘要保存在[本轮证据](../reviews/evidence/2026-09-24-pptx-colors-verification.json)。

| 类别 | 当前解释与约束 |
| --- | --- |
| scRGB / sRGB | scRGB 保留线性通道，sRGB 使用分段传递曲线；最终采样才量化为 8/16 位 |
| RGB 通道变换 | 按标准示例在编码 sRGB 中设置、偏移、调制并裁剪；例如全红调制 50% 得到 128 |
| tint / shade / inv | 在线性通道计算；绿色 tint 50% 得到 `(188,255,188)` |
| HSL | 普通通道变换裁剪，色相补色循环旋转；连续变换保留 HSL 工作状态 |
| alpha | 非预乘透明度，按声明顺序计算并限制到有效区间 |
| gamma / invGamma | 对线性分量应用 sRGB 正向/反向传递函数，再按目标空间采样 |
| gray | 本 profile 显式采用 22/72/6 权重，返回 `grayWeightsProvisional`；不把该选择说成唯一规范系数 |
| 预设色 | 与经摘要核实的标准表逐项核对；短名 `ltGoldenrodYellow` 在该版标准表为 `(250,250,120)`，不同于长名，返回差异提示 |

实际 LibreOffice 的 18 个探针中 6 个颜色相等、12 个存在差异。RGB 调制/偏移输出与上述标准示例不同，部分通道还存在量化差异；scRGB/HSL 的整数属性对照与等价百分比属性输出也不同。连续 HSL 与跨方案引用同样观察到差异。观察工具保存实际 PDF 摘要、样本颜色及逐通道差异，不通过改变预期值把差异改成通过。Office、WPS 的目标版本验证仍待完成，后续兼容策略必须基于这些目标应用的可复现证据。

## 接口与资源限制

开发 CLI 为 `mo-cli pptx-colors <request.json> <source.pptx>`；WASM 为 `resolve_pptx_colors(requestJson, sourceBytes)`。两者使用相同 Rust 计算，输入实际文件，不接受调用方伪造的来源索引。Rust 内部 `source::color::query` 可复用已经检查过的不可变索引，避免每次颜色计算重新读取文稿。

请求遵循生成的 [SourceColorQuery](../../contracts/generated/pptx-color-query.schema.json)，响应遵循 [PptxColorResponse](../../contracts/generated/pptx-color-response.schema.json)。必需字段包括源 SHA-256、表面部件、profile、查询色名和显式 context。摘要冲突、非法请求、取消与预算超限保持独立错误类型；某个颜色缺少语义上下文则在该结果中返回 `unresolved`。

默认每次最多 256 项查询、65536 个计算步骤、合计 64 KiB 百分比词法计算量；引用与变换共享预算，每步检查取消。源文件读取还受 XML/OPC/主题总预算约束。超出 `f64` 有限范围的合法来源词法保留原样，计算返回 `numericRange`。CLI/WASM 开发字节入口保留已有 128 MiB 源文件、32 MiB JSON 限制；生产 Worker 与操作服务接入尚待实现。

数学依赖只有新增 `libm 0.2.16`，关闭默认特性并启用 `force-soft-floats`，使 Native/WASM 的非线性计算使用同一实现。它不引入浏览器、Python、LibreOffice 或系统颜色依赖，见[依赖记录](dependencies.md)。

## 验证与复现

真实 PPTX 语料覆盖全部模型/变换/预设色，另验证母版/版式/页面映射、分项主题覆盖、MCE、循环、缺失上下文、原始小数精度、顺序变换、工作空间状态及超限拒绝。独立参考程序从实际 ZIP/XML 和关系重新定位颜色，使用 64 位有效数字的十进制计算与不同的 HSL 算法；RGBA8 要求精确相等，RGBA16 允许最多 1 个码值的参考误差，本轮实际差异为 0。参考舍入保留 48 位小数以去除精确半整数边界的计算尾差，不放宽输出颜色检查。

标准 PDF 仅放在忽略缓存。先从上述官方地址取得 Part 1 第五版 PDF，`color-preset-facts.py` 会核对固定 SHA-256，再提取事实并检查 Rust 表。该脚本及外部 PDF 观察器需要开发环境 PyMuPDF，独立 XML/合同验证还使用 lxml/jsonschema；它们不进入核心依赖。

```sh
python3 tools/verification/color-preset-facts.py .codex-work/ecma376/part1.pdf .codex-work/ecma376/preset-values.json
python3 tools/verification/pptx-color-fixtures.py .codex-work/pptx-fixture/native-initial.pptx .codex-work/ecma376/preset-values.json .codex-work/pptx-colors-fixtures
node tools/verification/pptx-color-parity.mjs target/release/mo-cli .codex-work/wasm-node/mo_wasm.js .codex-work/pptx-colors-fixtures/manifest.json .codex-work/pptx-color-map-fixtures/manifest.json > .codex-work/pptx-colors-parity.json
python3 tools/verification/pptx-color-independent.py .codex-work/pptx-colors-parity.json .codex-work/ecma376/preset-values.json --xsd-dir .codex-work/ecma376/xsd > .codex-work/pptx-colors-independent.json
```

基础导出和来源语料的生成步骤见[开发说明](development.md)及[颜色映射](source-color-mapping.md)。外部观察使用 `pptx-color-external.py`，显式传入本机 LibreOffice 路径及不存在的 PDF 输出目录。外部颜色相等也不能证明完整页面、可编辑性、播放或往返互操作。
