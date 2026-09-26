# 核心算法、组件和跨语言边界

v0.4 设计基线。这里固定算法责任与默认路线；组件版本和性能结论必须来自 [E0](delivery-plan.md)。基础库解决通用计算，Office 文稿语义和质量规则由 MusterOffice 自己实现。

## 1. 模块边界与依赖

拟议 crate/module：`mo-common`（数值/ID/摘要/诊断）、`mo-resources`（句柄/预算接口）、`mo-presentation-model`、`mo-presentation-edit`、`mo-text`、`mo-geometry`、`mo-charts`、`mo-diagrams`、`mo-math`、`mo-timeline`、`mo-ooxml`、`mo-legacy-ppt`、`mo-render`、`mo-execution`。原生宿主、WASM 绑定、MCP/CLI、Musterwork 适配位于其外。

依赖自下向上，领域模块不依赖宿主。`mo-ooxml` 读写 typed model，`mo-render` 消费编译计划，两者互不通过调用对方生成再截图的方式计算语义。共享 geometry/text 提供计算原语，chart/diagram/math 保留自身对象和规则。包名是设计内名称，发布前再检查名称占用。

| 基础能力 | 默认候选路线 | 不交给组件的责任 / 决策证据 |
| --- | --- | --- |
| Unicode / 文本塑形 | 锁定 Unicode 数据；HarfBuzz 作为塑形基准及优先候选；Rust 同类实现作实测对照 | 段落换行、字体回退、Office autofit、列表和布局由 mo-text；复杂脚本/cross-target/完整体积决定是否采用 |
| 字体读取/轮廓/栅格 | 受控字体解析；FreeType 等成熟基础组件与绘制后端去重评估 | 字体选择、许可、嵌入、family/style 对应与可编辑完整字体策略属于本项目 |
| 绘制 | Skia 的原生/精简 WASM 构建作为优先验证路线；CPU 标准后端＋GPU 合成 | 不使用其段落系统代替领域排版；不默认装入全部 CanvasKit。若完整闭包超预算，按相同 Draw IR 对照轻量路径，不牺牲效果 |
| ZIP/XML/压缩 | 有界流式 Rust 实现、明确 XML namespace/tokenizer、可复用 deflate | OPC、OOXML关系/语义、来源保留、压缩炸弹预算是自己的格式层 |
| 媒体 | 宿主提供解复用/解码/音频设施，统一 PTS/seek 接口；完整离线宿主必须有依法可分发的补足实现 | 时间图、触发、trim/fade/循环/跨页由核心；浏览器 WebCodecs 可用不等于特定 codec 必然可用 |
| 3D | 自有场景与相机语义＋受限模型解析；复用 GPU 绘制基础 | Office 场景映射、原生模型对象、动作时序和确定性测试自己负责 |
| 加密/签名 | 复用经过审查的密码基础库，格式映射独立模块 | 不自创密码算法；密钥/证书/签名权限归宿主 |

组件准入表必须有版本/commit、许可证、传递依赖、Native/WASM 支持、全功能与裁剪配置、SBOM、可重现构建、风险及替代路径。选型失败要回到此表裁定，不在宿主堆相似逻辑或新增商业 Office 黑盒兜底。

构建/许可核对入口：[Skia 构建说明](https://skia.org/docs/user/build/)、[FreeType 许可说明](https://freetype.org/license.html)。Skia 的功能裁剪与系统依赖选项要逐项锁定，不能用“official build”名称推断依赖已包含；组件自己的许可证选择也不能替代整个产品的许可决定。

## 2. 文字排版流水线

1. **解析有效属性。** 按属性族解析主题/母版/版式/占位符/run；保留作者与继承来源。确定方向、语言、书写模式、tab stop、缩进、列表编号、边距和可用域。
2. **itemize 和选字体。** 按 script/language/bidi/样式划分，按 grapheme 与 shaping cluster 保证不拆组合字。字体映射由显式 FontProfile 和覆盖率决定；回退有顺序和诊断，不读宿主隐式系统字体。
3. **形成测量项。** HarfBuzz 等只塑形给定 run，返回 glyph/advance/cluster；处理 contextual shaping、连字、字距、可变轴和垂直替代。缓存键包含字体 bytes/face/axes/features、文本及方向。
4. **寻找合法断点。** Unicode line breaking 提供机会；Office/CJK 禁则、作者软连字符、显式换行、不可分域和段落规则再约束。逐行拟合需要的实际字体度量；在行边界重新塑形需要上下文的 run。
5. **行内与段落布局。** bidi 重排、tab、两端对齐、悬挂、基线/行距、多栏/竖排和图形文本域共同求解；列分配、段落间距不能由 DOM Range 决定。
6. **autofit 与溢出。** preserve 模式绝不自动缩字；作者指定 fitShape/fitText 时在允许参数空间内求解，记录结果与原始属性。超过下限仍溢出则诊断，不删除文本或改变措辞。
7. **原生输出与验证。** Writer 保持逻辑 run/段落及显式换行，软折行留在布局证据。文件重读后核对逻辑内容与字体映射；目标应用拉宽文本框/增字验证自然重排。

字体选项是产品契约：用于导出的字体须允许所需嵌入和编辑，子集只允许用于明确的只读预览资源。若目标应用编辑时需要完整家族/face，发行/文稿携带相应合法资源或取得明确安装前提。无法满足时阻止“无本机字体可编辑”的声明。

Unicode 只给基础算法，不定义 Office 的所有段落行为；参见 [UAX #14](https://www.unicode.org/reports/tr14/) 和 [UAX #9](https://www.unicode.org/reports/tr9/)。塑形边界参考 [HarfBuzz](https://harfbuzz.github.io/what-is-harfbuzz.html)。版本在依赖锁中固定，不能运行时追随网页最新版。

## 3. 几何、绘制与精度

预设几何由公式/调整值解释器求路径与连接点；解释器只提供有限数学指令，不执行 JS/WASM 插件。组采用明确的 child/local/world 变换，裁剪、阴影/反射/模糊扩大视觉边界，布局框与可见边界独立。图片裁剪保存源坐标，解码采样和滤波按 render profile 固定。

Draw IR 包含路径/笔刷/字形实例/图像/clip/group/effect/layer/blend 等受限指令及资源引用，文本已给出 glyph 位置。CPU 标准渲染接收同一 IR；GPU 将静态层、可变属性及媒体纹理组成 retained graph，只更新脏区域/属性。字体、路径与效果缓存共同受预算管理，不能让 GPU 驱动隐式持有未记账的文稿。

实时后端禁止每帧重塑全文、整页重编码 PNG 或回读 GPU 到 CPU 再上传。GPU 丢失后可以重建资源并暂停/恢复；仅当同质量 CPU 路线满足当前档案时允许切换，否则报告宿主能力不足，不暗中降低 DPI/删效果。

内存中的色彩模型和 premultiplied alpha 明确；图片 ICC 输入解析/转换与输出 sRGB 基线固定。广色域作为声明能力单独验证。CPU/GPU 输出不要求跨设备位级一致，语义、布局和事件轨迹要求一致，像素按 [验收档案](acceptance-profiles.md)判断。

## 4. 表格、图表和数据

表格先解析网格/跨度，计算列宽和行高约束，调用同一文本引擎布局单元格，再计算边框归属和绘制；合并区域只拥有一个主内容节点，编辑操作显式处理其余内容。布局迭代限定收敛准则和次数，失败返回约束集合，不能死循环调字号。

图表依序处理数据语义 → 类型与系列/轴验证 → 值域/尺度/刻度 → plot rect → 图元 → 标签/图例/数据表布局。标签避让固定候选顺序与代价函数，优先保持数据可读，不能静默删标签。坐标轴/日期/缺失值/负数/百分比各自有规则，3D 图表与 3D 模型不是同一领域。

图表数据具有单一事实源，嵌入工作簿是其 OOXML 投影。只实现所需的工作簿单元格、样式、范围和公式引用，不把电子表格完整计算器偷偷塞进演示核心；外部复杂公式的缓存/依赖必须保留并报告，改变影响数据时需对应计算能力或明确输入已计算值。chart cache、工作簿 cells、渲染采样和文件重读值四路比较，防止“看着对、点编辑数据却不同”。

## 5. SmartArt、数学和专业成稿

SmartArt 按固定顺序执行：校验 data model → 展开 layout 的 forEach/choose/条件节点 → 创建 presentation/layout instances → 解析变量/约束/规则 → 运行 algorithm node → 文字测量反馈 → 收敛布局 → 生成 drawing 和动画子目标映射。算法至少覆盖规范中的 composite/linear/cycle/hierarchy/pyramid/snake/space/connector/text 等注册种类，完整性由注册表检查。

约束带优先级和原生来源；硬约束不可冲突，软约束按确定顺序优化。布局反馈迭代次数/残差阈值纳入规则版本，非收敛返回具体节点与冲突，不能换成随意摆放。writer 同时输出 data/layout/style/colors 及一致 drawing；目标应用增节点、升降级、换布局和播放分步动画验证其原生性。

数学以 OMML 对应的 typed tree 构造 boxes：token、fraction、radical、script、n-ary、delimiter、matrix、equation array 等。使用 [OpenType MATH](https://learn.microsoft.com/en-us/typography/opentype/spec/math) 的常量、伸展字形和 assembly；处理脚标间隙、运算符上下限、嵌套字号、行内基线及明确断点。公式保留树形和样式写回原生数学格式，不把最终 glyph path 当编辑模型。

专业设计质量还需要模板与约束库：网格/间距、字号层级、色彩与对比、图文比例、数据图形选择、品牌主题和溢出诊断。内核提供确定的 layout intent/约束求解和证据；Agent 决定内容与方案，模板许可明确。盲评不足时改进模板/约束或 Agent 工作流，不能宣称“格式引擎正确就必然等同成熟产品成稿”。

## 6. 时间、媒体、Morph 与 3D

时间图先展开 seq/par/触发关系，保持原 native ID 与效果参数；对同目标属性，组合方式、优先序、additive/accumulate/fill 由格式规则明确，不能以遍历顺序碰巧覆盖。事件有单调序列和确定 tie-break；相同时间的点击/书签/页切换顺序进入日志。

`evaluate(checkpoint, orderedEvents, rationalTime)` 纯求值状态；checkpoint 覆盖已触发状态、repeat cursor、页返回栈和媒体控制意图。seek 先找兼容 checkpoint，再重放必要事件；seek 不能捏造过去的用户点击。time=0 编辑预览、初始放映和最终效果态必须使用不同采样模式。

宿主解复用/解码按 PTS 提供帧和音频样本，音频时钟优先作播放主时钟；核心计算显示区间与 seek/flush/rate 等意图。队列显式上限、迟帧丢弃规则、音画不同步诊断和资源释放共同验证；可丢过期显示帧以追时，但不能丢语义事件、跳过必需动画或把音频加速到错误时长。

Morph 用稳定对象映射、格式允许的命名匹配和确定性匹配规则建立两页对应，分别插值位置、形状/文本和视觉属性；对象增删有明确出现消失规则。必须与原生 Morph 写回和目标应用行为比对，不能用普通淡入淡出假装 Morph。

3D 保留模型 bytes、场景、相机、灯光、材质和模型内动画语义；模型动作与文稿时间图分层。对导入模型进行数量/纹理/拓扑预算检查，CPU 标准采样帧与实时 GPU 采用同一场景参数。墨迹同时保留笔画与时间数据，重放不等于每笔一张位图。

媒体容器与 codec 是独立能力。WebCodecs 不保证某种编码在每个浏览器存在，参见[标准](https://www.w3.org/TR/webcodecs/)；宿主 capability 决定可执行性，产品的完整支持档案必须提供补足或可验证的系统前提。所有下载/编解码模块体积计入完整成本。

## 7. 原生格式与来源写回

Reader：有界包目录 → 校验内容类型/关系/namespace → 按需解析 typed 模型 → 建立 source binding/保留区 → 能力/资源报告。拒绝路径穿越、重复冲突 entries、递归关系失控、外部实体和超预算解压；拒绝不意味着删除不理解的合法内容。

Writer：从 changeSet 算受影响关系闭包 → 对共享部件写时复制 → known-field overlay → 保留未触及片段/属性/关系 → 重建必要 part/relationships/content types → 流式写暂存 ZIP → seal → 实际 bytes 重读。未知节点依赖已删除对象时产生 preservation conflict，由显式操作决定，不能留下悬挂引用。

无修改 roundtrip 以原包 bytes 直接复用为最强路径；修改后保证未触及部件内容保持，ZIP entry 的重压缩/顺序变化单独说明，不虚称整个文件 hash 不变。数字签名只对原涵盖字节成立；重签归宿主。

原生文件标准依据 [ECMA-376](https://ecma-international.org/publications-and-standards/standards/ecma-376/) 与 [MS-PPTX](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-pptx/efd8bb2d-d888-4e2e-af25-cad476730c9f)。规范符合与 Office/WPS 实际兼容分别验证。

旧二进制输入另按 [MS-PPT](https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-ppt/6be79dde-33c1-4c1b-8ccc-4b2301c08662) 建独立解析器；加密/签名格式依据 [MS-OFFCRYPTO](https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-offcrypto/4c132f1a-f4a0-42cc-9046-15af9afa9b9e)。它们不是 OOXML Reader 的“改扩展名”路径，格式支持与宿主密钥/外部执行权限必须分别实现。

## 8. ABI、隔离与优化纪律

C/C++ 边界只传 typed POD、长度明确的借用 bytes 和 opaque handle；统一 create/release，分配者释放；显式错误码，无 C++ exception/Rust panic 越界，无跨语言隐式 allocator。回调次数、拷贝字节和峰值在 E0 测量，优先批量塑形/绘制，禁止每 glyph 跨 FFI。

WASM 使用显式线性内存视图/句柄，不依赖宿主读取 Rust 内部布局；增长内存后 TS 视图重建，释放/取消后旧句柄带 generation 拒绝。线程/SIMD/GPU 是宿主能力，不能作为未声明的隐含前提。

优化顺序：删冗余表示和不必要拷贝 → 按需解析/解码 → 合理缓存/增量 → 批处理/并行 → SIMD/GPU → 链接与体积裁剪。每一步保留相同质量断言。LTO/opt-level/裁剪配置通过测量选择；语言或 `-Oz` 本身不是性能与小体积保证。

## 作者变换实现约束

作者页面坐标与 Draw IR 的普通仿射节点保持职责分离。当前实现按子对象角度区间选择父组缩放轴、按父组翻转奇偶决定旋转方向，并用求值后的组空间定位子对象，不能把普通矩阵连乘直接当成作者语义。角度/有理比例的误差上界须加入设备坐标预算。此 profile 已由 48 个 WPS 实际非对称形状进行局部几何核对，仍属 draft；解释性附录推导出的首版曾产生明显外部差异，不能以数学精度或跨端一致代替互操作。完整 Office/WPS 编辑往返尚未验收，见[作者页面坐标](../../implementation/page-placement.md)与[组变换兼容核对](../../implementation/group-placement-compatibility.md)。

新建 PPTX 的静态 `a:xfrm` 旋转采用一圈内等价表示；作者存储与来源保留编辑保持原值，不将此规则用于动画增量和圈数。静态求值与 writer 共用角度单位和归一化定义，不能靠修改调用方文稿或目标产品补偿，验证见[静态旋转导出](../../implementation/static-rotation-export.md)。
