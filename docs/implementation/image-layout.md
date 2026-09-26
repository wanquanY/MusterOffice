# 原生图片局部布局

状态：DRAFT，库级实现与真实文件验证。此阶段把 PPTX 中的图片填充、解码图片和形状尺寸连接起来，输出可追溯的局部布局；尚未把结果作为完整页面中的图片绘制，也未提供新增的产品命令或 Agent 工具。

## 计算与原生含义

`mo-presentation-compile::source_image_layout::layout` 消费已解析的 `EffectiveImageFill`、解码信息和形状尺寸。它保留原生源记录，不将百分比提前舍入到整数像素或 EMU。

- `srcRect` 的四个百分比相对已归一化图片的宽高，得到源像素区域；正值内缩、负值外扩。
- stretch 把该区域映射到 `fillRect`。后者的百分比相对形状边界，输出额外的硬裁剪要求；采样边界不能代替这项裁剪。
- tile 先用物理像素尺寸及 `sx/sy` 求一个 tile 的大小，再按九种对齐位置放置，最后应用 `tx/ty`。平铺翻转分别转成两个轴的 mirror/repeat。
- 显式非零 `dpi` 使用每英寸 914,400 EMU 换算；为零时使用解码器提供的已归一化物理像素尺寸。stretch 不需要物理 DPI；tile 遇到缺失、零值或冲突的分辨率会明确返回问题，不读取宿主 DPI。

边界定义依据 Microsoft 的 [SourceRectangle](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.sourcerectangle?view=openxml-3.0.1)、[FillRectangle](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.fillrectangle?view=openxml-3.0.1)、[Tile](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.tile?view=openxml-3.0.1) 和 [BlipFill](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.blipfill?view=openxml-3.0.1) 说明。解码后轴的选择、负 tile scale 的应用行为和各产品缺省 DPI 仍需要目标应用核对；当前只接受正 tile scale，零或负值明确拒绝。

## 数值与布局结果

百分比与带单位坐标共用有界十进制解析。`mm/cm/in/pt/pc/pi` 使用精确有理换算，输入数值词法最多 256 字符。既有文字数值入口复用提取后的精确比值函数，保留其原先的 Q32 舍入行为。

图片计算在 Q96 向外区间上完成，最终才量化到 Q32。结果同时记录源矩形每条边、目标矩形每条边、源像素原点和每像素位移的误差界。源矩形误差以源像素为单位，其余使用局部 EMU。空、反向、精度不足或数值越界会失败，不输出替代的整图范围。极窄区域除法放大的误差继续保留；这不是颜色、滤波或全部页面误差保证。

结果是局部布局，**不是世界坐标 ImageBrush**。`rotateWithShape` 原样保留，页面编译器还需选择图片的实际变换，处理组合、翻转和不随形状旋转的情形；不能把形状变换无条件套上去。stretch 的额外裁剪也必须与形状路径求交。低层采样能力见[图片源区域](image-domain.md)。

## 来源与资源绑定

`layout_source` 消费同源 `SourceIndex`、原生图片资源目录和不透明的 `DecodedImage` 结果。它重新核对源包摘要、表面、资源数量和每项编码摘要，限制累计解码像素为 64 MiB，并复用批量来源放置查询。目录和索引由可信的核心源解析产生，不能由宿主反序列化数据冒充；`DecodedImage` 只能由经过验证的解码路径构造。

图片本体、形状图片填充和背景分别保持原生 target。重复图片引用复用目录资源索引，目标顺序不变。背景使用页面尺寸，对象使用原生放置结果的局部尺寸。根组合没有单独的图片绘制矩形，当前明确拒绝；继承到子对象的图片则按该对象的目标查询。任一目标未解析、资源绑定不符、布局失败或取消都会使整个目标批次失败，不能取得部分结果。

这层不加载文件或 URL，不复制像素，不调用解码器，也不建立缓存或产品提交。文件读取与实际解码由调用者负责；验证宿主示例逐项提取并解码真实包内图片。64 MiB 是输入结果像素总量检查，不是解码过程或进程 RSS 上限。

## 验证

证据入口：[原生图片布局验证](../reviews/evidence/2026-09-25-image-layout-verification.json)。

102 份本仓库生成的真实 PPTX 含原生图片、形状图片填充和背景三种目标，共享一个真实 PNG 资源。96 个成功请求输出 288 个目标布局；6 个请求拒绝空矩形、缺失/零分辨率、零/负平铺比例。所有 204 个 presentation/slide XML 部件通过固定官方 ECMA Transitional XSD 校验；这不代替 Office/WPS 打开或编辑测试。

Native 验证宿主执行实际包解析、来源关系查询、编码资源提取、固定 C++ PNG 解码及 Rust 局部布局。独立 Python `Fraction` 参考核对 3,456 个数值及逐项误差界，并额外限制误差界不能通过任意放大来通过验证。覆盖四类源区域、三类填充矩形、九种 tile 对齐、四种平铺翻转、DPI 覆盖和非方形物理像素。形状的旋转/翻转声明保留在结果中，当前没有声称已应用到图片绘制。

9 类来源/资源绑定拒绝和 69 个实际取消检查点验证整批失败。7 项新增 Rust 测试覆盖局部计算、极窄区域、单位、分辨率选择及取消。当前新增布局尚未导出为 WASM 产品运行入口；双端回归仅证明既有文字/页面链路未变，不能证明这项新增布局已完成跨端运行验收。

545 项 workspace Rust 测试、严格 Clippy、格式及既有 78 份 Schema/TS 检查通过。原生与实际 WASM 重跑 518 对既有文字/页面/整图请求，响应和成功像素保持不变。固定 C++ 组件、解码依赖及历史发行产物未改变；本阶段没有新增性能、体积或 sanitizer 测量。

## 重现

沿用[当前组件](development.md)与 `.codex-work/image-domain/component/` 固定 Native 库，不修改此前封存产物。验证示例属于开发宿主；新增 Cargo 引用只有已有 workspace 包，没有引入新的第三方运行依赖。

```sh
export MO_SKIA_LIB_DIR="$PWD/.codex-work/image-domain/component"
cargo test -p mo-presentation-compile --locked
cargo build -p mo-skia-sys --example source_image_layout --locked
python3 tools/verification/image-layout-reference.py
```

脚本将自有 PPTX、请求、实际结果和参考记录写入 `.codex-work/image-layout/`。后续仍需将局部矩阵及误差绑定到世界图片画笔、落实 fillRect 硬裁剪、在同一个页面编译器中保留所有内容顺序，并完成实际 Native/WASM 页面像素、Office/WPS 和 Musterwork 产品验收。
