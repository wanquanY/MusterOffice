# 来源文字页面运行接口

本文件记录运行接口首个冻结阶段；其后完善的字体资源错误与 run 位置见[文字资源诊断](text-resource-diagnostics.md)，单下划线/删除线见[文字装饰](text-decorations.md)。下文对应未完成项是该记录时点的状态。

2026-09-25。[原生文字页面库](source-text-page.md)已接入共享 Rust API、Native CLI/隔离工作进程和实际 WASM 执行。开发入口接受真实 PPTX、显式字体清单与字体字节，返回紧凑元信息及 RGBA。该实现仍使用纯色文字 draft 配置，完整编辑/高级内容/播放、Office/WPS 互操作及 Musterwork 产品适配尚未完成。

## 输入与资源责任

`PptxTextPageRequest` 的 `profile` 为 `drawingml-solid-text-page-q32-draft-v1`；`page` 复用原来源页面请求，包含源 SHA-256、slide 部件、形状配置、颜色上下文和视口；`fonts` 是显式 `FontManifest`。分别传入 PPTX 与字体 bundle 字节，不把文件路径、系统字体查找或下载行为加入核心。

字体 bundle 内的 offset/byteLength 是十进制字符串，按每项摘要、face、名称记录和实例验证。字体清单验证包括未使用的声明；一个已验证清单供所有框共享。PPTX 来源摘要冲突先于字体计算失败。JSON 的未知字段、重复键、未知配置和错误数值类型均被拒绝。

请求最多 32 MiB，PPTX 与字体 bundle 分别最多 128 MiB；后续来源、字体、整页和组件各有既定预算。这不是同时存活内存或 RSS 上限。Rust 类型化入口同样检查资源字节上限；页数/对象/文字等计算限制不能由 JSON 任意提高。普通渲染在调用绘制组件前释放详细来源和塑形计划；完整诊断计划仍由库级 `compile` 提供，本次没有扩张为完整计划的 JSON 返回接口。

Schema 分别为 `contracts/generated/pptx-text-page-request.schema.json` 与 `pptx-text-page-raster-response.schema.json`，TS 从这两份 Rust 生成的 Schema 生成，不能手写另一套排版或格式解释。

## Native 与 WASM

```sh
mo-cli render-pptx-text-page request.json source.pptx fonts.bin new-output.rgba
```

CLI 通过同目录 `mo-raster-worker --pptx-text-page` 同时调用 Native HarfBuzz 与 Skia。worker 的二进制输入按顺序是三个 little-endian u32 长度（JSON、PPTX、字体），再跟三个对应字节段；每段预算在分配前检查，截断/超限帧不产生成功响应。输出复用两个 u32 长度及 JSON、RGBA 段。一个进程可顺序处理多个完整请求；任何组件失效后退出，不能通过新建 Rust 包装器恢复静态组件。

CLI 复用现有 30 秒超时/进程回收、响应长度/配置/像素摘要检查及暂存校验发布。错误响应必须没有像素；已有目标文件不会被覆盖。此处的文件产物发布尚未实现 Musterwork Artifact/CAS、权限与任务持久化。当前合并 worker 增加对既有 `mo-harfbuzz-sys` 的本地运行依赖，没有新增外部组件版本或 C/C++ 实现；后续生产分发仍须测量共享组件、进程池及安装体积。

WASM 暴露 `render_pptx_text_page(request, source, fonts, shaping, raster)`，接受 JSON 字符串、两个 Uint8Array 和已有薄组件宿主。核心调用同一 Rust 页面链，TS 只做传输与组件内存操作。返回 `RenderedRaster`，先读取 `metadata`，再用 `take_pixels()` 消费句柄并取走像素。实例生命周期和浏览器 Worker 归宿主管理；本次没有实现浏览器生产 Worker、用户取消和实例池。

## 失败合同

返回 `status: rendered` 时，`info` 包含文字页面 profile、共享页面/场景元信息、文字框数和累计工作计数；像素的格式、尺寸、字节长度与摘要沿用实际 raster 合同。

`status: error` 的 `error.stage` 区分 `request`、`source`、`fonts`、`page`。资源阶段保留已有字体错误码；页面阶段保留来源页错误码和可用的 `location`，额外 `detail` 可表达原生框未实现项、文字画笔属性/声明、颜色原因、缺失画笔或字形簇颜色冲突。错误码与结构化原因来自 Rust 枚举，不根据错误字符串推断类别。新增内部对象位置封装让文字计算失败能定位到实际页面对象，同时保留底层错误类型。

声明型画笔错误保留其物理来源，框诊断保留可用段落位置。部分非声明画笔错误目前只定位到对象，字体选择缺失仍沿用既有文本错误分类；精确到 run 的统一诊断和可行动的字体缺失合同仍须完善，不能据此宣布完整 Agent 修复接口。旧无字体入口继续明确拒绝文字，没有从旧合同隐式切换配置。

## 本轮验证

- 全仓 463 项 Rust 测试；严格 Clippy、格式、70 份 Schema 生成检查及 TS 类型检查通过。新增边界测试覆盖源/字体身份、取消、未知/重复字段、字体名称和对象诊断。
- 27 个新操作 Native/WASM 请求的 JSON 完全一致；其中 11 个真实 PPTX 成功页面的全部像素字节一致，其中 10 个也与上一轮已核对坐标和内部像素的结果相同，另 1 个是原生叶文字编辑后的真实新包。其余请求覆盖缺失/损坏字体、资源范围、请求类型、后续对象下划线、autofit、缺失画笔和不同颜色的组合字簇。
- 原生叶文字编辑在 Native 与 WASM 中产生相同的新 PPTX 字节，新来源可重新渲染且像素发生变化；原始包保持不变，复用旧来源摘要被拒绝。此项不等于目标应用的编辑重存验收。
- 同样的 27 个请求经 CLI 发布链运行；11 次重复发布均拒绝覆盖，16 个失败请求均不产生图像。两个连续 worker 请求以及五种超限/截断帧分别检查。
- 对真实 WASM 字体组件在布局后的轮廓阶段注入分配故障，验证无部分图像、组件失效、拒绝复用及健康实例仍可绘制；另验证绘制宿主异常后的无图像与失效处理。故障组件是已有专用测试构建，不是向生产协议加入故障入口。
- 当前 Native 与当前 Rust WASM 另重跑 100 个旧页面请求和 207 个旧文字请求，全部保持原结果，21 个旧页面像素也一致。

这一轮的双端请求数是 334（27 新操作＋307 旧操作），不能把历史 10064 批当作新构建全部重跑的证据。原有 11 个冻结发行产物没有被重建；本次使用当前 debug Native 与独立目录的 debug Rust WASM，加上已有固定组件。没有新增产品性能、峰值内存或安装包体积测量，也没有 Office/WPS 打开编辑、重存或播放证据。像素样本使用原创合成字体，不能作为真实字体视觉质量展示。

## 复现

```sh
cargo build -p mo-cli -p mo-raster-worker -p mo-text-worker --locked
cargo build -p mo-wasm --target wasm32-unknown-unknown --locked
# 使用已固定 wasm-bindgen 版本，将 debug/mo_wasm.wasm 绑定到
# .codex-work/text-page-runtime/wasm-node，nodejs 目标需 CommonJS package.json。
cargo run -p mo-contract-codegen --locked -- check contracts/generated
pnpm check:types
python3 tools/verification/text-page-runtime-fixtures.py
node tools/verification/text-page-runtime-parity.mjs
node tools/verification/text-page-runtime-regressions.mjs
python3 tools/verification/text-page-runtime-contracts.py
```

夹具生成会先验证上一轮冻结记录绑定的原创 PPTX/计划/像素，然后构造新的原生 XML 失败样本；不会改写旧报告。记录见[文字页面运行接口验证](../reviews/evidence/2026-09-25-text-page-runtime-verification.json)。下一步仍沿原方案完善文字能力、字体资源与诊断、实际产品任务/产物连接及完整高级内容，直至达到 Musterwork 替换验收条件。
