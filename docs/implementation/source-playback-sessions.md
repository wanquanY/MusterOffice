# 来源资源播放会话

状态：已实现来源资源的显式所有权、准备与逐帧采样；尚未完成完整演示播放宿主或 Musterwork 替换验收。沿用 Rust 主体、TS 薄接入和精选 C/C++ 组件，不改变办公语义与原生 PPTX 的方向。

## 解决的问题

此前来源动画公共入口每次重新打开/index PPTX、解析时间树、验证字体、解码图片并塑形文字。[来源动画](source-playback.md)的逐请求接口保留，新会话将不变工作移至准备阶段。

`SourcePlaybackPlan::retain` 生成 `RetainedSourcePlaybackPlan`。其中 `ResourcePagePlan` 持有来源索引、固定页面请求、解码像素、局部图片布局以及紧凑文字绘图数据。它不借用原始 ZIP、字体字节、字体 manifest 或组件实例，不使用自引用结构、unsafe、全局句柄或进程隐式字体。输入字节和准备组件可在 prepare 返回后释放。

文字只保留可见字形/装饰的局部路径、原点、颜色和误差界；同一对象/段落内相同路径复用存储，完整塑形、排版中间结果不长期保存。图片像素通过 `PreparedImages::owned` 移动接管，与借用版本共用摘要、尺寸和预乘 alpha 验证；不在采样时重复哈希、解码或重新拼接像素包。

## 每帧仍须做的计算

时间求值生成部件限定的精确角度；来源声明不改写。页面引擎重新计算组合祖先变换、扇区缩放、反射、几何、公差分配和渐变画笔。图片局部布局的尺寸不变，世界画笔与拉伸裁剪根据本帧放置重编译。文字逐路径调用与单次渲染完全相同的局部放置及误差认证，不能复用已经烘焙的世界矩阵或上一帧精度结论。

准备阶段会编译并认证**来源声明的基础姿态**，不运行栅格组件。基础姿态失败时没有可用 owner，即使某个动画时刻可能可渲染，也不能绕过准备。采样失败不污染不可变资源；栅格组件失效时宿主须丢弃该组件。WASM 可用新的合规组件再次采样，Native 隔离进程遇到组件失效会终止，宿主重新建立 owner。

当前固定的是页面、viewport、颜色上下文、字体 manifest、图片选择/采样策略、来源字节身份及实现 profile。改动这些参数需新 owner。只更新 generation 不重建资源，不保存旧事件历史、时钟或输出帧。

## 接口与所有权

Rust `mo_kernel_api::PptxPlaybackSession` 和生成的 `pptx-playback-session-request/response` 提供：

| 操作 | 输入 | 结果 |
|---|---|---|
| prepare | 固定页面请求、binding、源包、字体 bundle、解码/文字组件 | prepared，含资源准备信息 |
| inspect | 完整 binding | 当前 planId、binding、准备信息 |
| render | binding、精确时间、显式事件历史、栅格组件 | 元数据及独立像素 |
| advance | 当前 binding、严格递增 generation | 同一计划的新 binding |
| dispose | 当前 binding | 释放计划；重复相同 dispose 幂等 |

作者和来源会话共用 `playback_owner` 的 Empty/Ready/Disposed 状态管理。准备原子提交；同一 owner 不接受重新准备；Disposed 不复活。所有操作核对 session/revision/generation，旧请求不能采样、推进或释放新代次。来源 revision 是包 SHA-256，产品授权与历史版本来源仍由宿主持有。

`planId` 的摘要覆盖完整页面请求（含字体范围/摘要/实例）、来源身份、准备结果及实现 profile；不包含会话名称与 generation。未引用的字体 bundle 填充字节不改变绘制身份。此 ID 不是授权凭证、跨版本缓存文件格式或完整生产组件依赖指纹。

WASM `new PptxPlaybackSession()` 提供 `prepare(request, source, fonts, decoder, shaping)`、`command(request)`、`render(request, raster)` 和生成的 `free()`。只有 prepare 需要来源、字体与准备组件。运行于宿主管理的 worker；宿主负责队列、超时、取消/终止、结果 generation 核对及显示。

Native `mo-raster-worker --pptx-playback-session` 一进程一 owner。每条请求帧依次为三个小端 u32 长度（JSON/source/fonts），然后相应字节；响应为两个小端 u32 长度（JSON/pixels），随后内容。非 prepare 命令的 source/font 长度必须为零。截断、无效 UTF-8 或超限传输终止进程；正常的语义拒绝返回错误且没有像素。现有一次性 CLI 仍可用于导出帧，本阶段没有伪装成完整播放控制器。

## 资源和工作计数

`preparation` 报告实际准备阶段的文字组件调用、字体上传、请求 word 数、字形/路径数量、解码图片数量、解码像素长度、编码资源长度、一次 gather 复制量和资源摘要。

渲染响应中的 `textWork.componentCalls/fontUploadBytes/requestWords` 及 `gatherCopyBytes` 为该采样的实际值零；glyph/path 数保留其逻辑内容数量。`decodedImages` 是所引用资源的元数据，`encodedBytes` 是准入资源规模，二者不是本帧解码工作计数。

紧凑文字 path/draw 逻辑元素采用 64 MiB 独立上限，图片包继续采用 64 MiB 解码总量及既有准入限制；这些不等于总内存承诺。源索引、容器容量、临时排版、准备时的旧/新数据共存、动态场景、WASM 桥接、栅格组件和输出像素另占资源。每帧仍会向栅格组件传输像素资源，尚无 GPU/组件内部纹理常驻 ABI；没有声称整条链路零复制。

## 验证与性能

结果由[本阶段证据](../reviews/evidence/2026-09-26-source-session-verification.json)绑定源码、测试、构建、实际输入/结果、旧记录和测量条件。原生/WASM 相同不代替 Office/WPS 验收。

新增混合组合包含文字、两处共享图片填充、非等比组合缩放、水平翻转及父子独立旋转。精确角度由 Python Fraction 计算，整数时刻另改写原生 XML 变换并移除 timing 作为像素对照；分数时刻与既有单次公共入口对照。

最终验证：683 项 Rust 测试（新增 5 项）、严格 Clippy/rustfmt、96 份 Schema 和同源 TS 合同通过。13 个来源 owner 的 135 对 Native/WASM 调用中，52 个成功采样均没有解码/塑形组件调用；37 个旧动画帧与旧元数据（扣除明示的实际工作计数差异）保持一致。7 个混合组合帧通过独立有理数角度和单次入口核对，其中 3 个原生 XML 静态对照的 360,000 个像素完全一致；24 个新增引用 XML 部件通过 XSD。

旧功能回放另外包含 355 次静态调用、246 次时间调用、111 次作者会话命令，以及来源单次入口与其对照的 70 次调用；保持既有结果。失败恢复、重复/过期命令、输入释放、取消原子性、无效传输、CLI 拒绝覆盖及失败不发布均有记录。实际返回像素的检查图使用自有合成字体（三角形字形）和图案，不代表真实办公字体视觉验收。

以下为 Apple M4 Max、arm64、Darwin 25.2.0、Node 23.5.0 上的 WASM 公共入口中位数。页面 400×300，模块已热身；逐帧测试预热 5 次、交错次序测量 20 次，独立准备测量预热 1 次、取 5 次。测量时本任务无并行构建/测试，其他系统负载未控制。包含 JSON/时间求值、动态编译、真实栅格、元数据及像素传递；两种模式逐次核对像素相同。

| 自有样例 | 准备一次 | 每帧重新准备 | 会话采样 | 采样耗时降低 |
|---|---:|---:|---:|---:|
| 图片与文字 | 2.392 ms | 4.395 ms | 2.731 ms | 37.9% |
| 母版/版式/页面图片 | 1.531 ms | 5.488 ms | 4.015 ms | 26.8% |
| 圆形渐变 | 1.409 ms | 18.595 ms | 16.897 ms | 9.1% |
| 原生文字 | 1.733 ms | 3.922 ms | 2.098 ms | 46.5% |

准备成本单列，不能把它从端到端首帧成本中隐去。圆形渐变仍约 16.9 ms，历史渐变性能回退仍开放；此结果不是实时 60 FPS、冷启动、RSS 或全产品性能承诺。

Rust WASM 原始模块为 7,588,927 字节，比上一阶段增加 241,995 字节；C++ WASM 仍为 2,370,691 字节。依赖锁、第三方组件及 TS 栅格适配未变。这些是模块大小，不能直接推算 Musterwork 安装包体积。

## 剩余目标

只实现此前严格旋转时间 profile 与已有来源页面可见内容范围。完整时间容器、其他效果/子对象、转场/Morph、媒体同步、SmartArt/公式等高级可编辑内容、真实字体语料和目标应用验收继续推进。动态几何和最终合成仍有优化空间；正式 SDK/MCP/Skill/Plugin 分发、Musterwork Artifact/历史迁移及完整替换门槛尚未完成。
