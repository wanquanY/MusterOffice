# 原生径向渐变几何

2026-09-25：DrawingML `a:path path="circle"` 的局部几何已进入实际 Rust 内核，可通过 Native、WASM 和 CLI 独立查询。输入是真实 PPTX、来源摘要、页面和目标对象；输出包含路径边界、平铺矩形、外圆、内椭圆、焦点、缩放及逐项误差界。原生填充声明、继承来源和页面放置一起保留。

本阶段完成几何求值，尚未接入来源页面的径向像素绘制。固定方向仅保留声明，不代表已实现静止方向绘制。`shape` 渐变、目标 Office/WPS 兼容验收、完整高级内容及 Musterwork 替换门槛保持开放。

## 几何依据与兼容边界

[微软 Open Specifications 的说明](https://learn.microsoft.com/en-us/answers/questions/2248059/non-preset-a-tilerect-behaves-strange-in-case-of-g)描述了外圆和内椭圆，并于 2025-04-30 纠正径向中心应使用实际路径边界。它也确认了自定义 `tileRect` 的应用实现问题。这些资料没有完全定义所有像素的插值方式，不能据此宣称任意焦点均与 Office 一致。

当前明确命名的开发 profile 为 `drawingml-circle-path-bounds-q96-v1-draft`：

1. 在接收对象的局部 EMU 空间编译预设或自定义路径。使用真实曲线轨迹的紧边界，包含有轨迹的多条路径，不把 Bézier 控制点当成曲线边界。只有 moveTo 的路径不贡献边界；描边宽度和效果不参与。现阶段包含 fill-none 路径，仍需目标应用校准。
2. 将 `tileRect` 应用到该边界，要求有误差保证的正宽、高。以所得矩形为外圆的外接依据：圆心取中点，半径取对角线的一半。
3. 将 `fillToRect` 应用于外圆的包围正方形，得到内椭圆的位置和两个半径。允许点、线、外扩及偏移，倒置焦点明确报错。

其中“实际路径边界同时用于 tileRect 和外圆半径”是本项目当前开发策略；公开纠正明确涉及中心，不能把整个策略视为已经过 Office/WPS 校准。非均匀内椭圆、外部焦点、退化轴及平铺的最终插值仍须独立完成。

设每个轴的外部起点和长度为 X、D，内部起点和长度为 X′、D′。焦点表示使用 S=D′/D、P=X′+D′(X′−X)/(D−D′)。[微软对 fillToRect 的解释](https://learn.microsoft.com/en-us/answers/questions/2247174/how-is-attribute-filltorect-evaluated-for-a-gradie)还给出了退化分支。本实现按精确十进制有理数判定 D=D′，此时保留 P=X′；零内部长度同样得到 P=X′。微软说明中的浮点近零阈值没有在此 profile 中模拟，极接近等长时可能产生很远的焦点，该兼容差异明确保留。

## 计算和接口

复用已有 NativePathCompiler 与 `mo_geometry::path_bounds`，把路径转换误差和曲线边界误差传播到 Q96 区间，最后一次转换为 Q32 的 nominal 值及非负绝对误差。误差相对于已求值的导引参数，不涵盖预设公式与目标应用的差异。`focusScale` 是无量纲 Q32，其余几何值为局部 EMU 的 Q32；矩形按 LTRB 排列。

`coordinateTolerance` 是路径/边界计算的输入容差，派生焦点误差可能放大，不能解释成最终像素或颜色误差。该阶段不计算世界变换或最终像素。页面放置独立返回，后续画笔编译使用同一结果。

- Rust 库：`radial_layout::layout_paths`、`layout_background`、`layout_source`。
- 公共 Rust API：`layout_pptx_radial` 与严格 JSON 包装。
- WASM：`layout_pptx_radial(request, sourceBytes)`，不需要塑形/解码/绘制后端。
- CLI：`mo-cli layout-pptx-radial request.json source.pptx`，标准输出返回 JSON，不改写源包。
- 两份新 Schema/生成 TS：`pptx-radial-layout-request` 与 `pptx-radial-layout-response`；此前 80 份 Schema 及其 TS 保持摘要不变。

查询当前只接受对象或背景目标，背景使用页面尺寸。对象引用背景填充时明确诊断其坐标空间要求。来源摘要、页面、目标去重、继承、几何和放置绑定在计算前检查；全查询共享路径及边界预算。任一对象失败或取消，返回错误而不发布部分计划。默认最多 4096 个目标、4,000,000 个边界工作步骤，并沿用路径编译器限制。JSON/WASM 使用既有有界内联包策略；原生库允许宿主提供取消检查。

## 验证与大小

[阶段证据](../reviews/evidence/2026-09-25-radial-layout-verification.json)封存源码、实际构建、输入及输出：

- 609 项 Rust 测试、严格 Clippy、格式、82 份 Schema/TS 检查通过；取消测试覆盖 101 个检查点。
- 29 份自有 PPTX 产生 31 个目标布局。42 对实际 Native/WASM 调用包含上述 29 个成功查询及 13 个错误查询，响应字节一致。
- 原始 XML 的独立 160 位 Decimal 参考核对 27 个目标、513 个数值与误差界，包括二次/三次曲线解析极值。四种复杂预设只做双端一致性验证，没有纳入独立几何参考。
- 29 份正例及 4 份语义负例共 231 个 XML 部件通过官方 XSD。结构有效不代表渐变参数一定可求值。
- 1,116 对已有通用绘图、139 对来源页面及 307 对页面/文字调用保持结果，来源页面的帧和像素不变。CLI 创建、拒绝覆盖、失败不发布继续通过。
- 新接口及来源页面实际请求/响应共 356 次 Schema 验证通过。没有把未运行的 Office/WPS 测试记作通过。

未压缩 Rust WASM 为 **6,396,636 字节**，比上一阶段增加 116,393 字节。固定 C++ WASM 仍为 **2,327,852 字节**，组件和第三方锁定依赖未变。本阶段没有重跑未改动 C++ 的消毒器测试；上一阶段记录保留。上述大小仅是裸模块，不是完整安装包或性能收益测量。

## 复现和后续

使用上一阶段固定 `.codex-work/rect-gradient/component`；新产物写入 `.codex-work/radial-layout`，不得覆盖已封存产物。

```sh
python3 tools/verification/radial-layout-checks.py
python3 tools/verification/radial-layout-reference.py
python3 tools/verification/radial-layout-invalid.py
node tools/verification/radial-layout-parity.mjs
node tools/verification/radial-layout-regressions.mjs
node tools/verification/radial-layout-source-regressions.mjs
node tools/verification/text-page-runtime-regressions.mjs .codex-work/radial-layout .codex-work/rect-gradient/component .codex-work/radial-layout/ts-raster/index.js
python3 tools/verification/contracts.py
python3 tools/verification/radial-layout-evidence.py
```

各命令日志按证据脚本指定名称保存；封存后使用 `--check` 核对。后续实现共享径向场、世界画笔和来源页面预检复用，并用独立像素参考及 Office/WPS 语料校准。完整接入还须按一期能力清单推进，不能把单项渐变进展视为产品替换完成。
