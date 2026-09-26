# 共享图片资源与绘制

状态：实现与开发验证，DRAFT。此阶段为原有 Rust → Skia CPU 链路增加图片资源和画笔，沿用路径、描边、世界坐标画笔、SrcOver、隔离组件与原子输出；不是完整 PPTX 图片能力或产品替换验收。

下文记录最初 V5 阶段。当前图片扩展 ABI 2 在保留 V5 的同时增加 V6 显式源像素区域，指令上限为 2,691,084 字；新合同、采样语义与验证见[图片源区域采样](image-domain.md)。

## 输入与职责

`mo-raster::PreparedImages` 接收显式清单及独立二进制 bundle。每项声明宽、高、Alpha 类型和 SHA-256；数据按清单顺序紧密排列，每个像素为 sRGB RGBA8，无行填充。核心检查尺寸、总量、摘要、预乘通道和取消，不查找文件、URL、系统资源或 ICC。输入应已完成解码、方向处理和色彩归一化。

一次请求最多 4096 项、64 MiB 源像素，每边不超过 8192。总量约束按完整资源计算，不能通过重复引用放大扫描工作。准备结果持有验证后的元数据并借用不可变字节，Rust 宿主可以复用准备结果而不再次校验摘要。绘制对象引用资源索引，重复画笔在编译批次内去重。

`Brush::Image` 包含资源索引、世界 Q32 EMU 原点、每个源像素对应的两个位移向量、横纵独立平铺模式及采样模式。源像素边界 `(u, v)` 映射为 `origin + u*xStep + v*yStep`。支持仿射旋转、缩放、翻转、倾斜，以及 clamp/repeat/mirror/decal；采样为 nearest 或 linear，无 mipmap。路径定义覆盖区域，画笔独立于局部路径原点。尚未在来源 PPTX 编译器中把 `srcRect/tile/stretch` 等原生声明降低为该画笔。

Q32 → f32 仍使用精确有理数转换。图片的 `coordinateErrorBound` 是各系数量化误差在整个源图片域上的保守位移界，必须满足视口精度预算；不是逆矩阵、滤波、抗锯齿或最终颜色误差界。组件进一步拒绝非有限矩阵、越界角点与最小奇异值小于 `1/16384` 的矩阵，不静默降级。

## Alpha 与共享后端

组件复用现有绘制器。预乘输入通过借用 `SkData` 构造图像，Native 不复制源像素；WASM 跨独立线性内存需要显式拷贝。所有借用在同步调用返回前释放。一次绘制中每项资源只构造一次，多个画笔和对象复用它。

直接对 Skia 的 straight Alpha 输入进行线性过滤，会先插值未预乘通道，导致透明边缘颜色偏差。实现已在资源入口统一处理：straight 输入按 `(channel * alpha + 127) / 255` 转成 RGBA8 预乘数据，再使用同一图像着色器。每项 straight 资源每次组件调用最多分配和转换一次；优先让解码器直接提供预乘数据。隐藏的透明 RGB 不参与边缘着色。

资源驻留、跨帧图像缓存、解码/ICC、重采样质量档位及动画帧预算尚未完成。当前 64 MiB 限制是源 bundle 上限，不是整个进程 RSS 上限；straight 转换、输出帧、指令、运行时均另占内存。不能据此宣称产品性能或安装包达标。

## 运行接口与二进制协议

- Rust：`compile_images` / `render_compiled_images` / `render_images`，共用既有编译和结果验证路径。
- Native 开发 Worker：`mo-raster-worker --images`，每次输入为 `u32le JSON长度 + u32le 资源长度 + JSON + 资源`；输出沿用 `u32le 元数据长度 + u32le 像素长度 + 元数据 + 完整像素`。
- WASM：`render_image_paths(json, resources, rasterComponent)`；TS 组件提供 `supportsImages` 和 `rasterImages`。输入禁止共享可变缓冲区及组件自身内存视图。捕获 trap 或无效结果后丢弃实例。
- 合同：[请求](../../contracts/generated/image-raster-request.schema.json)、[响应](../../contracts/generated/image-raster-response.schema.json)，与其他合同一样由 Rust 生成 Schema/TS。请求包含 `raster` 和 `images`，像素不进入 JSON。

旧组件入口 `mo_skia_abi() == 4` 与 V4 指令保持不变。新扩展查询 `mo_skia_images_abi() == 1`，`mo_skia_raster_images` 只接受 V5 指令与独立 bundle：

1. 原有 10 个头字，版本改为 5，追加资源数、图片画笔数。
2. 原有路径、描边、渐变资源；追加每项 4 字的 `[byteOffset, width, height, alpha]`。
3. 每个图片画笔 10 字：`[resource, tileX, tileY, sampling, m00, m01, m02, m10, m11, m12]`，矩阵为 f32 位模式。
4. 原有每项 6 字的 draw；最后一字为统一着色器索引，先渐变、后图片，零仍表示纯色。

指令上限 2,674,700 字。整个语法、范围及预乘数据在 Skia 调用前验证。返回码和输出所有权与 V4 一致；任意失败不发布部分像素。没有资源参数的旧 Rust 路径/场景入口遇到 image 画笔会明确拒绝，不能单独调用旧入口来完成图片渲染。

图片输出使用 `skia-8d6d37b-q32-image-brushes-srgb-premul-rgba8-v5-draft`。身份由 profile、frame SHA-256 和 resources SHA-256 联合确定，不能只用指令摘要作为缓存键。

## 当前验证

证据入口：[图片绘制验证](../reviews/evidence/2026-09-25-image-brush-completed-verification.json)。原始输入、执行日志与组件构建放在忽略的 `.codex-work/image-brush/`；公开记录只含摘要、统计与相对路径。

- 30 个成功图片用例，覆盖两种 Alpha、两种采样、四种平铺、旋转/翻转/倾斜、分数坐标、极大世界原点、系数量化、矩阵下界、背景混合、多次绘制、多个资源和透明渐变叠加。
- 9 个 Rust 资源/几何预检失败、7 个 JSON 拒绝、1 个 WASM 宿主坏结果拒绝；无部分输出。23 个直接组件语法/范围/像素拒绝用例在 Native、WASM、ASan/UBSan 下执行，同一个 WASM 实例拒绝后恢复成功。
- Native Rust 与实际 Rust WASM 的元数据及成功像素一致；成功图片及 23 个组件拒绝输入另外通过独立 Native probe 和 ASan/UBSan probe。当前 macOS 工具链不支持 LeakSanitizer，因此未声称泄漏检测通过。
- 独立 Python `Fraction` 参考从原始请求反演仿射矩阵，计算横纵平铺、预乘采样及 SrcOver，比较 7,680 个像素，每通道容许 1 个 RGBA8 舍入单位，不排除几何边缘像素；另核对 124 个仿射角点的输入误差界。straight 与等价 premultiplied 图像（含隐藏透明颜色）另做逐字节比较。
- 既有 472 个当前 Native/WASM 请求回归，覆盖来源页面、文字页面、文本与几何，原有元数据和成功像素不变。历史总量与本轮执行量分开记录。

505 项 Rust 测试、严格 Clippy、格式检查及 72 份 Schema/TS 检查通过。

固定 Skia 版本、构建配置和运行依赖未改变；增加了自有 C++ 适配代码及 Rust/TS 接口。该构建的未压缩 Skia WASM 为 1,810,991 字节，旧组件为 1,808,300 字节，相差 2,691 字节；这是单个开发组件比较，不是完整内核、压缩包或 Musterwork 安装包估算。没有新增产品延迟、RSS 或安装包测量。

## 重现

先按[Skia 组件说明](skia-component.md)准备固定归档及构建工具。此轮在独立目录构建，保留此前组件与发行证据：

```sh
python3 tools/components/build-skia.py --target native --directory .codex-work/image-brush/component
python3 tools/components/build-skia.py --target wasm --directory .codex-work/image-brush/component
python3 tools/components/build-skia.py --target native --sanitize --directory .codex-work/image-brush/component
```

Rust 构建时将 `MO_SKIA_LIB_DIR` 设为上述组件目录的**绝对路径**，生成 Worker、验证 example 和 Rust WASM；WASM bindgen 输出到 `.codex-work/image-brush/wasm-node/`（Node commonjs）。TS 适配器用 `pnpm exec tsc --project packages/raster-component/tsconfig.json --outDir .codex-work/image-brush/ts-raster` 构建；保留此前 `.codex-work/raster-component/index.js` 封存文件。随后运行 fixtures、parity、reference、regressions 脚本及[开发检查](development.md)。旧链路回放使用 `node tools/verification/text-page-runtime-regressions.mjs .codex-work/image-brush .codex-work/image-brush/component .codex-work/image-brush/ts-raster/index.js`。证据脚本要求全部结束标记并校对，不覆盖已有封存记录。此前一次候选记录在最后回放结束前记录了空日志，已由当前 completed 记录明确取代，不能使用该候选作为最终验收证据。

下一步继续把原生图片关系、资源解码/色彩、裁剪/平铺、图片效果、编辑与可编辑 PPTX 导出接入此共享链路。完整文字、动画/转场、音视频、SmartArt、公式、生产 SDK/MCP/Skill/Plugin、Musterwork 宿主和 E0–E3 仍须完成。
