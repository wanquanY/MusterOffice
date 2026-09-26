# 组合图片填充的属性继承

状态：受支持的图片填充已通过 `grpFill` 进入同一来源页面引擎，覆盖库、Native worker、WASM 和 CLI。完整 PPT、Office/WPS 与 Musterwork 替换验收仍未完成。本阶段修正了[来源资源页面](source-resource-page.md)对组合跳转的过宽拒绝条件；[验证证据](../reviews/evidence/2026-09-25-group-image-verification.json)绑定实际源码、构建与运行结果。

后续：[背景合成](background-compositing.md)已实现共享快照和背景恢复的开发策略，替代本阶段描述的背景前置拒绝；透明度、平铺及本页列出的组合兼容性差异仍未验收。下文保留本阶段证据时点。

## 语义与实现

ECMA-376 Part 1 §20.1.8.35 将 `grpFill` 定义为继承组合的填充属性，见[微软的 GroupFill 说明](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.groupfill?view=openxml-3.0.1)。§19.3.1.23 定义组合属性与个体属性的优先关系，其父节点包括 `grpSp` 与 `spTree`，见[GroupShapeProperties](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.presentation.groupshapeproperties?view=openxml-3.0.1)。这些条款没有提供目标应用完整的图片坐标数值规则。

依据上述属性继承定义，以及本阶段显式组合的应用观察，当前 draft 计算采用每个接收形状自己的尺寸、位置、方向和几何轮廓：先完成属性继承，再执行既有局部图片布局和世界画笔编译。不将组合边框用作每个子形状的图片边框，不新增组合位图，也不通过预渲染组合图片替代原生内容。两个并排的形状分别显示各自的完整图片；根组合与跨版式继承采用同一属性模型，其目标应用差异仍列在下文。

来源关系与绘制位置保持独立。`SourceImageBinding` 继续保留声明部件、有效属性的来源和完整 `redirects`；资源按声明部件中的关系解析。接收形状可以位于幻灯片上，图片却声明于对应版式。布局仍使用该幻灯片形状的实际尺寸，不把版式占位符的位置或根组合的位置搬到幻灯片上。

`source_resource_page::resources::prepare` 现在仅对包含背景跳转的对象图片保留 `fillSpace` 前置诊断。`useBgFill` 要呈现形状后方的背景区域，仍需要独立背景重绘与透明合成；它和 `grpFill` 的属性继承不是同一项语义。图片 `rotWithShape=false`、缺失资源和未由宿主提供的外链继续在首次解码之前失败。

多个接收者继续复用已校验的编码/解码资源，每页相同内容只解码一次。本阶段没有新增缓存层、画笔算法、公共 JSON 字段、第三方依赖或 C++ ABI；Rust/Schema/TS 的公共合同不变。

## 实际验证

- 573 项 Rust 测试、严格 Clippy、格式、80 份 Schema 和 TS 检查通过。新增三项测试覆盖实际来源继承/绘制和整页资源前置检查。
- 十组来源与显式填充对照，共二十份实际 PPTX；涵盖兄弟形状、不同尺寸/翻转、根组合、裁剪、平铺、嵌套旋转、最近组合覆盖、版式占位符、接收形状旋转与图片双填充。三份额外资源错误语料使新语料合计二十三份，161 个 XML 部件通过固定官方 XSD。
- 56 组公共 Native/WASM 调用通过，其中 38 组成功。十组继承/显式来源在两端均逐像素一致。上一阶段全部 33 个请求重跑，32 个元数据和像素完全不变，原 `group-fill` 请求从明确不支持转为实际绘制成功。CLI 创建、拒绝覆盖和失败不发布通过。
- 独立参考从自有 2×2 PNG、显式对照的原始 XML 尺寸/翻转/裁剪计算预期颜色，核对五个页面的 544,960 个内部像素；不使用内核编译计划。参考排除了形状、裁剪、纹素和抗锯齿边界。

## 应用观察与未关闭差异

LibreOffice 26.2.0.3 导入上述二十份文件并输出 PDF，观察第一张页面的 400×300 像素；原始语料保留第二张页面，此次不对它做视觉验收。应用内的十组继承/显式对照中八组像素完全一致，根组合两组存在差异。以下两种比较各自记录，不能混为“应用兼容通过”：

| 比较范围 | 实际观察 |
| --- | --- |
| LibreOffice 内部继承与显式对照 | `root` 不同 60,100 像素；`layout-placeholder` 不同 11,325 像素；其余八组相同 |
| 内核独立内部像素参考与 LibreOffice | `siblings` 和 `stretch-clip` 一致；`root` 不同 55,296 像素；`layout-placeholder` 不同 9,798 像素；`different-receivers` 不同 12,328 像素 |

后一项 `different-receivers` 的差异位于水平翻转形状内部，设备像素边界为 `[212, 52, 307, 197]`。显式填充和继承填充在 LibreOffice 中相同，并不证明其结果与当前内核的翻转规则相同。根组合继承与形状图片翻转均保留为目标应用核对项；现有证据不足以判断 Office/WPS 的最终规则，也不据此给单一应用定性为错误。

这些差异不会通过复制图片到每个形状、压平来源或丢弃填充来隐藏。后续应使用相同文件获取目标 Office/WPS 的绘制及保存重开证据，再决定原生规则或明确的兼容 profile。渐变/图案页面画笔、背景重绘、完整文字、效果和高级内容仍继续按完整一期目标推进。

本阶段没有完整性能、RSS、安装包或新增 sanitizer 测量。固定组件/依赖未变和单次图片解码不能外推为桌面端体积或性能收益。

## 复现

使用仓库已记录的固定 Skia/HarfBuzz 组件和 TS 适配层。构建与实际执行分别进行，所有新产物保存在 `.codex-work/group-image`：

```sh
python3 tools/verification/group-image-checks.py
python3 tools/verification/group-image-fixtures.py
node tools/verification/group-image-parity.mjs
# 在具有 PyMuPDF 的验证 Python 环境中运行，显式选择应用。
python3 tools/verification/group-image-observe.py --soffice /Applications/LibreOffice.app/Contents/MacOS/soffice
python3 tools/verification/group-image-reference.py
python3 tools/verification/group-image-evidence.py
```

最后一项复核冻结证据；源码或输入改变时应创建新阶段证据，不覆写本阶段记录。
