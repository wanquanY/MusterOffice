# 原生线性渐变与独立坐标平面

2026-09-25：将 PPTX 的线性渐变接入现有来源页面、Native/WASM 与 CLI。当前是开发 profile 的真实实现，**完整演示功能、Office/WPS 互操作和 Musterwork 替换尚未验收**。本阶段证据见[验证记录](../reviews/evidence/2026-09-25-native-linear-gradient-verification.json)。

## 已实现的链路

`source_page/gradient.rs` 消费既有原生填充继承与颜色查询结果，用接收形状的逻辑尺寸求值 `lin`、`tileRect`、色标、形状/组合变换；背景使用页面尺寸。保留来源声明及归属，不把渐变转换成图片，不修改原始 PPTX，不让共享绘制组件理解 Office 属性。此前 `grpFill` 的属性继承继续使用每个接收者自己的坐标系。

渐变色标保持来源顺序和相等位置形成的硬边，不排序、不合并。原生百分比以有界十进制整数比解析，直接按最近值、偶数舍入转换为 binary64 工作位置；不先降为 Q32。不同来源位置在转换中重合时拒绝。颜色采用颜色查询的非预乘 sRGB 工作通道，避免提前降为 RGBA8。现有下游仍检查 float32 色标坍缩、有限值、颜色范围及资源预算。

角度计算复用有界 Q96 三角函数。局部平面经已解析的原生放置映射一次，携带原生变换和数值求值误差；接收几何的锚点与来源原点分别处理。画笔随后属于世界坐标，路径或场景重复放置不会再次变换它。

来源填充已进入同一个页面预检和绘制顺序引擎，普通形状、组合继承、渐变背景、背景区域窗口共同工作。未实现的原生路径渐变与 `rotWithShape=false` 对象明确返回带位置的诊断，在图片解码之前停止；不会用圆形径向原语或普通线性渐变冒充这些能力。背景没有对象旋转，使用页面基准。

## 语义依据与限制

[Microsoft Open Specifications 确认记录](https://learn.microsoft.com/en-us/answers/questions/2265121/error-in-description-of-attribute-scaled-of-linear)确认规范对 `scaled` 法向量的宽高关系写反了，正确方向是 `(h cos θ, w sin θ)`。在单位平面中，`scaled=true` 的系数与 `(cos θ, sin θ)` 成比例；`false` 与 `(w cos θ, h sin θ)` 成比例。用投影极值归一化到 [0,1]，而不是把世界空间渐变线段的两个端点直接做非均匀缩放。

[MS-OI29500 §2.1.1297](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/f43f30df-c829-41f3-ba6f-52e9ac3b4e20)记录 Office 忽略 `gradFill.flip` 并使用 xy。因此本来源开发 profile 对两个平面坐标都反射平铺；原始 flip 声明仍在来源 binding 中。单位坐标分别平铺后再求颜色参数，二维平铺不能用一维渐变参数的 repeat/mirror 替代。

本阶段颜色插值选择 straight sRGB，是明确的开发解释，不能凭 Native/WASM 一致宣称目标 Office/WPS 的全部透明度、色彩和边界行为已经匹配。固定方向、circle/rect/shape 路径渐变、箭头线条的渐变边界等仍需补齐；微软文档所述箭头边界扩大不能套用本阶段普通形状矩形。

## 通用绘制模型与组件

`GradientGeometry::Plane` 包含两个独立对象：

- `GradientPlane`：Q32 世界 `origin`、`xStep`、`yStep`、独立的 `tileX`/`tileY`（clamp/repeat/mirror），可选非负参数误差。
- `GradientField::Linear`：三个 Q32 无量纲系数及可选非负误差。对平铺后的 `(u,v)` 求 `t = nx*u + ny*v + offset`，再使用既有颜色 ramp 的 tile、颜色空间和 alpha 策略。

图片与渐变共用前向基矩阵量化和参数误差验证；旧图片的帧、数值报告与像素保持不变。二维变换需要可逆，最小奇异值不得低于 2⁻¹⁴ 设备像素，映射边界保持既有设备范围限制。重复相位误差覆盖可见视口，不能仅检查一个 tile。字段三个系数量化及上游误差的和不得超过 2⁻²⁰。

`gradientCoordinateErrorBound` 与 `gradientValueErrorBound` 是输入参数与工作值的误差报告，**不是整个浮点 shader 或最终像素误差的证明**。binary64 来源色标工作值与其原始有理数仍是不同数值层。

CPU 组件使用固定向量坐标阶段 `mo_gradient_plane`，接入原有 Skia 颜色渐变和共享合成。没有额外渐变位图、按 tile 增加的绘制项、运行时 shader 编译器或新的第三方依赖。新增补丁与源码属于仓库维护扩展；Skia、图片解码依赖和许可证锁不变。

只有使用映射渐变的请求升级为 V9：

| 项目 | 约定 |
| --- | --- |
| Header / draw | 沿用 V8 的 14 / 8 words，含图片、裁剪、快照字段 |
| 最大帧 | 2,883,662 个 u32 |
| 渐变公共头 | kind、ramp tile、插值、预乘、stop count，共 5 words |
| kind 0 / 1 | 旧线性/径向几何 4 words，旧帧不变 |
| kind 2 | 11 words：前向仿射矩阵 6、tileX/Y 2、线性系数 3 |
| 色标 | 仍为 position、r、g、b、a，每个 5 words |
| 可选能力 | `mo_skia_gradient_planes_abi() == 1`，TS `supportsGradientPlanes` |

旧组件遇到 V9 在调用前拒绝，已有 ABI4 / images2 / clips1 / compositing1 不变。C++ 完整验证几何、停止位置、参数范围、帧长度和预算后执行。当前 Native 静态构建使用同一固定扩展闭包。

## 本阶段证据

- 584 项 Rust 测试、严格 Clippy、rustfmt、80 份 Schema 和同源 TS 检查通过。
- 66 对共享路径/场景 Native/WASM 请求；27 类矩阵与轴平铺组合的 20,736 像素经独立逆矩阵及平铺参考核对，最大通道差 1 色阶。其余验证透明度模式、颜色空间、硬边、裁剪、快照及非法参数。
- 17 个自有原生 PPTX，119 个 XML 部件通过官方 XSD；其中 14 页、1,680,000 像素通过独立来源 XML 参考，最大通道差 1 色阶。组合继承与显式填充控制的完整像素一致。独立像素参考不涵盖旋转/组合抗锯齿边界。
- 另有 4 个结构有效的原生负例，当前页部件通过 XSD，路径渐变、固定方向、非正 tile 和逆序色标均得到诊断；Native/WASM 不调用图片解码或栅格组件。
- 来源页面共 107 对公共调用，87 对成功；前一阶段 86 对请求的元数据、帧摘要和像素不变。794 对旧图片/裁剪/合成请求，以及 307 对旧页面/文字请求保持一致。
- 250 组 Native/WASM/ASan/UBSan 直接组件检查，包括 22 个拒绝和 168 个旧 V7/V8 帧；未运行 LeakSanitizer。共享路径和来源页面 CLI 验证创建、拒绝覆盖及失败无输出。

LibreOffice 26.2.0.3 对 16 个新样本进行了真实导入与 PDF 转换。记录所有 400×300 页面差异，**不把 PDF 比较当成 Office/WPS 验收**。部分大差异位于边缘，但缩放和组合也有内部差异，例如 `scaled-30` 在 (225,75) 为内核 RGB `[131,0,124]`、应用 `[112,0,143]`；组合继承在 (50,75) 为 `[178,0,77]` 与 `[244,0,10]`；平铺在 (50,75) 为 `[193,0,62]` 与 `[227,0,28]`。现阶段保留已查证的微软规则及差异记录，目标应用实测仍待补齐。

未压缩 C++ WASM 为 **2,325,263 字节**，比前阶段增加 1,636；Rust WASM 为 **6,206,176 字节**，增加 61,060。这仅是两个组件产物的体积，**没有测量整个安装包、RSS 或产品性能收益**。

## 复现

使用新输出目录 `.codex-work/gradient-field/`，保留此前阶段封存产物。`component` 目录需要已有锁定 Skia 压缩包，图片依赖使用已有固定 native/wasm/asan 构建。

```sh
python3 tools/components/build-skia.py --target native --directory .codex-work/gradient-field/component --image-codecs .codex-work/image-codec/deps-native
python3 tools/components/build-skia.py --target wasm --directory .codex-work/gradient-field/component --image-codecs .codex-work/image-codec/deps-wasm
python3 tools/components/build-skia.py --target native --sanitize --directory .codex-work/gradient-field/component --image-codecs .codex-work/image-codec/deps-asan
python3 tools/verification/gradient-field-checks.py
python3 tools/verification/contracts.py
python3 tools/verification/gradient-field-invalid.py
node tools/verification/gradient-field-parity.mjs
node tools/verification/gradient-field-source-parity.mjs
node tools/verification/gradient-field-components.mjs
node tools/verification/gradient-field-regressions.mjs
node tools/verification/text-page-runtime-regressions.mjs .codex-work/gradient-field .codex-work/gradient-field/component .codex-work/gradient-field/ts-raster/index.js
python3 tools/verification/gradient-field-reference.py
python3 tools/verification/gradient-field-observe.py --soffice /Applications/LibreOffice.app/Contents/MacOS/soffice
python3 tools/verification/gradient-field-evidence.py
```

Python 验证需要 `jsonschema`、`lxml`；应用观察另需 PyMuPDF，应显式选用含这些包的 Python 环境。完整来源导入、创建/编辑/导出、播放与高级对象，以及 Agent/Musterwork 的可安装接入闭包继续推进。
