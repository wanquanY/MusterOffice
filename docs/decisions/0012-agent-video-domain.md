# ADR 0012：视频剪辑作为第二个 Agent 办公领域

2026-10-06 · Accepted：产品目标、仓库边界与开发授权。实现及验收分别记录。

## 用户决定

用户要求以最终方案实现面向 Agent 的原生视频剪辑内核，作为 MusterOffice 的第二个办公场景；先完成开闭源研究，再形成 Markdown 产品、架构、技术与验收方案。随后明确要求先发布 MusterOffice 当前代码的稳定版本，再建立目标持续实施整个方案，直至可以在本地开发环境测试验收。

- MusterOffice 在主工作区功能分支开发，保持既有 PR 治理。
- Musterwork 使用本次视频工作 worktree。旧实现有用的部分保留；不因更换剪辑内核丢弃已有 Admin、生成、计费和多模态接入。
- 桌面和 Web 均以用户设备本地处理为默认；不自动部署服务端剪辑算力。
- 产品 UI、权限、保存、任务、模型配置和计费属于 Musterwork。时间、剪辑语义、校验、编译及媒体计算属于 MusterOffice。
- 当前完整视频方案不受 ADR 0008 的演示文稿基础需求收敛限制；不以局部演示代替本次完整目标。

## 稳定基线

已发布 [v0.1.0](https://github.com/wanquanY/MusterOffice/releases/tag/v0.1.0)，固定源码 `a4901290c6ccf6660aabe38d42d2dd650428219f`。这是经过当前 CI 的稳定源码回滚点，不宣称所有平台已提供预编译包，也没有切换 Musterwork 的组件频道。新视频代码不进入这一不可变标签。

## 已落地的基础决定

- `mo-time` 承载精确有理数中间计算，PPT 通过兼容适配层使用，视频独立依赖它；不创建第二套时间算术。
- `mo-video-model` 使用可移植领域数据，不包含绝对路径、URL、权限、任务或业务 head。
- 视频对象集合使用固定 `rpds 1.2.1` 的有序持久树，保持普通 JSON map 合同；复制快照共享结构，修改时复制树路径与被改记录。直接及新增传递依赖来源、锁定摘要与许可见 [组件清单](../../components/persistent-collections/component.json)。没有复制第三方编辑器领域代码。
- `mo-video-edit` 只有纯计算的候选与有条件逆操作。宿主持有 CAS、epoch、幂等回执与 outbox；不能把 `PreparedEdit` 当成保存成功。
- 当前 prepare 的完整校验、摘要与失效范围传播仍扫描工程；持久树的局部更新复杂度不代表整个事务已经达到交互性能目标。后续以真实工程基准决定增量索引与缓存，不隐藏这部分成本。

## 授权与实际状态

完整权威方案：[产品/架构](../design/video-editing.md)、[技术规格](../design/video-editing/technical-spec.md)、[验收](../design/video-editing/acceptance.md)。实际源码、测试与缺口见[实施记录](../implementation/video-editing-progress.md)。

开发授权和本次稳定源码发布不代表视频方案已验收、允许生产部署，或允许公开发布 Musterwork。新的媒体组件仍需各自固定来源、版本、许可及验证证据；选择方向不等于所有平台已经实测。
