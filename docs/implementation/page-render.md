# 作者页面编译与纯色路径绘制

2026-09-24：当前实现把作者文稿、组合坐标、解析几何、Draw IR 和真实 CPU 绘制接成一个 Rust 计算入口。它是完整页面内核的增量实现，尚未覆盖全部页面内容，也没有关闭 E0 或 Musterwork 替换门禁。验证记录见[页面绘制证据](../reviews/evidence/2026-09-24-page-render-verification.json)。

## 当前计算边界

`mo-presentation-compile` 接受 document、slide、viewport 和显式绘制默认值，不依赖 Musterwork、模型推理、任意路径、网络或系统字体。初始 profile 为 `author-page-solid-paths-certified-cpu-v1-draft`；后续[作者描边](stroke-author.md)已升级至 v2，增加画笔角色和独立尖角参数误差。

支持矩形、椭圆、圆角矩形及现有作者路径的直线/二次/三次曲线，纯色或无填充、RGBA 透明叠加、组变换和现有母版/版式/页面的对象次序。圆角半径不得超过短边的一半。自定义路径使用 nonzero 填充规则；未增加作者模型中没有的路径规则。

背景按页面→版式→母版寻找首个显式值，全部继承时使用宿主提供的 `pageBackground`。显式无填充保留 viewport 的 RGBA 背景，与继承默认背景不同。主题色先查询当前母版关联主题，缺槽时查显式 `themeColors`；仍缺失则带对象与缺槽信息返回输入错误。隐藏页面可显式绘制，并保留 hidden 标记。

当前形状必须没有文本，且显式声明 fill 和 stroke。后续 v2 已支持显式纯色描边及其端点/连接，未解析的端点/连接和填充/描边继承仍整页失败；图片、连接线和文字也尚未连接。组合提供坐标空间，不生成自身填充；组描边明确拒绝。既有独立文本管线仍可用，但尚未在这个页面入口连接。不会通过漏画、替代截图或只输出其余对象报告成功。

当前模型尚无完整占位符替换、母版对象显隐、效果和所有 DrawingML 预设；此入口不能据此声称完整母版/版式渲染。来源 PPTX 到完整作者模型的导入也仍待实现。

## 几何和误差

复用[作者页面坐标](page-placement.md)的 Q96 区间与已验证的组语义。每个形状在以自身源尺寸中心为原点的空间生成路径，再使用已经求值的页面矩阵；不会重复叠加组变换。

椭圆和圆角圆弧使用正弦/余弦的三次 Hermite 插值。单段弧度跨度为 h、轴向半径为 r 时，该轴的连续位置误差不超过 `r*h^4/384`：四阶导数界为 r，Hermite 余项的最大因子为 `h^4/(16*24)`。这不是把四段经验 Bézier 近似当作精确椭圆。

每象限自适应使用 1、2、4、8、16、32 或 64 段，先要求变换后的解析误差不超过总容差的四分之一。控制坐标的 Q32 量化误差向外累计；过严容差返回 `PRECISION_EXCEEDED`。三角值在同页共享缓存，不引入平台 libm 差异。

总预算分为：

1. 作者矩阵与中心量化误差，在所有生成控制点上计算最大值。
2. 曲线近似与本地控制坐标量化误差，用 `abs(量化矩阵)+矩阵不确定度` 放大并换算为像素。
3. Draw IR 到实际设备路径的变换和 float32 坐标误差。

三个预算相加不得超过原始 `coordinateTolerance`。场景编译只获得扣除前两项后的剩余预算。所有上界以像素单位的原始 Q32 整数字符串传递。它们约束几何坐标，不是抗锯齿覆盖率、颜色误差或与 Office 画面的像素差承诺。

当前 viewport 必须从零原点按单一有理比例精确覆盖页面。该约束使后端帧裁剪等于页面裁剪；任意视口、切片和复杂 clip 需先实现对应语义，不能通过忽略裁剪放开参数。

## 资源、执行与失败

相同本地路径和相同求值变换按值复用，颜色及绘制顺序仍属于各个实例。背景使用 identity 实例，不消耗变换节点。8192 个相同对象的实测计划只含一份路径和一个变换。

请求上限 32 MiB、作者对象上限 8192；最多 4096 个唯一填充路径、262144 条唯一命令、1048576 条实例展开命令和 8192 个变换。继续执行既有[场景](scene-raster.md)和[栅格](path-raster.md)的设备范围、工作量与像素内存限制。超限失败，不截断文稿。

`compile_page` 已执行设备降低和全部坐标认证，返回可检查的作者计划、绘制来源、设备工作统计和总误差。它不调用绘制后端。`render_page` 使用同一准备过程生成的私有已认证批次，直接执行一次绘制，不重复编译场景，也不信任外部传回的计划。

取消检查覆盖遍历、几何展开、控制点预算和现有降低/后端边界；模型完整性验证、规范摘要和序列化的细粒度取消仍待完善。失败没有像素或部分计划。Native 由独立 raster worker 处理字节请求；CLI 校验结果后原子发布且拒绝覆盖。WASM 继续使用现有失效隔离规则，宿主 Worker 与生产实例池仍待建设。

## 开发入口与验证

请求见 [PageRenderRequest](../../contracts/generated/page-render-request.schema.json)，编译响应见 [PageCompileResponse](../../contracts/generated/page-compile-response.schema.json)，绘制响应见 [PageRasterResponse](../../contracts/generated/page-raster-response.schema.json)。Schema 和 TS 由同一 Rust 类型生成。

- CLI：`mo-cli compile-page <request.json>`；`mo-cli render-page <request.json> <new-output.rgba>`。
- WASM：`compile_page(requestJson)`；`render_page(requestJson, rasterComponent)`。后者与既有场景接口一样，元数据和像素分别传输。
- 内部 Native worker：`mo-raster-worker --page`，使用长度前缀字节协议，不接受输入/输出文件路径。

初始 74 个逻辑页面用例分别核对编译和绘制两个跨端入口：53 个成功、21 个拒绝；计划、元数据和像素完全一致。覆盖曲线、极端角度、翻转、组合、主题/背景/层次、透明叠加、路径洞、退化尺寸、8192 个重复对象、非法输入、范围/精度与未实现内容。三份既有组合页面的像素不变。另有三种桥接故障核对失效回收，CLI 核对拒绝覆盖和失败不生成文件。

140 位 Decimal 独立参考核对 8305 个对象、71806 项控制坐标、6300 个真实圆弧采样点和 26 项连续 Hermite 余项预算。八份同源原生 PPTX 的 Native/WASM 字节一致，并通过独立 ZIP/XML/对象核对、48 次官方 ECMA XSD 部件校验；LibreOffice 26.2.0.3 的实际 PDF 几何观察最大采样边界差约 0.0932 个 96 dpi 像素，低于事先设置的 0.27 像素观察容差。该观察不是全画面像素比较，也未验证这些新增曲线的 WPS/PowerPoint 编辑往返。

全量回归为 242 项 Rust 测试、原有 2271 批结果不变、新增 74 个页面逻辑用例，合计 2345 批；页面的两个 API 调用不重复计作两个逻辑用例。49 份 Schema、TS、严格 Clippy 和格式检查通过。八份外部样本的额外导出对比单列，不加入该主内核累计批数。

```sh
cargo test --workspace --locked --offline
cargo build -p mo-cli -p mo-text-worker -p mo-raster-worker --release --locked --offline
cargo build -p mo-wasm --target wasm32-unknown-unknown --release --locked --offline
node tools/contracts/build-wasm-node.mjs
node tools/verification/page-render-parity.mjs
python3 tools/verification/page-render-reference.py
python3 tools/verification/contracts.py --page-render-report .codex-work/page-render/parity.json
node tools/verification/page-render-external-fixtures.mjs
```

跨端脚本的三个历史组样本需要先按[静态旋转导出](static-rotation-export.md)生成。外部观察还需要明确提供 LibreOffice 可执行文件，并使用带 PyMuPDF/NumPy/Pillow 的开发 Python；详见验证脚本。所有样本都是本仓库生成的自有内容。新增页面能力没有新增外部运行依赖；内部计算 crate 增加对既有场景/栅格 crate 的依赖。

后续 v2 已接通显式作者描边，页面用例扩为 131 批；最新数据及目标应用尖角差异见[作者描边](stroke-author.md)。接下来连接显式字体资源、主题/段落/形状文字、完整线型与图像资源，继续完成通用 Draw IR、播放、高级可编辑对象、公开 Agent 接入和 Musterwork E0–E3。完整一期功能与质量目标不变。
