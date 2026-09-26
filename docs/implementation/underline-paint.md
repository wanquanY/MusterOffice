# 独立下划线画笔

后续已将[原生文字基线偏移](native-baseline.md)接入同一文字和装饰几何链路；下文保留本阶段范围与冻结证据。

2026-09-25，开发态实现。延续[原生文字装饰](text-decorations.md)，将 `a:uFill` 的纯色和 `noFill` 接入同一 Rust 页面编译、Native / WASM 绘制及 CLI 输出。原定完整演示文稿范围保持不变。

## 语义与实现边界

字符填充、下划线填充分别继承。`uFillTx` 显式选择当前文字填充；`uFill` 持有独立子填充。`u="none"` 不执行未启用的下划线画笔。字符 `noFill` 不隐藏独立着色的下划线；下划线 `noFill` 不隐藏文字或删除线，也不要求字体提供下划线度量。单删除线继续跟随文字填充。

库层使用 `TextRunPaint`，分别保留字形填充、下划线画笔及删除线开关。独立画笔同时记录 `uFill` 包装节点和直接填充子节点的来源；缺失系统色、缺失占位色及未实现填充可精确定位到段落、run 和原始 XML 物理序号。`FollowText` 借用已有字形填充，不重复求值或复制颜色依赖；独立画笔放入 Box，避免普通文字为此预留大枚举空间。这是结构选择，尚无产品级内存收益测量。

两种画笔共用颜色引擎、主题/映射/显式上下文和同一个计算预算。每个 run 占一个画笔槽，独立下划线再占一个，跟随文字不额外占槽。颜色变换步骤同样累计，不因增加通道而重置限额。占位色仍按需使用 `fontRef` 或宿主显式上下文；核心不读取系统颜色或字体。

字簇绑定比较完整工作精度颜色、下划线及删除线状态。同一个塑形字簇跨越不同下划线颜色时返回 `glyphPaintConflict`，没有部分图像；来源声明不同而有效画笔相同时可共享该字簇。可见下划线使用自己的 RGBA，沿用选定字体实例度量、advance 范围、Q32 坐标及页面变换。Native / WASM / CLI 的公开运行合同不变，只有 Rust 库计划中的画笔诊断结构扩展。

语义依据仓库固定 ECMA-376 Part 1 §21.1.2.3.12–15；[Microsoft 的 uFill 说明](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.underlinefill?view=openxml-3.0.1)也将下划线填充定义为独立于文字填充。当前仅支持单下划线的纯色 / noFill；渐变、图案、图片、独立 `uLn` 描边和其它下划线形态继续明确返回未实现，不因源声明已经可读取而宣称可绘制。

## 验证

[冻结证据](../reviews/evidence/2026-09-25-underline-paint-verification.json)绑定本次源码、构建、请求、响应及像素摘要：

- 480 项 Rust 测试通过，新增 6 项，严格 Clippy、格式、70 份生成 Schema 和 TS 类型检查通过；没有新增运行依赖。
- 20 个新 Native / WASM 请求逐字节一致，其中 16 个成功、4 个失败；同一批 CLI 请求核对响应、实际输出及失败无输出。55 个此前文字页面请求保持原响应及成功像素，307 个更早请求回归通过；本次实际重放共 382 个请求。
- 12 个库级成功样本覆盖不同颜色、双方独立 noFill、继承覆盖、组合字符、旋转/翻转、占位色、系统色及未启用画笔。独立 XML / FontTools / Fraction 参考核对 42 个画笔来源声明、36 个行度量值、11 个装饰矩形及 1,381,352 个内部像素，排除 58,648 个边缘像素。
- 额外运行请求验证补齐显式系统色/占位色后恢复绘制；使用此前固定的缺少 `post` 表字体，验证隐藏下划线不要求其度量，同时保留可见文字和删除线。

失败样本包括缺失上下文、图案画笔及同字簇颜色冲突。前三级画笔错误发生在字体/栅格组件调用前；字簇冲突在塑形后、栅格前失败。预算/取消测试同时验证跟随和独立画笔的实际成本。

核心复现命令：

```sh
MO_UNDERLINE_EVIDENCE_DIR="$PWD/.codex-work/underline-paint/cases" cargo test --workspace --locked
PYTHONPATH=.codex-work/font-tools-venv/lib/python3.13/site-packages python3 tools/verification/underline-paint-reference.py
node tools/verification/underline-paint-parity.mjs
node tools/verification/text-page-runtime-regressions.mjs .codex-work/underline-paint
python3 tools/verification/underline-paint-evidence.py
```

跨端探针需先按[开发说明](development.md)构建当前 worker 和 `.codex-work/underline-paint/wasm-node`，并保留上一阶段冻结证据及其输入。使用开发环境已有 FontTools 4.61.1、NumPy、Pillow、lxml、jsonschema；原创字体和固定 HarfBuzz / Skia 不变。验证器默认只校验，`--seal` 仅用于第一次生成新证据，拒绝覆盖。

参考使用已选字形整数及断行拓扑作为输入，独立计算样式、度量、变换、矩形和图像；它不认证全部塑形。像素样本为不透明原创轮廓，排除 2 像素边缘带，不代表抗锯齿、透明混合、真实字体或 Office/WPS 视觉验收。完整文字布局、其它装饰、图片/效果、高级内容、生产 Agent/Artifact/Worker 与 Musterwork 接入仍待实现；本轮没有产品延迟、RSS 或安装包体积测量。
