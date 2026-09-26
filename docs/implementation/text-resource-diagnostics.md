# 可行动的文字资源诊断

2026-09-25。[文字页面运行接口](text-page-runtime.md)的字体选择缺失已从通用输入错误改为 `RESOURCE_REQUIRED`，携带明确字族、常规/粗体/斜体/粗斜体要求及来源位置。文字画笔错误已定位到具体段落和 run。此阶段完善同一渲染链的失败与重试合同，没有缩减一期功能范围，也不表示 Musterwork 已可替换。

## 计算层与来源绑定

`mo-text::manifest` 在查找清单映射和样式实例时生成 `FontSelectionFailure`：`style` 是实际计算样式索引，`typeface` 是所需名称，`fontStyle` 是明确样式，`reason` 区分 `unmappedTypeface` 与 `missingStyle`。空名称、超限名称、错误资源摘要/范围、错误名称记录仍按各自输入或资源错误处理，不能假称缺少字体。

`PreparedSourceText` 核对失败索引及所需名称/样式确实属于当前准备计划，然后附上源摘要、对象、段落物理序号和全部匹配的 `uses`。计算样式可能合并多个原生 run；每个 run 的字体声明、脚本、字体槽和主题来源都保留。`run: null` 明确表示插入样式，不能当作前一个可见 run。单段计算与原生框预检/计算共用这条来源绑定，不在宿主重建位置关系。

页面错误在 `error.detail.kind: fontSelection` 下返回完整绑定。共享文本错误也增加可选 `fontSelection`；未涉及此错误的旧响应不新增空字段。TS 从 Rust 生成的 Schema 获取相同结构。诊断来自错误枚举和当前源计划，不解析 `message`；宿主按资源策略提供明确字体清单/字节后重试，核心继续核对身份与实例。没有增加系统字体发现、下载、隐式家族替代或伪造粗斜体。

当前每次返回遇到的第一个缺失选择及其在该段落内的所有原生使用点，不是全稿缺字体清单，也不是完整字体回退、嵌入/许可或资源获取系统。

## 画笔位置与错误分类

原生画笔求值按 run 保留 `{paragraph, run, sourceOrdinal}`，在属性/声明拒绝、缺少画笔以及颜色求值未解决时，通过 `error.paintLocation` 返回。对象位置与原始声明仍分别保留；从工作颜色到页面画笔的检查同样附加实际 run 来源。

位置封装不改变底层取消、预算或来源错误类别。单元测试特别验证嵌套在 run 和对象位置中的取消仍返回 `CANCELLED`。`sourceOrdinal` 对应原始物理 XML 节点；调用者必须结合响应的源摘要/对象上下文使用，不能套用到编辑后的新来源。

下划线等尚未实现的画笔仍明确失败。新增位置不会把未实现内容当作可绘制，也不构成这些样式的功能验收。系统颜色缺失仍保留当前画笔错误码及 `missingSystemColor` 结构化原因；示例宿主据此补充明确颜色上下文。

## 验证结果

全仓 469 项 Rust 测试、严格 Clippy、格式、70 份 Schema 生成检查及 TS 类型检查通过。新增六项测试覆盖类型化字族/样式要求、无组件调用/失效、空与超限名称、合并 run 的完整来源、插入样式、后续 run/未解决颜色位置，以及带位置的取消。

实际 Native CLI/worker 与 Rust WASM＋固定 HarfBuzz/Skia 执行了 12 个新请求：七次明确失败和五次资源/上下文补齐后的成功绘制。字体修复只修改显式清单，不修改 PPTX；常规映射修复及系统颜色补齐后的像素与原样本完全一致。粗体使用原创可变字体中显式选择的 `wght` 实例，验证资源路由，不证明目标应用视觉质量。清单没有斜体实例，斜体插入样式保持未解决，没有用常规实例冒充恢复成功。

另重跑此前 27 个文字页面请求，仅两条旧画笔失败响应新增 `paintLocation`，其他字段和全部成功像素保持一致；再重跑 307 个旧页面/文字请求及 21 个旧像素结果，Native/WASM 均一致。本轮双端请求总数为 346（12＋27＋307）。12 个新请求也经过 CLI 发布链：失败无图像，恢复成功产生与 worker/WASM 相同的图像。

独立 lxml/MCE 参考从真实 PPTX 核对四个画笔 run 的物理序号，以及九个字体使用点的段落位置、字体声明和显式粗/斜体/插入属性；JSON Schema 验证 35 个合法请求、四个非法请求和 39 个响应。这只覆盖已记录的原创、直接字体样本，不等于完整主题字体规则或 Office/WPS 互操作。

未改变 Cargo/pnpm 依赖版本、C/C++ 组件或旧 11 个发行产物。仍无新增产品时延、峰值内存、安装体积、真实字体视觉或目标应用重存/播放验收结论。证据见[文字资源诊断验证](../reviews/evidence/2026-09-25-text-resource-diagnostics-verification.json)。

## 复现与后续

当前 debug Native 和独立目录中的 debug Rust WASM 构建步骤沿用[运行接口](text-page-runtime.md)，输出目录改为 `.codex-work/text-resource-diagnostics/wasm-node`。

```sh
python3 tools/verification/text-resource-fixtures.py
node tools/verification/text-resource-parity.mjs
python3 tools/verification/text-resource-reference.py
node tools/verification/text-page-runtime-regressions.mjs .codex-work/text-resource-diagnostics
```

验证先核对上一阶段冻结输入，再构造新的原生样本；旧报告和样本保持原样。后续继续完善实际文字装饰/复杂布局、真实字体质量与完整高级内容，并连接生产任务、权限、Artifact 和 Musterwork 宿主。
