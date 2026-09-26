# 组合缩放、旋转与翻转的目标应用核对

2026-09-24 · 本文记录组变换修正时的证据；后续已用[静态旋转导出](static-rotation-export.md)解决其中 15 个 LibreOffice 角度差异，历史输入和证据没有覆盖。

已修正 `mo-presentation-compile` 的组变换求值。当前 profile 为 `drawingml-sector-scale-reflection-q96-enclosure-v2-draft`；它替代解释性附录推导出的 v1。当前模型范围、数值区间、接口及生产页面编译缺口见[作者页面坐标](page-placement.md)。

## 根因与规则

v1 将组轴向缩放分别累计、旋转直接相加，中心沿原始矩阵链移动。四个可编辑 PPTX 在 LibreOffice 的几何结果与内核相差约 28–60 pt；随后直接打开其中一份，WPS 同样显示了比例差异。因此，原生/WASM 相同和高精度计算不能证明作者语义正确。

新规则使用每个父组已经求值的坐标空间。父组非负轴向比例为 `(sx,sy)`，翻转位为 `(hx,hy)`，旋转为 `r`；子对象原始旋转 `a` 按整圈 21600000 单位取 Euclidean 余数：

1. 当 `a` 位于 `[45°,135°)` 或 `[225°,315°)`，子对象宽、高分别使用父组的 `sy,sx`；其他区间使用 `sx,sy`。区间判断使用完整的 1/60000 度单位，不截断整数角度。
2. 翻转按对应轴异或累计，不随缩放比例交换。父组恰好翻转一个轴时，子对象旋转方向取反；求值角为 `r ± a`。
3. 对象中心通过父组求值后的空间定位；求值后的组自身再成为下一级的坐标空间。不能重新应用未求值的祖先矩阵链，否则嵌套时会重新引入剪切和位置偏差。
4. 当前对象的 viewport 到 size 比例仍由对象自身承担；翻转与非负比例分开保存，包括退化轴，避免依赖矩阵数值分解识别方向。

底层 `mo-geometry` / Draw IR 的普通仿射规则没有改变。这一阶段没有改写导出作者对象、原始角度、组层级或可编辑路径；后续静态导出表示调整见上方链接。没有图片展平或按目标应用另画一套场景。

## 依据与适用程度

[ECMA-376 Part 1](https://ecma-international.org/wp-content/uploads/ECMA-376-1_5th_edition_december_2016.zip) 附录 L.4.7 是解释性材料。微软的 [20.1.7.5 实现说明](https://learn.microsoft.com/lb-lu/openspecs/office_standards/ms-oi29500/98567b7e-097a-4450-a82a-f83e25539aaf) 说明了 child bounding box 及角度单位，但没有完整给出本次发现的角度分区规则，不能据此声称 Office 实测通过。

定位过程中查阅了 [LibreOffice 当前安装版本的 DrawingML 源码](https://github.com/LibreOffice/core/blob/afbbd0df0edb6d40b450b0337ac646b0913a760c/oox/source/drawingml/shape.cxx)，作为解释轴交换与翻转行为的线索；没有迁入其源码、引入运行依赖或沿用它对未归一化角度的处理。最终规则由自有非对称形状在目标 WPS 的实际结果核对。该外部源码摘要与来源角色记录在[本轮证据](../reviews/evidence/2026-09-24-group-placement-verification.json)。

WPS 12.1.22553 在调用者已有权限的环境下，已通过 `cua-driver` 后台打开三份自有测试文稿并取得原生窗口截图；没有启动或修改此前权限不足的守护进程。旧记录中的守护进程限制仍属历史事实，已不再代表本机完全无法进行 WPS 观察。完整截图含其他应用页签，留在忽略的临时目录；公开记录只包含自有图形几何、摘要和验证方法。

## 本轮验证

三份文稿共 48 个非对称原生路径，覆盖 45/135/225/315 度边界的前后一个最小单位、负角度、多圈角度、父子各轴翻转和两层旋转组合；另有 12 个页面定位标记。非对称轮廓能观察矩形无法区分的镜像和 180 度方向差异。

- 67 批作者坐标请求在真实 Native/WASM 入口一致；其中 55 批成功，独立 140 位 Decimal 参考核对 8931 个对象、53586 项系数/中心、44854 个源控制点。
- 7 份诊断页面经真实 Native/WASM Skia 绘制，像素相同；同一作者文稿导出的原生可编辑 PPTX 逐字节相同。诊断桥仍只处理经过断言的纯色路径/矩形，不是通用页面编译器。
- WPS 三份截图通过四个自有标记配准。48 个图形均在本次 2.5 截图像素的粗粒度几何观察容差内，最大约 2.07。截图存在缩放、重采样和压缩；测量比较边界采样点与顶点，**不代表像素保真、无损几何证明、Office 行为或外部编辑往返验收**。
- 原先四份矩形的 LibreOffice PDF 与修正后的内核比较，最大顶点到边界偏差约 0.0843 pt，均低于原有 0.1 pt 观察阈值。PPTX/PDF 输入没有改写。
- 新增 48 个图形在 LibreOffice PDF 中，33 个位于 0.2 pt 观察容差内，15 个负角度或多圈角度样本仍明显不同，最大约 18.26 pt。WPS 对这些角度按整圈等价处理，已由实际样本核对；不同应用的结果分别保留，不把 LibreOffice 当作唯一真值。
- 235 项 Rust 测试、严格 Clippy、46 份 Schema 和 TS 检查通过。此前与作者坐标无关的 2180 批在新产物上结果不变；连同更新后的 67 批，主内核累计 2247 批。Schema 与外部依赖版本均未变。

未压缩开发产物为 CLI 4113632 字节、文字 worker 2375232 字节、绘制 worker 2839936 字节、Rust WASM 3737429 字节；Rust glue 25307 字节。两套 C++ WASM/薄层未变。这些是部分开发产物，不是完整内核或 Musterwork 安装包；本轮没有新的完整性能结论。

## 重现与后续

```sh
node tools/verification/page-placement-parity.mjs
python3 tools/verification/page-placement-reference.py
node tools/verification/page-placement-render.mjs
python3 tools/verification/group-placement-fixtures.py
node tools/verification/group-placement-parity.mjs
python3 tools/verification/page-placement-reference.py .codex-work/group-compat
python3 tools/verification/contracts.py \
  --page-placement-report .codex-work/page-placement/parity.json \
  --page-placement-report .codex-work/group-compat/parity.json
```

外部步骤先用隔离配置的 LibreOffice 转换自有 PPTX，使用 `cua-driver` 打开三份原始文稿并保存实际 WPS 窗口截图。`group-placement-observations.py` 只接受本次已目视确认的窗口尺寸与定位标记范围；换机器、窗口或应用版本须重新检查，不能盲用屏幕坐标。该开发脚本使用 PyMuPDF/Pillow/NumPy，不进入内核依赖。具体输入与输出摘要见[证据](../reviews/evidence/2026-09-24-group-placement-verification.json)。

接下来需要核对目标 PowerPoint、任意 chOff/零 chExt、更多嵌套与对象类型及外部编辑往返，并把此阶段接入完整页面编译。当前证据不关闭 E0/E1–E3 或 Musterwork 替换门禁。
