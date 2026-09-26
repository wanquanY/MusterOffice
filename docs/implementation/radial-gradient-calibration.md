# 径向焦点的 WPS 校准

2026-09-25：11 份自有 PPTX 已在 WPS 12.1.22553 实际打开并取得窗口截图。观察推翻了上一阶段对偏移焦点位置的统一解释：将位置和缩放都应用到外圆包围正方形，会使非正方形对象上的偏移焦点明显错位。

核心新增显式 `drawingml-circle-anchor-focus-q96-v2-draft`，将焦点位置与外圆尺度分开计算，已接通原有 Native/WASM/CLI 查询。旧 v1 仍保持既有几何输出，以便复核历史证据。这次尚未将径向着色器接入来源页面，也不代表 Office/WPS 全部兼容或 Musterwork 可替换。

## 实测与推断

自有输入使用原生矩形、三个不同灰度色标以及四个洋红色定位标记。三个色标使已知 Office 特殊 gamma 条件不成立；没有文本、字体依赖或图片展平。观察脚本以原始 XML 为输入，比较几何假设，不使用内核结果反推预期。下表是截图采样与假设模型之间的平均灰度差，单位为 0–255 色阶；它不是生产渲染器的性能或保真成绩。

| 样例 | 圆包围框定位 | 矩形定位后缩放外圆 |
| --- | ---: | ---: |
| 偏移点焦点 | 11.986 | 0.289 |
| 外部点焦点 | 5.065 | 0.704 |
| 偏移椭圆焦点 | 6.049 | 0.789 |
| 外部椭圆焦点 | 5.482 | 0.553 |
| 等宽但偏移的焦点矩形 | 6.973 | 0.107 |

两种解释在普通中心点和中心椭圆上相同，所以只测试中心渐变无法发现问题。新模型在所测偏移样例中更接近 WPS；截图仍有重采样和亚像素配准误差，当前不足以宣称逐像素相同。近等宽样例保留了明显差异，见下文。

着色方面，居中非均匀椭圆的“沿同一射线混合”模型平均差为 2.512，而逐步改变椭圆中心及轴长的模型为 0.530。这是选择后续算法的实验依据，尚不是所有输入的规范证明。诊断脚本先在 128 个区间找首次进入，再二分；它不是经过证明的四次方程求解器，也没有进入产品绘制路径。

[微软关于渐变的说明](https://learn.microsoft.com/en-us/answers/questions/2248059/non-preset-a-tilerect-behaves-strange-in-case-of-g)与[焦点转换解释](https://learn.microsoft.com/en-us/answers/questions/2247174/how-is-attribute-filltorect-evaluated-for-a-gradie)提供了外圆、缩放和焦点的线索，但其中参考矩形措辞有歧义。当前推断以这批 WPS 观察为限，不能据此声称 PowerPoint 实测通过。[Skia 双圆渐变](https://docs.skia.org/docs/dev/design/conical/)的多解选择也不能直接视为 Office 径向规则；非均匀内椭圆更不能用两个圆替代。

## 核心修正

路径边界、`tileRect`、外圆及数值误差传播继续复用[径向几何实现](radial-gradient-layout.md)。每轴用 anchor 的起点 X、长度 D 求出内部矩形 X′、D′，计算 S=D′/D 以及 P=X′+D′(X′−X)/(D−D′)。然后将外圆中心 C 和半径 R 变为内椭圆：

```text
内椭圆中心 C′ = S*C + (1-S)*P
内椭圆半径 R′ = S*R
```

因此位置依据矩形，半径依据外圆。实际实现保留精确百分比分类；当分母非零，以代数化简避免很远的 P 放大区间依赖。精确 D=D′ 时使用有限 P=X′，该轴 S=1，内椭圆中心保持 C。这也解释了等宽偏移为何不直接移动内椭圆。

`CircleFocusBasis` 在同一个求值器内明确选择规则，没有复制文档模型、来源解析或路径编译。新增 `layout_paths_with_basis`、`layout_background_with_basis` 和 `layout_source_with_basis`；公共 API 按请求 profile 选择。同一源包、原生声明和可编辑对象不变。

**仍未关闭的精度语义差异：**相对边距相加约为 10⁻²⁰ 时，核心仍按精确非零处理，WPS 观察却与等宽分支相同。新精确模型该样例平均差 6.006 色阶；模拟浮点等宽分支可降至 0.107，但这不足以确定正确阈值、计算单位及所有边界。因此没有把经验阈值写入生产代码。后续需补测边界并明确格式数值语义，不能靠截图拟合常数关闭问题。

## 验证与状态

[阶段证据](../reviews/evidence/2026-09-25-radial-anchor-verification.json)记录源码、产物与范围：

- 611 项 Rust 测试、严格 Clippy、格式及 82 份 Schema/TS 检查通过。新增两项测试覆盖焦点位置、缩放、精确退化、全部来源语料与原始绑定保留。
- 82 对实际 Native/WASM 查询一致，包含 69 个成功查询、73 个目标计划和 13 个错误查询。旧 42 个请求中 41 个响应字节不变；一个未知 profile 错误只扩展了有效枚举列表。
- 独立原始 XML/160 位 Decimal 参考核对 38 个目标的 722 个数值与误差界；四个复杂预设仍只做双端一致性验证。40 份 PPTX 的 280 个 XML 部件通过官方 XSD。
- 1,116 对通用绘图、139 对来源页面和 307 对页面/文字回归保持结果；组件、字体/图片依赖未变。
- 只扩展一份请求 Schema 和生成 TS 的 profile 枚举，其他 81 份合同及生成类型保持不变；实际请求/响应另经 Schema 验证。
- 11 份 WPS 输入、原始窗口截图、配准和自有画布裁剪有摘要记录。窗口截图含应用其他页签，保留在忽略目录，公开文档只记录自有内容及结论。

裸 Rust WASM 为 **6,398,266 字节**，比上一阶段增加 1,630；固定 C++ WASM 仍为 **2,327,852 字节**。没有新增第三方依赖，没有新的完整页面性能或安装包测量。

本次没有完成实际径向像素输出。下一步实现有界的椭圆插值求解及共享绘制，接入世界画笔和页面预检，再继续形状渐变、固定方向、高级内容及 Musterwork 替换验收。

## 复现

```sh
python3 tools/verification/radial-observation-fixtures.py
python3 tools/verification/radial-anchor-checks.py
node tools/verification/radial-anchor-parity.mjs
python3 tools/verification/radial-anchor-reference.py
node tools/verification/radial-anchor-regressions.mjs
node tools/verification/radial-anchor-source-regressions.mjs
node tools/verification/text-page-runtime-regressions.mjs .codex-work/radial-observation .codex-work/rect-gradient/component .codex-work/radial-observation/ts-raster/index.js
python3 tools/verification/contracts.py
python3 tools/verification/radial-observation-compare.py
python3 tools/verification/radial-anchor-evidence.py
```

比较脚本需要真实 WPS 截图及状态文件；不能凭运行数学模型生成这些输入。更换窗口或机器须重新观察定位区域。保留本阶段目录，后续开发另开产物目录；封存后使用证据脚本的 `--check` 核对。
