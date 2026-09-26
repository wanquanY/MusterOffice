# 原生来源页面：图片、形状与文字的统一编译

状态：已实现来源页面、资源解码与绘制的共同链路，并接通 Rust、Native worker、WASM 和 CLI。[阶段验证记录](../reviews/evidence/2026-09-25-source-resource-page-verification.json)绑定源码、产物及检查结果。完整一期能力及 Musterwork 替换验收仍未完成。

后续[组合图片填充继承](group-image-inheritance.md)已修正本阶段对 `grpFill` 的过宽拒绝，[背景合成](background-compositing.md)已接入开发版背景恢复策略并记录透明度/平铺的应用差异。下述阶段验证数字保留历史含义；背景兼容性尚未通过目标 Office/WPS 验收。

## 入口与职责

`mo-presentation-compile::source_resource_page::prepare` 接受同一包计算得到的 `Package`、`SourceIndex`、页面请求、图片解码器及可选字体/排版上下文。来源索引属于内核计算结果；不得把用户提供的替代索引当作该入口的可信来源。返回的不透明 `PreparedResourcePage` 持有页面和资源，可消费为详细计划，或调用 `render` 绘制。

公共 JSON 边界为 `mo-kernel-api::render_pptx_resource_page_json`。它从实际 PPTX 字节打开一次包、计算来源索引并校验摘要，加载显式字体资源，再进入同一个页面计算过程。失败返回结构化诊断和空像素，不发布部分页面。

- profile：`drawingml-resource-page-q32-v1-draft`。
- 请求：`PptxResourcePageRequest`，包含既有 `SourcePageRequest`、`imageSource`、`sampling` 和可选 `fonts`。
- `imageSource` 必须明确为 `embeddedSnapshot` 或 `linkedSource`；缺失指定关系、外部 URI 或尚未实现的内容均返回诊断。内核不访问网络，也不偷偷改用另一来源。
- `sampling` 明确选择 `nearest` 或 `linear`。
- `fonts: null` 配合空字体字节允许无文字页面。可见文字要求字体清单和排版组件；没有清单却传入字体字节属于请求错误。
- `page.profile` 继续选择既有形状策略；外层 resource profile 声明新增图片与可选文字能力。旧资源无关入口保持原有能力边界。

WASM 导出 `render_pptx_resource_page(request, source, fonts, decoder, shaping, raster)`。解码器、排版器和绘制器是独立能力参数，宿主可以复用同一个已验证的图片/绘制组件实例。JS/TS 不重新实现来源解析、布局或坐标计算。

Native worker 模式为 `--pptx-resource-page`，输入使用三个小端 u32 长度及依次排列的请求 JSON、PPTX、字体字节。输出沿用元数据长度、像素长度和对应字节。CLI：

```sh
mo-cli render-pptx-resource-page request.json source.pptx fonts.bin new-output.rgba
```

没有文字时使用空的字体文件。CLI 复用隔离 worker、结果摘要检查和只创建新文件的发布机制。该文件发布尚不是 Musterwork Artifact/CAS 接入。

## 同一页面计算过程

原 `source_page` 已按职责拆为来源对象查询、纯预检、统一绘制三个内部模块。纯几何入口、文字入口及资源入口共用这些模块。

1. 校验来源、页面尺寸和 viewport；选择可见 master/layout/slide 对象，保留原始顺序与声明来源。
2. 预检全部可见形状、填充、轮廓和原生几何，编译路径并累计放置/几何误差。存在字体上下文时，继续预检所有文字声明与资源选择。
3. 仅对真正参与填充的图片目标解析关系，核对声明来源和颜色/背景上下文，执行来源策略及已知语义前置检查。
4. 按编码内容摘要与长度去重，每份资源每页解码一次；使用已验证的解码信息计算局部布局、世界画笔与独立拉伸裁剪。
5. 按页面层级统一输出背景、形状、图片、轮廓和文字，再进入已有共享场景与绘制组件。

图片对象保留 `p:spPr` 填充和 `p:blipFill` 两个来源。绘制顺序为形状填充、图片填充、轮廓；透明图片会露出其形状填充。依据是 ECMA-376 Part 1 §19.3.1.4 的双填充示例，已检查本地固定标准 PDF 第 2574 页（印刷页 2564）中的图示；对应的[微软文档](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.presentation.blipfill?view=openxml-3.0.1)亦说明该示例。普通形状继续保留各原生路径的填充/轮廓顺序。文字在所属对象的形状绘制完成后插入。

`SceneBuilder` 的绘制路径和裁剪路径共用路径/变换驻留与预算。裁剪独立使用其世界变换，图片画笔也已处于世界空间，不能再次乘上被填充形状的变换。图片填充裁剪只约束对应图片绘制，不裁掉该图片下面的形状填充，也不替代形状自身的轮廓边界。

## 坐标与资源预算

上游图片裁剪误差转换为设备像素，记录在可选的 `imageClipCoordinateErrorBound`。它与原有放置、几何误差共同从 viewport 预算扣除；最后的总误差还纳入共享场景和图片画笔各自的下游误差。不能只报告形状坐标误差而遗漏平铺相位或裁剪误差。

当前每页最多 4,096 个实际图片填充使用、4,096 份唯一图片；唯一编码输入总量最多 64 MiB，每份最多 32 MiB，保留的解码 RGBA 总量最多 64 MiB。隐藏对象及没有填充的路径不要求无用的图片解码。

只有一份解码资源时直接移动其像素缓冲；多份资源做一次有界合并，不按放置次数复制图片。结果的 `gatherCopyBytes` 仅记录这次 Rust 合并的复制量，不能当作总内存复制量。解码器 scratch、正在返回的图片、FFI/JS 复制和合并缓冲可能同时存在，因此 64 MiB 是保留资源预算，**不是进程 RSS 上限**。尚未进行本阶段安装包、完整性能或 RSS 基准测试。

## 明确的未完成语义

- 对象图片 `rotWithShape=false` 的独立方向规则；背景不持有对象方向，已可使用页面坐标。
- `useBgFill`：本阶段最初在解码前返回 `fillSpace`；后续[背景合成](background-compositing.md)已接通背景画面恢复。ECMA-376 Part 1 §19.3.1.43 和 [PowerPoint 接口](https://learn.microsoft.com/en-us/office/vba/api/powerpoint.fillformat.background)要求对应背景区域，但半透明和图片采样细节仍需目标应用确认；当前与 LibreOffice 有差异，不能视为兼容性已完成。`grpFill` 继续采用独立的属性继承规则。
- 图片效果、其余图片格式/动画资源、渐变/图案页面填充、复杂轮廓、完整文字布局、图表、SmartArt、公式、播放与媒体等仍按完整一期计划继续实现。
- 新入口完成受支持页面的来源绘制，不代表 Office/WPS 视觉一致、保存重开、完整可编辑性或 Musterwork 产品链路已经验收。

## 验证与复现

原始测试图片由 `tools/verification/resource-page-images.py` 生成，不依赖应用截图、用户资源或隐式系统字体。测试字体仍使用仓库已有的自有合成字体，不能外推为真实字体质量验收。

```sh
python3 tools/verification/resource-page-images.py
python3 tools/verification/resource-page-checks.py
python3 tools/verification/resource-page-fixtures.py
node tools/verification/resource-page-parity.mjs
python3 tools/verification/resource-page-reference.py
python3 tools/verification/contracts.py
```

构建脚本沿用已有固定图片/文字组件，并限制 Cargo 并行数为 2。完整 Rust 测试包括原有静态/文字页面回归，以及新增去重、可见性、双填充、取消、资源诊断和实际 Native 页面测试。运行验证通过实际公共来源页面入口完成 Native/WASM 对比，再验证 CLI 创建、拒绝覆盖及失败不发布。

本阶段通过 570 项 Rust 测试（新增 10 项）、严格 Clippy、格式和 80 份 Schema/同源 TS 检查。24 个来源语料的 168 个 XML 部件通过固定官方 XSD。33 组实际公共 Native/WASM 调用中，17 组成功页面的元数据和像素完全一致，16 组诊断一致；64 份实际请求/响应通过对应 Schema（故意损坏的重复键、未知 profile 请求不作为合法请求校验）。

独立参考直接使用自有 2×2 PNG 图案、来源 XML 的原始尺寸和填充、矩形/圆形遮罩及覆盖顺序，核对五个页面的 559,132 个内部像素，不使用编译计划的坐标生成预期颜色。参考排除了抗锯齿、裁剪及纹素边界附近的像素，因此不覆盖这些边界的视觉质量，也不代替真实字体或 Office/WPS 验收。

本阶段仅改动 Rust/宿主接入和同源合同；固定 C++ 组件、ABI 及第三方版本保持不变，没有新增组件 sanitizer 运行。证据中的 WASM 字节数是特定未压缩构建产物大小，不能视为完整内核或桌面安装包大小。冻结记录复查使用 `python3 tools/verification/resource-page-evidence.py`，后续源码改变应产生新阶段证据，不覆盖历史记录。

后续[原生线性渐变](native-linear-gradients.md)已接入本页面引擎的共享预检与绘制；继承、接收尺寸、组合变换和渐变背景使用同一链路。路径渐变及固定方向等剩余能力继续返回明确诊断，不表示完整渐变已经验收。
