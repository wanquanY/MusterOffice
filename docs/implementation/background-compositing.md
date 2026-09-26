# 共享合成与背景区域重绘

状态：共享 Rust 路径/场景、原生与 WASM CPU 组件已实现有界画面快照及 Source/SourceOver 合成；来源页面的开发 profile 已接入背景区域恢复。**背景透明度和平铺存在应用侧差异，尚未通过 Office/WPS 验收。** 本阶段不代表完整演示内核或 Musterwork 替换条件达成。[阶段证据](../reviews/evidence/2026-09-25-background-compositing-verification.json)绑定最终源码、构建和运行结果。

## 语义与职责

ECMA-376 Part 1 §19.3.1.43 将 `useBgFill` 定义为使用形状正后方的幻灯片背景区域。[PowerPoint 的 Background 接口](https://learn.microsoft.com/en-us/office/vba/api/powerpoint.fillformat.background)也区分背景填充、透明填充和向形状应用同样的填充。这支持页面坐标中的背景区域语义；文档示例没有解决所有透明度和图片采样细节。

当前开发策略保存实际背景绘制完成后的像素，用形状轮廓恢复对应区域。它是对“背景表面”的明确实现解释，**不是已经由目标 Office/WPS 证明的透明度规则**。下文独立参考验证该策略的实现；应用差异仍独立保留，不能用内部测试代替互操作验收。

绘制核心新增两项通用能力，均不依赖 PPTX：

- `Brush::Snapshot { after_draws }`：读取执行指定数量绘制后的完整视口像素。JSON 为 `{"kind":"snapshot","afterDraws":1}`。零表示视口初始清屏；正数表示该数量的绘制已完成。引用值不得超过消费它的绘制序号。
- `PathDraw.blend` / `PathInstance.blend`：`sourceOver` 为缺省且序列化时省略；`source` 在路径及裁剪覆盖范围内替换目标。画笔来源与合成方式独立，快照也可使用 SourceOver。

快照使用设备坐标，与消费形状的平移、场景变换及视口原点重定位无关。先前绘制顺序属于引用的含义，场景降低保持每个 instance 对应一个 draw。未来若重排、合批或删除绘制，必须同时重新绑定捕获位置；不能把序号当作可独立缓存的资源 ID。

来源页面仍持有原生对象、几何、填充关系和资源声明。背景在母版、版式及当前页前景之前绘制，随后记录捕获位置。`useBgFill` 消费这一结果，不使用形状自己的尺寸再次铺图，不重复查询或解码背景资源。纯色且完全不透明的整页背景可直接使用同色画笔；来源页面预检要求视口精确覆盖整页，这一优化不适用于任意局部视口。

捕获的是本次绘制的临时状态，不导出到 PPTX，不替换原生文字或形状，不生成用于规避可编辑性要求的截图对象。背景本身不受支持时，仍在页面预检阶段失败。

## 固定组件与预算

固定 Skia 提交、图像解码依赖和许可证锁均未改变。C++ 先完整验证帧及预算，再执行绘制；每个不同前缀复制一次像素，使用拥有独立内存的 SkData/SkImage 及最近邻设备坐标采样。后续输出绘制不能修改已捕获画面。形状和裁剪共同作用一次，不采用“清除后再叠加”两次抗锯齿操作。

| 项目 | 本阶段界限 |
| --- | --- |
| 不同捕获位置 | 最多 64 个，按序去重 |
| 所有捕获像素之和 | 最多 64 MiB |
| 输出像素 | 沿用独立的 64 MiB 上限 |
| V8 帧 | 最多 2,854,990 个 u32 |
| 无快照的 Source 绘制 | 不分配捕获像素 |
| 无新增合成的旧请求 | 保留原 V4–V7 布局及省略字段 |

`RasterWork.compositing` 报告 `captures`、`capturedBytes`、`snapshotDraws`、`sourceDraws`。`capturedBytes` 是本次完整捕获的像素复制量；不包含输出、图片、JS/FFI 拷贝或 Skia 临时内存，**不是 RSS 上限**。当前保留捕获直到本次绘制结束；没有实现区域捕获或存活区间复用，也没有把它们当成已有性能收益。

新增可选 `mo_skia_compositing_abi() == 1` 与 TS `supportsCompositing`，普通 ABI 4、图片 ABI 2、裁剪 ABI 1 不变。旧组件在进入 V8 前被拒绝，仍可处理其原有请求。V8 包含 V7 的图片/裁剪字段及新增的第 14 个 header word（捕获数量）；裁剪表后是严格递增的捕获前缀，随后每个 draw 增为 8 words，末字为 blend（0=SourceOver，1=Source）。画笔索引依次引用渐变、图片和快照，零仍为纯色。

在实际 WASM 内存压力测试中，保留 128 MiB 分配后仍可生成 64 MiB 无捕获输出；加入 64 MiB 捕获会超过当前 256 MiB 线性内存限制，组件失效且不发布部分像素。宿主必须销毁失效 Worker。C++/Skia 某些分配可能触发不可恢复陷阱，不能承诺所有分配失败都返回可继续使用的普通错误码。

## 验证结果与应用差异

- 577 项 Rust 测试、严格 Clippy、rustfmt、80 份 Schema 与生成 TS 检查通过。新增 4 项净测试；旧“背景图片必须拒绝”测试改为验证一次解码和正确合成，原半透明背景测试也已转为正向检查。
- 34 类共享画笔/路径/描边/裁剪/巨大原点用例及控制、拒绝用例形成 97 对公共 Native/WASM 调用。矩形区域核对 6,144 像素，半透明/透明纯色控制核对 12,288 个完整像素（包含抗锯齿边缘），逐字节一致。
- 10 类来源页面各有背景及轮廓控制，共 30 个原生 PPTX，210 个 XML 部件通过官方 XSD。8 类整数对齐的来源用例用独立 XML、PNG 与 alpha 计算核对 960,000 个完整像素；该参考不读取内核布局、画笔或快照计划。
- 来源页面共 86 对公共调用，其中 70 对成功。此前 56 项来源请求中 54 项元数据和像素不变，背景图片与半透明背景两个原拒绝项进入新开发策略。背景图片只解码一次；多个窗口复用同一捕获。
- 190 组底层 Native/WASM/ASan/UBSan 执行通过，包括 20 个语法/资源预算拒绝和 100 个旧 V7 帧；未运行 LeakSanitizer。旧组件能力拒绝、真实堆压力失效与无部分结果发布均已验证。
- 726 对既有图片/裁剪公共请求、307 对既有页面/文字公共请求重跑；旧帧、元数据和像素保持一致。路径、场景和来源页面 CLI 均验证新文件输出、拒绝覆盖及失败不生成像素文件。

LibreOffice 26.2.0.3 对十份原生来源文件完成导入与 PDF 转换。比较首张 400×300 页面；内核预乘 alpha 输出先合成到白色底。完全不透明背景、无填充背景及隐藏窗口的恒色内部区域相同，但边缘仍有差异。其余 **7 类**有恒色内部差异：

| 用例 | 内部差异像素 |
| --- | ---: |
| 半透明红色 / 透明视口 | 36,556 |
| 半透明红色 / 白色视口 | 36,556 |
| 带拉伸区域裁剪的图片背景 | 17,860 |
| 透明图片背景 | 6,958 |
| 多个重叠窗口 | 14,072 |
| 旋转椭圆窗口 | 2,771 |
| 平铺图片背景 | 10,197 |

这里“内部”要求两端各自的 3×3 邻域恒色，仅用于定位差异，不是通用视觉验收指标。例如半透明窗口在 LibreOffice 内部像素为 RGB `(127,85,0)`，当前策略合成到白色后为 `(255,127,127)`；透明图片区域在该应用中保留绿色前景，当前策略恢复背景透明区域。平铺也存在采样布局差异。必须用相同原生文件取得 Office/WPS 结果，确认透明度、背景坐标及平铺规则后才能关闭兼容性门槛；不认定哪一方天然正确，也不隐藏或放宽这些差异。

此前组合填充中的根组合、版式继承和翻转差异仍见[组合图片记录](group-image-inheritance.md)。当前尚缺完整原生画笔/文字/效果、动画转场、媒体、图表、SmartArt、公式及生产 Agent/Musterwork 链路，继续按完整一期目标实现。

## 体积与复现

单个未压缩 Skia WASM 为 2,323,627 字节，比旧裁剪组件增加 1,427 字节；Rust WASM 为 6,145,116 字节，比组合图片阶段增加 13,969 字节。未新增第三方依赖。这不是完整内核、宿主安装包或运行时内存的测量；本阶段没有完整性能、RSS 或 Musterwork 打包收益数据。

固定组件按 [Skia 构建说明](skia-component.md)重建至 `.codex-work/compositing/component`，分别使用既有 native/wasm/asan 图像编解码依赖。随后运行：

```sh
python3 tools/verification/compositing-checks.py
python3 tools/verification/contracts.py
python3 tools/verification/compositing-reference.py
node tools/verification/compositing-parity.mjs
node tools/verification/compositing-source-parity.mjs
node tools/verification/compositing-components.mjs
node tools/verification/compositing-regressions.mjs
node tools/verification/text-page-runtime-regressions.mjs .codex-work/compositing .codex-work/compositing/component .codex-work/compositing/ts-raster/index.js
python3 tools/verification/compositing-observe.py --soffice /Applications/LibreOffice.app/Contents/MacOS/soffice
python3 tools/verification/compositing-evidence.py
```

应用观察脚本需要显式的 LibreOffice 可执行文件和具备 PyMuPDF 的 Python。封存后不要在原阶段目录重建或覆盖证据；后续阶段使用新输出目录并重新绑定源码、构建及运行结果。
