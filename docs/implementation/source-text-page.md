# 原生文字到页面像素

本文件冻结描述库级阶段；其后新增的 CLI/WASM/资源合同与双端执行见[来源文字页面运行接口](text-page-runtime.md)。下文“尚未接入”均指该库级记录时点。

2026-09-25。[文本框布局](source-frame.md)之后，`source_text_page::compile` / `render` 已把来源文字接入实际页面合成：PPTX 原生声明 → 已验证字体 → HarfBuzz 塑形/轮廓 → 框内定位 → 原生纯色画笔 → 页面变换 → Skia 像素。此阶段仍不是完整演示文稿内核或 Musterwork 替换验收通过。

## 入口与共同页面引擎

新 Rust 库入口接收不可变 `SourceIndex`、来源摘要及页面请求、`PreparedManifest`、调用者持有的文字/绘制后端、`TextPageLimits` 和取消回调。核心不增加文件、网络、系统字体或隐式组件生命周期。

原有页面引擎继续负责背景、母版/版式/幻灯片层、可见性、形状路径与画笔、坐标预算及最终绘制。新入口在各对象的形状路径之后、下一个对象之前插入文字；不是将全部文字最后盖在所有形状上。路径与变换继续使用共同的 `SceneBuilder` 去重。

`compile` 返回完整来源/几何计划供查询与诊断；`render` 只返回图像及必要的页面信息、框数和工作计数，并在调用绘制宿主前释放详细塑形和来源计划。普通绘制无需携带全部字体片段和轮廓来源作为响应。验证宿主分别调用这两个入口，核对完整计划与实际像素。

`SourceTextPagePlan.profile` 为 `drawingml-solid-text-page-q32-draft-v1`。其中 `page` 是共用页面几何/绘制计划，内层旧配置名只标识其形状计算策略；额外的 `texts`、`textSources`、`textWork` 定义本次文字能力、字形来源和累计工作量。`texts[].binding` 对应原页面对象，文字实例另绑定框、字形和原始 run 簇，不能用一个没有来源的路径替代可追踪的文字。

旧 `source_page` 入口和已有 CLI/WASM 协议仍拒绝文字：它们没有字体资源及文字后端参数，不能自动开启新能力。新入口尚未通过运行时协议和 TS 暴露给产品；既有 Schema/TS 字段保持不变。后续统一资源合同和宿主适配应直接调用共同编译链，不能在 TS 中另写文字排版。

## 画笔与字形来源

`mo-pptx::source::text::paint` 使用现有颜色 `Session`，在实际绘制表面的主题/颜色映射上下文中求值。当前支持来源 `noFill`、纯色与颜色变换；没有显式 fill 时使用来源 `fontRef` 的颜色，遇到 `phClr` 才延迟查询此引用。缺失或未解析颜色返回诊断，没有默认涂黑。颜色结果保留声明、主题/占位符依赖、工作精度通道和离散采样值。字体引用同时指定主题字体和颜色的依据是固定 ECMA-376 Part 1 §20.1.4.1.17。

可见文字的下划线、删除线、轮廓线、highlight、非空效果和带交互含义的链接声明尚未连接当前静态文字画笔，返回诊断；显式空效果列表可通过。未生效的下划线画笔声明保留给后续装饰计算。字号/字体选择、自然行距等继续使用前述 draft 配置，未因此获得 Office/WPS 认证。

字形与来源 run 通过实际塑形 cluster 连接。每个片段只建立一次排序簇边界，复用簇的画笔和来源映射，避免逐字形扫描整段寻找后继簇。一个簇跨多个 run 且工作颜色一致时，保留全部 run；颜色不同则返回 `GlyphPaintConflict`，不取第一个 run 的颜色，也不随意拆坏组合字符或连字。这类混合画笔簇的完整实现仍在一期范围内。

## 位置与数值边界

文本框位置从形状局部左上角计量，`NativePlacement` 的仿射变换接收相对形状锚点的坐标。文字先减去锚点一次，再应用页面变换；每个轮廓仍留在局部空间以便共享。组合变换产生的平移取整误差单独计入设备误差预算。旋转、翻转及分组的非均匀尺度沿用原生放置引擎。

文字局部误差上界以已选定的断行拓扑和 HarfBuzz 整数输出为参考：每次设计单位缩放至 Q32 至多半个最小单位，自然行高累积至多每行 1.5 个单位；再覆盖垂直锚定、基线、视觉片段前缀、水平对齐、轮廓控制点和文字区域转换。实现保守使用 `4L + 2F + 8 + 4E` 个原始 Q32 单位，其中 L 是框内总行数、F 是一行最多的片段数、E 是区域转换界限。形状变换误差按含字形原点的控制点计算，随后与平移取整、设备路径误差合并，超过请求容差便失败。

这个界限不证明原生公式、字体组件算法、断点选择与目标应用一致，也不度量抗锯齿覆盖误差。独立参考从原始 XML 重建变换，并从 FontTools 读取轮廓；开发时由此核对并修正了“框内左上角直接送入中心坐标变换”导致的整体偏移。没有只凭页面出现了颜色就判定坐标正确。

## 页面预检、资源与失败

页面先核对全部可见形状画笔，再准备所有文本框的来源语义、字体清单和画笔，之后才调用文字组件。内部 `PreparedFrame` 不能从外部 JSON 构造；新入口也不接受调用者修改过的 `SourceTextPagePlan` 作为可信输入。后段缺字体、未实现文字画笔或预检预算超限，会在任何字体/绘制组件调用前失败。

默认最多 128 个文字框、512 个段落、4096 个 run；已准备的来源计算计划累计核算上限为 64 MiB。该计数沿用文字层的保守计划分配核算，不包含整个来源索引、所有继承结果或当前准备帧的峰值，不能当作硬 RSS 上限。来源目录/继承层及单框仍各有已有预算。对象索引按表面建立一次，不逐框线性搜索整页对象。

组件调用、逻辑字体上传、请求字、字形及轮廓命令预算在所有框之间共享。每框只获得剩余额度，不重新领取完整页面预算。完整场景和坐标预算计算完成后才调用 RasterBackend；塑形、原生属性、预算、取消、簇画笔冲突和绘制宿主失败均不返回图像。实例失效仍遵守 Native 进程级/WASM 实例级隔离要求；此处没有实现生产实例池。

## 验证与复现

全仓 461 项 Rust 测试、严格 Clippy、格式、来源编译器 WASM 目标编译及 68 份 Schema 生成检查通过。新增 7 项真实 Native 测试覆盖相邻 run 颜色、来源簇、对象遮挡、旋转/翻转/分组、透明文字、fontRef/phClr、组合字符颜色冲突、整页预检预算、跨框组件预算、取消、绘制宿主失败，以及旧入口继续明确拒绝文字。

独立验证宿主重新执行 HarfBuzz + Skia 测试，在 10 份原创 PPTX 中核对 11 个文字对象、18 个字形的轮廓，验证 102 个页面控制点坐标。参考从 XML 读取形状和颜色，通过 Decimal 重建分组变换、Fraction 计算字形位置、FontTools 独立读取字体轮廓。1154714 个非边缘像素逐字节一致；45286 个距离轮廓边缘两像素以内的像素明确排除，不作为抗锯齿验证。

这些样本使用原创合成字体：字母轮廓是三角形，目的在于可复核地检验计算和遮挡，不是实际字体视觉展示。未核实 Office/WPS 打开、编辑、播放或重新保存的效果。当前 Native 工作进程另重放 207 个旧文字请求及 100 个旧来源页面请求，检查已有响应和有像素的旧页面保持一致；不是新的跨端认证。

```sh
mkdir -p .codex-work/source-text-page
MO_TEXT_PAGE_EVIDENCE_DIR="$PWD/.codex-work/source-text-page/fixtures" cargo test -p mo-harfbuzz-sys --test source_text_page --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo check -p mo-presentation-compile --target wasm32-unknown-unknown --locked
cargo build -p mo-cli -p mo-raster-worker -p mo-text-worker --locked
python3 tools/verification/source-text-page-regressions.py
python3 tools/verification/manifest-layout-regressions.py --output-dir .codex-work/source-text-page
```

`source-text-page-reference.py --executable <测试日志中的 source_text_page-* 可执行文件>` 会重跑真实组件并保存 PPTX、结果、原始 RGBA 与 PNG。参考 Python 需要 NumPy、Pillow、lxml 和既有固定 FontTools 4.61.1；本轮通过已有独立字体工具环境的 `site-packages` 提供 FontTools，版本记入报告。测试中的故障场景采用子进程，不能用重新创建 Rust 包装器恢复已失效的静态组件。

本轮仅增加 Native 测试宿主对已有 `mo-raster` / `mo-skia-sys` 的两个本地开发依赖，无外部依赖版本变化。历史 11 个发行产物保持不变，没有产品性能、峰值内存或完整安装体积结论。证据见[来源文字页面库级记录](../reviews/evidence/2026-09-25-source-text-page-library-verification.json)。此前 10064 批只认证原冻结运行时；新增入口还需 Native/WASM 执行、运行时资源合同、生产宿主和产品链路验证。

下一步将文字页面能力连接到实际双端运行时，并继续实现文字装饰/复杂布局、真实字体质量验证、高级内容与播放、Agent 分发和 Musterwork 接入。一期完整范围与替换验收条件保持不变。
