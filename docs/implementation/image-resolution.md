# 图片分辨率与物理尺寸

2026-09-25。状态：在静态 PNG/JPEG 解码入口实现来源分辨率解析、精确换算和歧义报告。此能力为原生 PPTX 图片平铺提供尺寸依据；尚未连接完整来源页面图片绘制，也不代表 Office/WPS 的 DPI 选择行为已经验收。

## 数据与职责

`mo-image` 在固定组件完成容器/像素解码后，从同一份已核对摘要的编码字节读取 PNG `pHYs`、JPEG APP0 `JFIF` 与 PNG/JPEG EXIF IFD0 的 `XResolution`、`YResolution`、`ResolutionUnit`。不读取文件路径、系统 DPI、屏幕缩放或网络，不新增解码器依赖，不改变固定 C++ ABI。

`DecodedImageInfo.resolution.declarations` 保留来源种类、chunk/marker 的绝对字节偏移、编码轴上的原始分子/分母及单位。EXIF 缺省字段保留 `null`，不把推导值写成原始声明。分辨率不会改变输出像素、颜色转换、像素摘要或已归一化方向；同一个解码结果仍进入原有图片资源和共享场景。

`physicalPixelSize` 是各声明的严格一致性结果，不是目标 Office/WPS 兼容性 profile。它只有以下四种状态：

- `known`：横纵像素的精确 EMU 尺寸；约分后分子使用 int64 十进制字符串，分母为正 uint32。EXIF 方向 5–8 只在该推导值上交换横纵轴，原始声明不变。
- `unspecified`：没有绝对尺寸声明，包括只有相对像素比例的 PNG/JFIF。不会自动补 96 DPI。
- `zeroDensity`：存在零密度，不能除以该值推导物理尺寸。
- `conflicting`：声明的物理尺寸或横纵密度比例不一致。全部声明保留，不按读取顺序覆盖。若同时存在多种问题，冲突优先于零密度；这只是确定的诊断顺序。

换算常数是 1 inch = 914400 EMU、1 cm = 360000 EMU、1 m = 36000000 EMU，全程整数/有理数计算。容器的原始分子分母不约分；只约分派生几何。大分母、极小或极大的密度不会先经过浮点或 JS Number。

## 格式规则与边界

PNG `pHYs` 的单位 0 只定义比例，单位 1 是每米像素数；不接受重复、IDAT 后的 `pHYs`、错误长度或保留单位。JPEG JFIF 单位 0/1/2 分别表示比例、每英寸、每厘米；验证头部与缩略图字节长度，重复声明失败。JPEG 扫描覆盖 entropy stuffing/restart 与扫描后的 APP 元数据，避免只读首个 SOS 前的数据。

EXIF 只读取 IFD0 的相关字段，支持两种字节序，不跟随缩略图、GPS、Exif 子目录或任意 IFD 链。按 Exif 规范，缺省 X/YResolution 使用 72、缺省 ResolutionUnit 使用 inch；这些是格式规定的缺省值，并非宿主猜测。即使 EXIF 仅含方向，其存在与缺省分辨率也会记录。EXIF 单位仅接受规范列出的 2/3；错误类型/计数、重复字段、越界指针、零分母或保留单位失败。零分子作为原始声明保留并返回 `zeroDensity`。

PNG 规范指出 eXIf 可能记录历史信息，并不规定它与其他 PNG chunk 的冲突裁决。因此当前同时保留它与 `pHYs`，分歧明确报告；不能把该严格一致性规则声称为 PowerPoint 或 WPS 的最终选择规则。实际页面编译还需结合 DrawingML 的显式 `dpi`、来源裁剪、stretch/tile 和兼容性 profile。完整的 TIFF 文件解码并不在本入口范围内。

解析沿用 32 MiB 编码输入及 4096 chunk/marker 上限，所有切片和偏移有界；JPEG 长扫描和 marker padding 每至多 16384 字节检查取消，IFD0 逐项检查取消。无效分辨率会丢弃整份待发布解码结果；普通输入错误不损坏组件实例，组件调用后的取消仍使实例失效。解码组件继续负责 CRC、压缩流、色彩和像素完整性；元数据阶段不建立第二套像素解码路径。

## 验证

库测试覆盖单位精确换算、方向与原始值区分、格式缺省值、绝对/相对声明冲突、两种字节序、晚到 JPEG 元数据、截断、恶意指针/重复字段、最大有理数、预算和逐检查点取消。运行时语料由 `tools/verification/image-resolution-fixtures.py` 生成；几何期望由独立 Python `Fraction` 计算，像素期望来自自有固定样本值。

原生 Worker 与实际 Rust/WASM 执行解码并比较完整元数据/像素，成功结果继续走共享场景绘制。另以新运行时重放上一阶段语料，要求除新增 `resolution` 字段外所有旧字段、错误和像素保持一致。检查结果与产物由[本阶段证据](../reviews/evidence/2026-09-25-image-resolution-verification.json)绑定；固定 C++ 组件和旧发行产物必须保持原摘要。本阶段没有新增目标应用、峰值 RSS、时延或安装包实测结论。

## 参考

- [PNG 第三版：pHYs / eXIf](https://www.w3.org/TR/png-3/)。
- [JPEG File Interchange Format 1.02](https://www.w3.org/Graphics/JPEG/jfif3.pdf)。
- [CIPA DC-X008-2019 Exif 规范](https://www.cipa.jp/std/documents/e/DC-X008-Translation-2019-E.pdf)。
- [DrawingML blipFill 的 DPI 语义](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.blipfill?view=openxml-3.0.1)。
