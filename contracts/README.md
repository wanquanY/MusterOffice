# 运行时合同

`asset-info`、`asset-binding`、`upload-request`、`upload-info` 增加资源声明、接收状态和明确的文档绑定；二进制经独立通道传输。`host-request/response` 接通元数据命令，新增资源错误码也进入公共任务类型。当前同源合同共 104 份，详见[资源宿主](../docs/implementation/resource-host.md)。`verification=bytesSha256` 不代表格式/解码或目标应用验证。

`host-request`、`host-response`、`operation-request`、`operation-job` 定义持久文档操作合同，统一创建/编辑及任务和文档读取结果。当前 `presentations-author-model-v01-draft` 仅对应已实现的作者模型操作；路径、身份和权限不属于 JSON 参数。行为及未完成范围见[文档操作宿主](../docs/implementation/operation-host.md)。该阶段生成合同共 100 份，其中此前内核合同 96 份保持不变。

`generated/` 的运行时合同由 Rust 类型及 `mo-contract-codegen` 生成，禁止手改。当前版本是 `0.1-draft`，覆盖文档计算子集、OPC 包检查、新建 PPTX 导出、来源读取/文本修改/颜色查询、字体与文本计算、几何/画笔求值及受限页面编译绘制；完整一期功能仍按 [v0.4 实施规格](../docs/design/implementation/README.md)扩展。

`pptx-playback-session-request` / `pptx-playback-session-response` 定义来源 prepare、inspect、render、advance、dispose。来源与字体字节只在 prepare 传入，render 只接收时间/事件与栅格组件；所有权、实际工作计数和宿主边界见[来源资源播放会话](../docs/implementation/source-playback-sessions.md)。

`pptx-playback-page-request` / `pptx-playback-raster-response` 将来源包绑定、slide 时间求值与资源页面绘制连接；原生 ID 由部件限定，快照不经作者模型重建。当前范围与成本见[来源动画](../docs/implementation/source-playback.md)。

`playback-session-request` / `playback-session-response` 定义 prepare、inspect、compile、render、advance、dispose 的同源命令，使用显式绑定和有界所有权；生命周期与当前限制见[播放采样会话](../docs/implementation/playback-sessions.md)。

`playback-page-request`、`playback-compile-response` 与 `playback-raster-response` 绑定快照求值和共享作者页面绘制，分数角度、事件上下文、放置/像素摘要及诊断见[动画共享绘制](../docs/implementation/playback-rendering.md)。逐请求入口保持可用，可复用 owner 见会话合同；完整宿主播放控制和媒体合成继续实施。

时间模型新增 `musteroffice.timeline/0.2-draft` 的显式 `tree`：并行/顺序容器、作用域时钟和 hold；旧 `0.1-draft` 序列化省略 tree。帧可携带 `containers` 和 `suppressed` 状态，规则与兼容边界见[分层时间树](../docs/implementation/timing-trees.md)。该阶段同源生成合同为 96 份。

行为叶子可携带 `timeTransform`，为两个时间模型版本提供有限变速、倒放、自动往返及加减速；缺少声明时保持旧计算和序列化。字段、精度、原生映射及未完成范围见[行为时间变换](../docs/implementation/time-transforms.md)。容器尚不接收该声明。

`repeatMilli` 保留原整数表示，并新增显式 `"indefinite"`；可选 `repeatDuration` 为有理时间值或 `"indefinite"`。无限与未知结束不通过极大整数或零表示。时长求值、反向起点与原生默认值边界见[重复次数与时长](../docs/implementation/repeat-bounds.md)。

行为叶子的可选 `endConditions` 数组与 `start` 共用 `TimeCondition`：at、after、click。空数组省略，Rust 保留 `StartCondition` 别名。条件取最早有效结束，原子依赖包括结束引用；父时钟、点击序号、倒放及尚未支持的区间规则见[行为结束条件](../docs/implementation/end-conditions.md)。

`TimingContainer` 现在也接收 `endConditions`，与叶子共用准入和原生条件列表；因果事件、子树终止、同刻完成与工作预算见[容器生命周期](../docs/implementation/container-lifecycle.md)。该阶段合同为 96 份，完整容器重复／重启尚待实现。

`timeline-evaluate-request/response` 绑定快照、页面、播放会话、版本、generation 与事件历史，返回精确旋转属性状态；`pptx-timing-query/response` 按源包和页面查询已实现的原生时间树。作者文档的可选 `timelines`、事务 `setTimeline` 和回执 `changedTimelines` 共用这些类型。当前不代表完整播放或对任意来源时间树的编辑，详见[原生旋转时间图](../docs/implementation/timeline-rotation.md)。

`pptx-resource-page-request/raster-response` 定义实际 PPTX 来源页面的图片解码、形状/文字绘制和结果诊断。显式输入来源策略、采样及可选字体清单；像素与字体通过独立二进制通道传递，详见[来源资源页面](../docs/implementation/source-resource-page.md)。`rendered` 仅表示所选草案 profile 和本次页面通过计算，不表示完整功能或产品替换验收。

`pptx-image-query/response` 定义来源图片继承、显式嵌入/外链选择和编码资源清单。`inspected` 可含未解析/外链目标，不能表示整页可渲染；字节通过独立 bundle 传递，详见[来源图片](../docs/implementation/source-images.md)。

`image-scene-request/response` 把显式已解码图片连接到共享场景编译与绘制；资源摘要和图片误差独立报告，字节不放入 JSON，见[场景图片](../docs/implementation/image-scene.md)。

图片画笔可选 `sourceDomain` 采用 Q32 **源像素**矩形；省略保留原行为，不能当作世界 EMU 或原生 PPTX `srcRect` 直接传入。V6 编译、精度预算及边缘规则见[图片源区域](../docs/implementation/image-domain.md)。

`image-decode-request/response` 将源摘要绑定到实际编码字节，返回已应用方向及色彩转换的 RGBA8 元数据；像素另走二进制通道，见[编码图片解码](../docs/implementation/image-codec.md)。响应中的 `resolution` 保留 PNG/JFIF/EXIF 原始分辨率、来源偏移及严格一致性的精确 EMU/像素结果；来源声明保持编码轴、派生尺寸采用归一化轴，格式缺省值和冲突规则见[图片分辨率](../docs/implementation/image-resolution.md)。

这些运行合同与 [设计阶段交付信封](../docs/contracts/README.md)职责不同。当前 `initialize/validate/prepare` 是开发计算入口，不是已完成的 MCP 导出服务，也不包含 Artifact 提交。`prepared` 结果必须由宿主校验授权及 revision 来源后，以 CAS 和幂等收据的同一持久事务提交。

```sh
cargo run -p mo-contract-codegen --locked -- write contracts/generated
cargo run -p mo-contract-codegen --locked -- check contracts/generated
python3 tools/verification/contracts.py
```

Schema 检查形状；Rust 语义验证器另查引用、所有权、循环、资源类别和数值范围。EMU/ticks 的 int64 边界在 Schema 中记录为 `x-integer-minimum/maximum`，通用 JSON Schema 工具不会自动执行这两个扩展；必须经过 Rust 反序列化。JSON 输入先拒绝重复键，不能依赖普通 JSON 对象的最后一个值覆盖行为。

`package-inspection.schema.json` 是二进制检查的响应合同。部件名/ContentType/关系由 OPC 层校验后输出；包与部件长度使用 `ByteLength` 的 uint64 十进制字符串，其 Schema 扩展范围同样由 Rust 校验。`containsSignatures` 只表示发现签名相关部件或关系，不代表已经验证签名。

`pptx-export-request.schema.json` 包含文稿、显式默认主题/文本设置以及资源 ID 到独立二进制 bundle 的范围绑定。偏移和长度也使用 `ByteLength`，禁止路径、URL 或 JSON base64 隐式取资源。Native/WASM 调用者提供同一请求和二进制字节；核心检查范围、重复绑定和实际资源摘要。成功返回实际 PPTX 字节，失败不返回候选文件。当前只开放开发桥接，128 MiB bundle/32 MiB JSON 上限不是生产资源协议；字体策略暂只有显式 `referenceOnly`，不表示字体已解析或嵌入。详细边界见 [PPTX 导出实现](../docs/implementation/pptx-export.md)。

当前规范摘要固定 UTF-8、UTF-8 键排序、无多余空白、无 Unicode 归一化，数值只接受安全整数；EMU/ticks 用十进制字符串。它是 `MusterOffice canonical JSON v1`，不宣称 RFC 8785 JCS。摘要有用途域分隔，不是权限凭据。

`pptx-source-response.schema.json` 描述原生来源的部分索引或类型化错误，未解析内容由 notices/barriers 明示。`pptx-text-edits.schema.json` 要求源包摘要、部件作用域对象/段落/run、旧文本和替换文本；内核重新建立绑定，不信任调用方传入的 XML 位置。生产全局身份、完整 SourceBinding、跨 run 操作及 MCP 信封尚待实现，见[来源编辑实现](../docs/implementation/pptx-source.md)。

来源合同中的 `themes` 保存共享主题声明，`themeSelection` 分别绑定颜色/字体/格式方案；`colorMapping` 保留作者映射，`resolvedColorMapping` 引用有效显式表。物理元素序号仅在对应不可变来源版本内有效，不是外部可提交的写入偏移。声明与来源绑定不等于已经完成颜色求值、字体塑形或渲染，见[主题声明](../docs/implementation/source-themes.md)和[颜色映射](../docs/implementation/source-color-mapping.md)。

`pptx-color-query.schema.json` / `pptx-color-response.schema.json` 为独立的按需颜色计算入口，要求源摘要、表面部件、显式数值 profile 和系统/占位色上下文。结果包含采样值、依赖来源与未解析原因；查询不改写上述来源声明。当前 profile 为 draft，目标应用颜色差异仍需验证，见[颜色计算](../docs/implementation/source-colors.md)。

`font-request/response` 绑定字体摘要与 face，查询真实目录/元数据和 cmap。`shape-request/response` 在同一字体上执行有上下文的 run 批次，范围使用全文 Unicode scalar，轴坐标使用精确 16.16 输入并报告有效 binary32。成功输出含缺字诊断，失败不返回部分 run。它们都不读取路径或隐式系统字体；隔离宿主和当前边界见[文本塑形](../docs/implementation/text-shaping.md)。

`text-analysis-request/response` 使用固定 Unicode 18.0.0 数据，返回扩展字簇的 scalar/UTF-8/UTF-16 边界及受限字符属性查询。它不承诺字体选择、bidi 或断行，见[Unicode 实现](../docs/implementation/unicode-text.md)。

`cascade-request/response` 描述显式字体 bundle 与原子 item 的候选求值。`evaluated` 可含 `unresolved`，`selected` 只证明当前字形可用性 profile；不等于完整字体、段落或视觉验收。字体摘要/face、uint64 范围、候选轴、字簇边界与累计预算仍由 Rust 语义验证。见[字体资源复用与候选选择](../docs/implementation/font-cascade.md)。

`bidi-analysis-request/response` 对显式单段文本计算双向层级，对显式字簇对齐行尾执行 L1/L2，并查询固定 Unicode 18 分类/括号/镜像属性。X9 控制项以 `null` 保留原 scalar 位置，作者文本不删除、不重排。`visualOrder` 是 L2 的 scalar 索引映射，不能当成塑形输入或最终字形绘制次序。它不自动断行，不代表完整 L3/L4 或 Office/WPS 排版验收，见[双向文字实现](../docs/implementation/bidi.md)。

`itemization-request/response` 从逻辑段落、方向和有效样式跨度生成脚本/层级片段及控制项。`paragraph-shape-request/response` 进一步绑定显式字体 bundle 与样式，合并等价样式并调用隔离塑形；`shapedItemIndices` 保留文字项与字形结果的对应。段落响应现为 `unicode18-auto-items-hb14.5-mixed-font-reshape-v2`，以 `fallback` 替换先前开发版的 `cascade` 字段，包含字体片段、完整探测及重塑证据；独立原子 `cascade` 合同未改。作者文本不重排，`evaluated` 可含未解析范围。完整边界见[自动分段](../docs/implementation/text-itemization.md)及[混合字体回退](../docs/implementation/font-fallback.md)。

`line-break-request/response` 求固定 Unicode 18 / UAX #14 r57 默认机会，返回 `allowed` / `mandatory`、原始三个坐标系和 SA 分类位置；原始文字不改写。空文本仅有零结束坐标，没有断行机会。它不提供词典分词、最终行宽、Office/CJK 定制、连字符绘制或字形裁剪许可，见[Unicode 断行](../docs/implementation/line-breaking.md)。

`font-metrics-request/response` 绑定字体、face、实例轴和最多 28 个度量名；结果保留显式可用性及 1/64 设计单位，`null` 不等于真实零。固定 HarfBuzz profile 不合成缺失值，也不是 Office/WPS 行高选择合同。见[字体实例度量](../docs/implementation/font-instance-metrics.md)。

`line-shape-request/response` 使用既有段落、字体 bundle 和显式计算行尾，重新求行内上下文塑形并返回原段落坐标。它不把软折行写入作者文本，也不自动决定合法断点、行宽或 glyph 绘制次序。见[行上下文与重塑](../docs/implementation/line-shaping.md)。

`line-geometry-request/response` 将显式行计划、对应字号/偏移、strut 和行距转换为完整水平字形坐标；EMU 使用 int64 字符串，缺少布局前提时输出明确问题和 null layout。它不自动换行，不返回 ink bounds。见[显式行几何](../docs/implementation/line-geometry.md)。

`paragraph-layout-request/response` 消费显式宽度和溢出策略，输出默认断行分析、实际行尾、完整几何或未决前提。行尾为派生数据，终端显式换行可以生成同坐标末尾空行；当前 profile 保留所有空格 advance。见[自动段落布局](../docs/implementation/paragraph-layout.md)。

`font-outlines-request/response` 将显式 glyph ID 批次连接到同一字体实例的二次/三次曲线。单色可用性、空路径、轴与双预算明确分开；结果不代表彩色绘制或所有字形已成功。见[字体实例轮廓](../docs/implementation/font-outlines.md)。

`paragraph-paths-request/response` 将段落布局连接到共享单色曲线和实际 Q32 原点；`boundsTolerance` 是原始 i128 Q32 EMU 整数字符串，语义范围由 Rust 检查。scene 为 null 时不发布部分绘制，几何边界不等于像素/效果边界，详见[段落路径](../docs/implementation/paragraph-paths.md)。

上述行几何、段落布局和段落路径的 `GeometryStyle.baselineShift` 接受既有整数 EMU 字符串，或精确 `{ "q32": "2147483648" }`（示例为 0.5 EMU）。未知字段、非规范字符串、越过 i128 范围和重复键均拒绝，等值表示按数值参与样式归并。原生百分比到该合同的转换及误差规则见[基线偏移](../docs/implementation/native-baseline.md)。

同三份请求的 `LineSpacing` 新增 `{ "kind": "styleMaximum", "heights": [...] }`，每个几何样式对应一个非负 Q32 EMU 高度；文字行按实际文字项取最大值，无文字项时使用 strut。长度、非负及 i128 范围约束由 Rust 语义校验，既有行距形式不变。来源映射和零值规则见[段落间距](../docs/implementation/paragraph-spacing.md)。

同三份请求的 `GeometryStyle.clusterSpacing` 是可省略的 signed Q32 EMU 字符串，默认零且零不输出，允许负数。间距在每个塑形字簇的视觉尾部应用一次，包含行末。低层接口不替调用者选择字体 features，来源策略和统一笔位见[字符间距](../docs/implementation/character-spacing.md)。

`pptx-page-request` / `pptx-page-compile-response` / `pptx-page-raster-response` 绑定真实源包和显式静态页面 profile，返回带来源的场景或实际像素元数据。未支持可见内容使整页失败，不输出部分像素。源响应新增可见性/文字体/视觉缺口，填充颜色结果可选 `contextOverride` 表示最终页面背景跳转。开发接口与未关闭项见[来源页面编译](../docs/implementation/source-page.md)。

`SourceIndex.text` / `SourceSurface.text` 为可缺省的原生文本声明目录，以部件内物理序号记录根、父子结构、类型化属性、画笔/效果和未解释位置。文本字符串仍在已有段落/run 投影中。缺省不等于有效默认值，字体声明不等于已绑定字体资源；详见[来源文本](../docs/implementation/source-text.md)。大型 TS 合同按完整声明生成分文件，原入口保持类型重新导出；不手工维护第二份合同。

`pptx-text-body-query` / `pptx-text-body-response` 绑定来源摘要、表面、对象列表及 draft 继承配置，输出 19 个文本框属性、逐项来源和 autofit 选择，或明确未决结果。`SourceThemePart.textDefaults` 保存主题根的文本默认项。值已计算不表示已经完成文字缩放、段落布局或绘制，详见[文本框继承](../docs/implementation/text-body-styles.md)。

`image-raster-request/response` 将独立 RGBA8 bundle、图像清单与路径绘制绑定；Native Worker / Rust WASM 使用同一入口。资源摘要、预乘 Alpha、矩阵条件和范围由 Rust/组件共同验证；PNG/JPEG 现由独立解码入口提供像素，本请求直接消费已归一化资源；完整 PPTX 图片声明编译仍需连接，见[图片资源与绘制](../docs/implementation/image-raster.md)。

作者与来源播放会话增加 `inspectTiming` → `timingInspected`，严格校验 owner 绑定并返回精确 uint64 字符串计数；原有成功响应不变。复用边界及计数语义见[保留式时间区间复用](../docs/implementation/retained-timing.md)。
