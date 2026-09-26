# 来源图片资源绑定与提取

状态：Rust、Native CLI、WASM 已实现并经过开发验证，DRAFT。此阶段连接真实 PPTX 的图片填充声明和 OPC 资源字节。它为后续图片解码及页面绘制提供输入，不表示完整图片渲染、Office/WPS 互操作或 Musterwork 替换已经完成。

## 来源与资源身份

`mo-pptx::source::images::query` 复用既有填充继承、主题引用、组合和背景跳转。请求由 `SourceFillQuery` 和显式 `ImageSourceSelection` 组成，绑定来源文件 SHA-256、表面部件、填充目标及填充解释 profile。`query_on_page` 可使用页面编译器提供的独立绘图/背景上下文。

每个 `r:embed` / `r:link` 都按照**该属性实际声明的部件**查找关系，不能按最终消费它的幻灯片查找。同一个 ID 在主题、母版、版式和幻灯片中可以指向不同资源；嵌入和链接属性也可以分别继承自不同部件。结果保留完整的有效图片填充、声明来源、跳转记录、关系所有者、原始 ID 和目标 URI。

来源选择不可省略：

- `embeddedSnapshot`：读取嵌入关系指向的包内快照。必须是当前支持的 Transitional image 关系、内部目标、无 fragment，并声明 `image/*` 内容类型。
- `linkedSource`：返回 `externalRequired` 及其来源绑定。核心不发起网络访问，不自动使用嵌入快照；宿主需要另行提供经过授权和封存的外部资源。

声明不存在、关系缺失、类型/TargetMode 不符、fragment 和非图片 MIME 分别给出结构化原因。非图片填充返回 `notImage`；尚不能求值的图片效果保留填充层的 `unresolvedFill`，不忽略效果后继续使用图片。

同一规范 OPC part 在一次查询中只加入清单一次。资源顺序按目标的首次使用确定，包含 part、原始 MIME、SHA-256、十进制字符串 byteLength/offset。不同 part 即使字节相同也保留各自的来源身份。关系查找按声明部件缓存；清单复用不复制图片数据。

## 字节提取和预算

`extract` 在读取任何图片 payload 前检查整个清单：来源摘要、资源数、部件存在性、无重复 part、连续 offset、原始 MIME/长度/摘要及总长。随后通过 OPC 读取、验证实际 SHA-256，并返回连续编码字节。任意错误或取消均不返回部分 bundle；输入文件与来源索引不改写。

检查目录返回 `inspected`，其中可以同时存在 `available`、外链和未解析目标。提取只包含清单中明确列出的内部资源，允许空 bundle；**`inspected` 不表示全部目标已就绪**，页面编译器必须检查各目标状态。宿主不得把空 bundle 或部分目标可用当作整页渲染成功。

默认预算为 4096 个资源、每项 32 MiB 编码字节、bundle 64 MiB、4 MiB 新增引用元数据、100 万次关系访问，同时沿用填充层及 OPC 的预算/取消。图片查询本身不再次读取 payload；建立 OPC 包图时已有范围读取、CRC 和摘要检查。提取阶段再次读取并检查实际字节。宿主仍须保证 `ReaderAt` 在操作期间不可变。

上述预算不等于解码后大小或进程 RSS。提取需要输出 bundle 及当前资源缓冲，尚未实现流式解码、驻留缓存或跨请求资源池。MIME 只是包内声明，并未证明编码有效；具体格式、尺寸、帧数、方向、ICC 和解码安全由下一层验证。

## 接口

- Rust API：`inspect_pptx_images` 支持范围读取器、预算和取消；`inspect_pptx_images_json` / `extract_pptx_images_json` 是严格 JSON 的开发接口。
- CLI：`pptx-images <request.json> <source.pptx>`；`pptx-extract-images <request.json> <source.pptx> <new-resources.bin>`。
- WASM：`inspect_pptx_images(request, source)`；`extract_pptx_images(request, source)` 返回 metadata 与一次性 `take_bytes()`。
- [请求 Schema](../../contracts/generated/pptx-image-query.schema.json)、[响应 Schema](../../contracts/generated/pptx-image-response.schema.json)及 TS 从共享 Rust 类型生成，编码字节不放进 JSON。

CLI 仅在完整提取成功后暂存、sync、读回校验并发布新文件；已有输出不覆盖。JSON/语义失败只有错误元数据，无输出文件。该入口是显式文件路径的开发宿主，不是已经完成的 Agent Artifact 或生产权限边界。

## 验证和重现

[验证记录](../reviews/evidence/2026-09-25-source-images-verification.json)绑定源码、当前构建、测试和独立参考。10 个新增 Rust 测试覆盖直接图片、同 part 复用、相同 payload 不同 part、四层同名关系、嵌入/外链的独立继承、组合/背景、错误关系、预算/取消和提取前全目录校验。

19 个实际 PPTX 查询及 6 个错误请求分别通过 Native 和 WASM 的查询/提取接口，共 50 对调用；结果元数据和编码字节一致，19 项已有输出拒绝覆盖。独立 Python ZIP/XML 检查声明部件、关系目标、内容类型、首次使用顺序、offset 和实际字节，核对 12 个可用/外链引用及 9 个包内 payload。没有用渲染截图代替可编辑来源或资源身份。

全工作区 515 项 Rust 测试、严格 Clippy、格式、74 份 Schema/TS 检查通过。既有 518 个 Native/WASM 请求回放涵盖页面、文字、几何和共享图片画笔；保存的元数据及成功像素保持一致，另保留语法拒绝检查。固定 Skia/HarfBuzz 组件与此前发行产物不变。没有新增产品延迟、RSS、安装包大小或目标应用测量。

先按[开发说明](development.md)准备依赖及固定组件，使用独立输出目录保留旧证据。生成语料时环境路径必须为绝对路径，因为 Cargo 测试在 crate 目录运行：

```sh
MO_SOURCE_IMAGE_EVIDENCE_DIR="$PWD/.codex-work/source-images/cases" cargo test -p mo-pptx --locked --test source_images
```

Native 构建使用 `MO_SKIA_LIB_DIR="$PWD/.codex-work/image-brush/component"`。构建当前 `mo-cli`、两个 Worker、`mo-wasm`，将 debug Rust WASM 的 bindgen 输出写入 `.codex-work/source-images/wasm-node/` 并设置 Node CommonJS 包边界。运行新 Schema/TS 生成和检查后执行：

```sh
node tools/verification/source-images-parity.mjs .codex-work/source-images runtime-new
python3 tools/verification/source-images-reference.py
node tools/verification/text-page-runtime-regressions.mjs .codex-work/source-images .codex-work/image-brush/component .codex-work/image-brush/ts-raster/index.js
node tools/verification/source-images-regressions.mjs
```

parity 的输出目录须是新的，防止意外覆盖已有产物。验证记录需在所有命令完成后生成并检查结束标记。

下一步是编码图片的确定性解码、方向/色彩处理，以及 `srcRect/tile/stretch` 到共享图片画笔的编译。来源编辑、原生导出、高级内容、生产资源宿主和 E0–E3 产品链路继续按完整一期目标推进。
