# 原生节点事件、缺省延迟与产品组件升级

2026-09-28：完成显式节点 `begin/end` 事件和缺省延迟的读取，作者模型的节点依赖改为导出 `begin/end`。Musterwork 的独立 worktree 已同步更新 Rust SDK、实际导出 Worker 和 WASM 播放包。完整一期目标继续进行，当前结果不代表 PPT 全量替换验收。

## 语义与实现

PowerPoint 区分节点开始／结束与事件目标收到 `onBegin/onEnd`；缺省 `delay` 按零处理。依据为微软的 [ST_TLTriggerEvent 实现说明](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/6acf6d2c-6423-4f5e-8a16-6533a20fbd41)和 [cond 实现说明](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/4e694ff4-c1d6-437e-9c0f-99bdd419c427)。

条件解析拆为独立模块，开始、结束和序列导航共享相同的延迟读取。时间中性的外层容器也接受省略的零延迟，不因此增加额外时钟或丢弃事件门控。`begin/end` 必须有明确的 `tn` 节点引用；既有 `onBegin/onEnd + tn` 的受限读取保留。没有把针对形状的生命周期事件、运行时子节点选择器或缺失目标归为同一语义。

作者模型 `After` 表示明确的节点生命周期依赖，导出使用 `begin/end`。它依然引用原节点 ID；不会把容器结束替换成子行为结束。源包保留能力不变，未实现的条件仍给出诊断。

独立 XSD 校验还发现：微软说明中的 `none` 不能直接当作合法的 `evt` 枚举值。早期实验曾按该措辞接受显式 `none`，此实现与实验包均已被后续版本取代；最终读取器拒绝该值，正式导出不生成它。新的样本生成器先校验全部 PresentationML 部件，再接受渲染结果。缺省事件与显式非法枚举保持区别。

## WPS 回存调查

使用自有六矩形文件，分别把同一动画的开始条件设为 `end → 叶节点`、`end → 预设容器`、`onEnd → 预设容器`。WPS Office 12.1.22553 打开并另存后，三份文件均保留事件名称与延迟，却删除了 `tn` 引用。按已有授权短暂前台操作并恢复原应用；没有编辑用户文稿。

这排除了“仅改事件拼写或引用到预设容器即可解决这组回存问题”的假设，尚未证明缺失目标具有可恢复的隐式语义。内核对三份回存文件均返回 `MAPPING_NOT_IMPLEMENTED`，不猜成上一动画，也不降为固定延迟。**该结构的 WPS 编辑回存兼容性仍未解决**，下一步需对照 WPS 原生作者生成的触发分组、预设与行为层级。上述调查不代表 Microsoft Office 或其他事件结构的结论。

## 验证与接入

- 三个相关 Rust 模块共 **350 项测试**通过，严格 Clippy 通过。新测试使用手写原生 XML，校验精确开始／结束边界、点击目标、结束条件、向后采样及非法输入拒绝。
- 六份自有 PPTX 的 **30 个 PresentationML 部件**通过 ECMA XSD 校验。80 个原生采样帧通过独立有理数时序断言及作者／导出／缺省值对照；WASM 同步和分步各通过 80 帧，397 个既有跨端采样保持一致。此一致性仍使用同一计算内核，不替代传统应用验证。
- 独立产品消费者通过固定 SDK 和实际导出 Worker 生成两份交付；28 帧原生参考在产品真实播放 Worker 中通过，共核对 42 帧（含非法输入后保持状态、重新选择页面及重播）。此前实际发布的两版样本另保持 8 帧一致。
- 产品准备脚本 18 项、客户端 114 项、Vue 查看器 7 项、Rust 接入库 19 项，合计 **158 项测试**通过；Vue TypeScript 和完整桌面前端 Vite 构建通过。旧的大 chunk 提示保留。证据扫描首次误包含 Vitest 缓存，后按明确源码范围重新运行并确认无源码变化。

可复现样本工具为 [native-event-fixtures.py](../../tools/verification/native-event-fixtures.py)；产品交付验证使用 [product-navigation-fixtures.py](../../tools/verification/product-navigation-fixtures.py)，现在可以指定单调采样时刻，覆盖延迟依赖真正开始后的状态。[阶段证据](../reviews/evidence/2026-09-28-native-node-events-verification.json)保留成功材料、失败实验和当前限制。

| 当前产品材料 | SHA-256 |
| --- | --- |
| Rust SDK 清单 | `1f15b1ce3b622b20ce741351f6694cb19a902e0799426d19f9f1ca71916031ea` |
| macOS ARM64 实际导出 Worker | `20f986d8aedeb25b4f82645f4855bf95c156dd335f365ab6eebe41e27b3187be` |
| WASM 播放包清单 | `fec79599ba9e7d376bfdbd53810b4385eeb4d4aee15359b30fc8097bd4c20315` |

桌面前端输出 42 个播放 SDK 文件，未压缩合计 **18,163,275 字节**，比上一显隐阶段增加 519 字节。导出 Worker 为 **43,181,440 字节的开发构建**。这只是固定材料大小，不是安装包、运行内存、启动速度或完整功能实现后的估计。

全部产品修改、构建与测试均在 `musterwork-musteroffice / codex/musteroffice-integration`；原主工作区留给用户。新增交付没有提交至产品数据库，本轮也没有新增完整 Agent／Electron 会话验收。权限、持久存储和面向人的交互仍归接入产品。

完整动画、转场、媒体、SmartArt、公式、复杂文稿质量、Office/WPS 编辑往返、跨平台、历史迁移、性能／资源／安装包和全部替换门禁继续按一期目标实施。

2026-09-29 后续校准：WPS 自动启动组使用 `onBegin + tn`；仅将此前六对象导出的 `begin` 改为 `onBegin` 就恢复了全部动画列表。模型现在分别保留四种原生节点事件身份，自动作者序列改用节点通知；旧 `After/Begin/End` 导出不变。此前将显式 `onBegin/onEnd + tn` 归一到 `Begin/End` 的读取现已修正。上面的早期失败证据保留，其任意事件／目标兼容性结论不因此扩大。
