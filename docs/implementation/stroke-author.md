# 作者描边、页面编译与原生 PPTX

2026-09-24：显式纯色描边已从作者模型接入页面编译、Native/WASM 实际像素和原生 PPTX 导出。支持三种端点、圆角/斜角/尖角连接、独立线宽、主题色和透明度；沿既定 Rust/TS/精选 C++ 路线推进。尖角在目标软件中仍有可见差异，本阶段不关闭保真或 Musterwork 替换门禁。

本轮[封存证据](../reviews/evidence/2026-09-24-stroke-author-verification.json)包含实际产物、全部回归、数值参考、原生文件检查和外部差异。此前[设备描边](stroke-raster.md)负责求值后画笔与 ABI 2；本轮连接作者语义和格式出口。

## 声明、求值与文件语义

作者 `Stroke::Solid` 增加可选 `cap` 和 `join`。端点为 `flat/round/square`，连接为 `round/bevel/miter`；尖角 `limit` 使用整数 1/1000 百分比单位，`400000` 表示 400%。[Microsoft 的 Office 实现说明](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/7bd817fb-79cf-4789-b733-4a29f032f5b7)将该限制解释为内外角间最大距离相对于线宽的百分比。该定义不充分说明各应用超限后的裁切方式，不能据此推定所有后端完全等价。

缺省声明保持缺省：不在反序列化时填入端点或连接，不改动作者数据和既有规范摘要。原生 PPTX 只写明确声明的 `cap="flat/rnd/sq"` 与 `a:round/a:bevel/a:miter`，连接元素位于线条填充之后；`lim` 不重写为某个应用的近似值。原生线宽和百分比范围分别检查。现有无新增参数的导出与来源保留回归保持原字节。

页面绘制要求端点和连接都已显式提供。任一缺失返回 `unresolvedStrokeParameters`，完整样式/默认值解析继续实现；不把一次 LibreOffice 缺省观察当作所有 Office 的默认规则。无描边是独立 `none` 声明，宽度 0 是当前后端的设备 hairline。无参数枚举采用空结构变体，使 Serde 真正拒绝 `none` 或 `round` 上的无效额外参数。

`page_paint.rs` 负责作者颜色和描边参数求值；`page.rs` 负责几何、对象顺序和页面计划。仅描边可以单独绘制；同时填充与描边时先填充，再描边，两次实例复用同一份本地路径与变换。几何和圆弧误差计算一次，不为两种画笔重复生成轮廓。

线宽在当前求值 profile 中属于页面/世界空间，组合非等比缩放作用于中心线，viewport 统一缩放再作用于笔宽。尖角百分比使用整数有理数舍入到 Q32，额外误差以 `authorMiterLimitErrorBound` 报告；设备 float32 参数误差继续单列。无量纲误差不混入像素坐标预算。这些界限仍不等于笔画外边界、尖角阈值连续性或抗锯齿保真证明。

页面 profile 升为 `author-page-solid-paths-certified-cpu-v2-draft`；`paintSources` 显式标识 `fill/stroke`。旧 74 页的输入、状态、设备帧和像素均保持不变。删除新增的零参数界和填充角色、回映射 profile，以及恢复一项旧的笼统描边诊断后，可以重建全部旧计划/响应摘要；合同升级不通过重录旧像素来掩盖差异。

## 实际验证与未关闭差异

新增 57 批作者页面用例：48 成功、9 明确拒绝。覆盖端点与连接组合、0/细/宽线、零长段、尖角边界参数、填充与描边、开闭路径、二次/三次曲线、旋转、非等比组缩放、翻转、主题色、透明度和 viewport 比例。页面合计 131 批，101 成功、30 拒绝；编译和绘制两个跨端入口不重复计数。

独立高精度参考核对 8356 个非背景绘制实例、73260 个控制坐标、7644 个真实圆弧采样点、32 项解析余项，以及作者/设备描边参数。绘制实例不是去重对象数；填充加描边的一个对象对应两个实例。

另外将 48 个成功样本转换为真实物理 EMU，再增加一份 12 对象样式网格。49 份同源文稿的 Native/WASM PPTX 字节、页面元数据和实际像素一致；独立 ZIP/XML/`python-pptx` 检查原生对象、线宽、端点、连接和颜色，294 个部件通过官方 ECMA Transitional XSD。这个辅助集合独立统计，不加入主内核累计批数。

LibreOffice 26.2.0.3 实际打开 48 份 PPTX 并导出 PDF。47 份带描边样本的端点、连接类型和颜色符合声明；透明样本通过 PDF 的透明组表达，观察器结合祖先组计算有效透明度，避免误把局部不透明画笔判断为丢失透明度。线宽最大观察差为约 0.01871 个 96 dpi 像素，低于事前设置的 0.05 像素观察容差。另一份无描边样本没有描边记录。

仍存在两类尖角差异：

- LibreOffice 在 11 个不同尖角声明中均输出 `3.8637033052 M`，没有反映请求的限值。保留实际 PDF 与声明对照，不能算作限值验证通过。
- WPS Office 12.1.22553 已在后台打开原生样式网格并取得实际画布。三种端点、圆角和斜角形态可见；三个 `lim=400000` 的尖角顶部明显截断，而当前 Skia 页面输出为完整尖角。同一网格的 `lim=0/100000` 接近斜角，`lim=800000` 为完整尖角。当前仅是形态观察，未做像素等价或编辑往返验收。

这一轮不根据少量截图猜测换算系数，也不把原生路径替换为图片。尖角限值的参数单位已有标准依据，但目标应用的超限几何、后端 miter-to-bevel 行为及 WPS 截断原因需要继续隔离核实。PowerPoint 尚无本轮实测结果。

累计 252 项 Rust 测试、2549 批主内核 Native/WASM 逻辑用例、49 份 Schema、严格 Clippy、格式与 TS 检查通过；9 份受作者模型/页面合同影响的 Schema 更新，其余 40 份不变。原有 2492 批保留上述明确的页面元数据升级，其余语义结果不变。独立组件的 174 批/61445 像素证据来自上一阶段，本轮组件二进制和适配器未变，未把复用证据写成重新执行。

没有新增外部运行依赖。当前未压缩开发产物中，CLI 为 4365264 字节，Rust WASM 为 3885653 字节，raster worker 为 3633680 字节；相对上轮分别增加 23504、34065、38864 字节。文字组件、Skia WASM 和薄适配器不变。以上不含完整一期能力，也不代表完整内核性能或 Musterwork 安装包大小。

## 复现与继续实现

按[开发说明](development.md)构建 Native、Rust WASM、配套当前 ABI 3 Skia 与 TS，生成既有组合和主内核回归输入，再执行：

```sh
node tools/verification/page-render-parity.mjs
python3 tools/verification/page-render-reference.py
node tools/verification/stroke-author-external.mjs
node tools/verification/stroke-author-grid.mjs
python3 tools/verification/contracts.py --path-raster-report .codex-work/path-raster/parity.json --scene-raster-report .codex-work/scene-raster/parity.json --page-render-report .codex-work/page-render/parity.json
```

实际文件/LibreOffice 观察使用带 lxml、python-pptx、PyMuPDF、NumPy、Pillow 的开发 Python，调用 `stroke-author-observations.py --xsd-directory <official-xsd-directory> --soffice <explicit-executable>`。工具只转换仓库自有输入，使用隔离的临时配置目录，保留差异，不更改验收容差来获得通过。WPS 记录来自 `cua-driver` 后台打开与窗口截图，公开记录仅保留自有画布裁剪和应用版本，不保存私人标签页内容。

`stroke-author-evidence.py` 检查当前产物/报告与历史摘要；`--seal` 只允许创建新证据，拒绝覆盖既有封存文件。重新复现需先按各实现说明生成全量回归报告。

接下来继续处理目标应用尖角/默认值/继承语义，并连接页面文字和图片、完整线型、复杂画笔与效果。高级可编辑对象、播放、公开 Agent 接入和 Musterwork E0–E3 保持原计划；没有缩减一期范围或宣称已经可以替换。

后续已对 5 份文稿、60 个原生对象完成限值/笔宽/坐标单位/夹角隔离观察。结果排除了直接套用路径单位的解释，并确认仅调整现有 miter-to-bevel 限值不能表达 WPS 的中间截断形状；生产语义尚未改写，详见[尖角兼容调查](miter-compatibility.md)。

后续已实现独立[求值截断尖角](miter-clip.md)及 ABI 3，全部既有作者页面像素保持不变；作者 miter 的目标应用映射仍需依据兼容证据单独完成。

后续作者 miter 的限值已改为可选，原生导出区分 `<a:miter/>` 与 `lim="0"`；缺省限值在页面编译中保持未解析诊断。实际文件往返读取、对象/主题线条声明及 136 批跨端验证见[来源线条](source-lines.md)。上文 252 项测试和 2549 批为作者描边阶段的历史统计。
