# 图片源区域采样

状态：DRAFT 实现；来源 PPTX 图片页面编译和 Office/WPS 裁剪规则验收继续推进。本阶段为既有图片画笔增加连续的源像素区域，保持资源、路径、场景、坐标预算和隔离后端共用。

## 合同与坐标

`ImageBrush.sourceDomain` 可省略；提供时为 `{left, top, right, bottom}`，四项是 **Q32 源像素**的十进制整数字符串。它们不使用世界 EMU，不随路径变换树再次变换。世界映射仍为 `origin + u*xStep + v*yStep`。画笔的区域可以小于一像素，也可以越出原图，不能为空或反向。

省略区域时保留既有整图行为和 V5 指令；同批含一个显式区域则生成 V6，每个普通图片画笔补全为其整图范围。显式整图区域在组件中直接使用旧 Skia 图像着色器，保持原有采样结果。请求仍通过原有 `render_image_paths` / `render_image_scene` 及 Native Worker 入口执行，无第二套页面引擎。

`sourceDomain` 是低层采样边界，不是对 DrawingML `srcRect`、`fillRect` 或 tile 对齐属性的直接解释。后者需要结合图片物理尺寸、来源继承及页面几何编译。来源解析见[来源图片](source-images.md)，分辨率见[图片分辨率](image-resolution.md)。

## 明确的采样规则

四种横纵平铺模式继续独立选择；nearest / linear 在归一化预乘 sRGB RGBA8 上计算，最后进入既有颜色转换及 SrcOver。

- clamp 把坐标限制在区域内的采样中心；linear 向内缩半像素，区域不足一像素时使用区域中点。
- repeat 按区域宽高求周期，并在线性模式的接缝处混合两侧采样；mirror 使用双倍周期和反射坐标。
- decal 在线性边缘计算连续覆盖，在 nearest 下使用左开右闭的区域；最近邻在整数边界选左方/上方像素，与既有 CPU 路径的方向相同。
- 完全位于图内的区域在原图边缘重建边缘像素，不能因窄裁剪产生额外透明度。区域沿某轴越出原图时，该轴超出原图的 texel 视为透明；另外一轴仍按自身是否越出判断。

这些是显式、可复现的 draft profile，尚未作为 Office/WPS 的所有分数裁剪、边界和默认规则。它也不包含 mipmap、各向异性过滤或媒体帧采样。

## 性能与资源边界

固定 Skia CPU 栅格管线增加一个自有 highp SIMD stage，使用有界 gather 和既有合成路径。采样前先把加载索引限制到有效原图，透明判断不允许形成越界读取。普通线性区域只读取四个 texel；repeat 接缝根据当前 SIMD 批次需要增加邻侧采样。

图片仍借用同一不可变预乘资源，不为裁剪复制或生成中间位图；straight Alpha 输入沿用每个资源一次的预乘归一化。WASM 跨模块内存的上传/返回拷贝仍存在。没有引入运行时着色语言编译器、逐像素 CPU 回调或新外部依赖。私有 shader 有独立类型标识，不冒充 Skia 的 image 类型；它仅在当前同步绘制内使用，不提供序列化工厂。

区域坐标绝对值及投影角点不超过 32768，量化后的宽高至少为 `1/16384` 源像素；资源尺寸、总量和矩阵条件继续服从[图片资源合同](image-raster.md)。Rust 用原始 Q32 值包围矩阵系数量化误差，同时把区域边界误差与可见范围内的重复周期数计入 `coordinateErrorBound`。预算不足则整批拒绝。该界不涵盖逆矩阵/着色器浮点运算、滤波跳变或最终颜色误差，不能当作完整视觉误差保证。

## 组件版本

`mo_skia_abi() == 4` 和解码 ABI 1 保持不变。图片扩展升级为 `mo_skia_images_abi() == 2`，接受 V5 和 V6：

1. 头部仍为 12 字，V6 图片画笔由 10 字增至 14 字，在矩阵之后追加四个 f32 位模式的源区域坐标。
2. 路径、描边、渐变、资源描述和 draw 的编码不变；全部语法与范围在进入 Skia 前验证。
3. 指令上限调整为 2,691,084 字。区域请求输出 profile 为 `skia-8d6d37b-q32-image-domains-srgb-premul-rgba8-v6-draft`，不含区域仍使用 V5 profile。
4. TS 的 `supportsImageDomains` 识别图片 ABI 2；当前薄层仍可使用旧 ABI 1 的整图入口，但在调用前拒绝不支持的 V6，实例可以继续处理合法旧请求。

缓存仍须绑定 profile、指令摘要和完整资源摘要。组件构建源清单记录自有扩展与固定 Skia 修改，历史组件产物单独保留。

## 验证与重现

证据入口：[源区域验证](../reviews/evidence/2026-09-25-image-domain-verification.json)。独立语料由本仓库拥有的 5×4 RGBA8 图片生成，Python `Fraction` 从源请求计算仿射反演、区域映射、平铺、过滤与输出舍入。另有四角极窄区域的同色场不变量，避免仅让实现与同形式参考共同出错。逐像素比较不排除边缘，容许每通道一个 RGBA8 舍入单位。

304 个成功用例和 9 个预检失败分别执行路径及场景入口，共 626 对 Native/WASM 调用；145,920 个独立参考像素最大通道差为 1，24 个同色场用例逐字节符合不变量。旧文字/页面 307 对、近期文字与整图 211 对、图片场景 148 对、来源资源 50 对、编码/分辨率 379 对全部重跑，当前共 1,721 对调用。另有 321 组 Native/WASM/ASan/UBSan 底层检查（含 15 种恶意帧和 V5/V6 整图对比），无诊断。538 项 Rust 测试、严格 Clippy、78 份 Schema/TS 和 9 个通用合同拒绝检查通过。

当前未压缩 Skia WASM 是 2,320,035 字节，比此前解码组件 2,313,369 字节增加 6,666 字节；依赖锁和 PNG/JPEG/zlib 构建不变。它不是完整内核或桌面安装包大小。

局部测量使用 Apple M4 Max / macOS Darwin 25.2.0 / arm64 / Node 23.5.0，1024×768 输出、256×256 不透明图中的整数 192×192 区域，无字体。每项预热 3 次后测量 25 次，组件已加载，逐次校验资源并分配输出。以下为毫秒中位数：

| 模式 | Native 预先复制的紧凑图片 | Native 源区域 | WASM 紧凑图片 | WASM 源区域 |
| --- | ---: | ---: | ---: | ---: |
| clamp / nearest | 2.11 | 2.19 | 16.79 | 19.86 |
| clamp / linear | 3.67 | 4.21 | 19.52 | 32.80 |
| repeat / nearest | 1.96 | 2.74 | 20.94 | 24.70 |
| repeat / linear | 5.19 | 5.11 | 102.45 | 41.55 |

此样本四种模式的输出完全一致，源区域免去预裁剪复制，但并非每种模式都更快。Native 只计 C ABI，WASM 包含 TS 内存分配和拷贝；紧凑图片的预复制成本未计入，资源扫描字节数也不同。测量按紧凑图片、源区域顺序执行，没有控制 JIT/GC 和热状态，不是统计受控的性能门禁，不能外推为整页帧率或产品收益。原始样本、构建和输入摘要绑定于阶段证据。

新组件使用独立 `.codex-work/image-domain/` 目录，固定 PNG/JPEG/zlib 依赖仍取自解码阶段。构建时提供已有固定 Skia 归档；下列命令不联网：

```sh
python3 tools/components/build-skia.py --target native --directory .codex-work/image-domain/component --image-codecs .codex-work/image-codec/deps-native
python3 tools/components/build-skia.py --target wasm --directory .codex-work/image-domain/component --image-codecs .codex-work/image-codec/deps-wasm
python3 tools/components/build-skia.py --target native --sanitize --directory .codex-work/image-domain/component --image-codecs .codex-work/image-codec/deps-asan
export MO_SKIA_LIB_DIR="$PWD/.codex-work/image-domain/component"
```

按[开发检查](development.md)重建 Native Worker / Rust WASM，bindgen 到本阶段 `wasm-node/`，TS 编译到 `ts-raster/`，均不覆盖旧封存目录。随后执行：

```sh
python3 tools/verification/image-domain-fixtures.py .codex-work/image-domain/cases
node tools/verification/image-domain-parity.mjs
node tools/verification/image-domain-components.mjs
node tools/verification/image-domain-benchmark.mjs
```

完整记录还绑定旧页面/文字/场景图片/来源资源/编码解码的当前双端回放、源码与生成合同、日志、原生/WASM/ASan 组件和依赖摘要。ASan/UBSan 与 Native/WASM 一致性分别记录，不将 macOS 不支持的 LeakSanitizer 计为通过。

此阶段没有完成真实来源 PPTX 图片页面、Office/WPS 图片编辑往返、生产 Agent 适配或 Musterwork 产品接入，完整目标保持进行中。
