# 自动启动组的原生事件身份与 WPS 编辑

2026-09-29：修复作者自动启动组在 WPS 动画列表中不可见的问题，已更新独立 Musterwork worktree 的 SDK、导出 Worker 和播放包。正式导出的六方向路径动画可在 WPS 中编辑并保存，再经公共 SDK 和实际产品播放 Worker 验证。完整一期和 PPT 替换验收仍在推进。

2026-09-29 后续：[原生路径缺省行为](native-motion-defaults.md)已通过对照接入省略／`base` 的直线路径，原 WPS 文件保持原字节并通过实际产品播放。以下保留本阶段原始结论与失败记录。

## 根因与实现

在同一个已验证的单击路径样本中，用 WPS 将开始方式改为“与上一动画同时”，保存的条件是 `onBegin + tn`。此前作者编译器生成 `Begin`，Writer 输出 `begin`；Reader 又把 `onBegin/onEnd` 归一为 `Begin/End`，丢失了原生事件身份。只把旧六对象导出中的一个 `evt="begin"` 改为 `evt="onBegin"`，其他 OPC 部件内容保持不变，WPS 就显示了全部六个动画。这个 A/B 结果定位了本次列表问题。

[Microsoft 的实现说明](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/6acf6d2c-6423-4f5e-8a16-6533a20fbd41)区分节点区间边界和指定目标收到的通知。模型现在保留 `Begin/End/OnBegin/OnEnd` 四种身份；原生读写和生成的 JSON Schema／TS 合同一致。当前支持域明确限定为时间节点 `tn`，由内核在其开始／结束时同步发出通知；只在调度监听索引中共用对应边界，不抹除文档中的声明。针对形状的事件、运行时选择器或缺失目标仍不得推断。

作者自动组使用主序列的 `OnBegin` 通知，保留 Never 候选和既有导航语义。没有把它改成无条件零开始，也没有在 Writer 增加按结构猜测事件名称的特例。旧 `After/Begin/End` 保持原生 `begin/end`。新增测试确认“上一步”清除组之后不会消费旧主序列事件自行重播；下一步仍能按原规则重播。

## 验证与产品接入

[可核对证据](../reviews/evidence/2026-09-29-automatic-sequence-verification.json)固定源码、命令、材料和 WPS 对照，保留上阶段失败记录。

- 521 项相关 Rust 测试通过，新增两项检查四种原生事件身份往返及自动组导航重置；既有手写原生事件测试继续核对精确开始／结束时刻。严格 Clippy、格式、合同生成及全部 TS 类型检查通过。
- 四份公共原子创作文稿和四份既有 WPS 对照构成 132 帧；44 个独立时序期望及 44 个 PresentationML 部件校验通过。相同 132 帧分别在真实 WASM Worker 的同步、分步执行中一致。WASM 另完成 4 次原子编辑、4 次逐字节相同的 PPTX 导出，以及 5 类非法输入与 Native 相同的拒绝。
- 新内核正式导出的六个自动路径在 WPS 12.1.22553 中全部显示。将首个时长从 2 秒改为 3 秒，另外五个保留 2 秒，保存后读回六个原生 MotionLine 和 `OnBegin`。正式导出及这份回存文件共有 11 个 PresentationML 根部件通过 XSD。
- 独立产品消费者使用固定公共 SDK 完成两份完整文稿的再导出；24 帧原生参考和 38 帧实际产品播放 Worker 对照通过。159 项产品测试、类型检查和完整桌面前端 Vite 构建通过。

| 材料 | 摘要／实测 |
| --- | --- |
| SDK manifest | `eca5e890c8ca03801e2e127859c26a8567b8e744c6ebe61e1bfe47c841a39778` |
| 播放包 manifest | `98a703f7f63e77d580096bde68c205142010c10b1cd816a1de065f7a9cd615e0` |
| macOS ARM64 导出 Worker | `9d9027c83f7b1b527e313409a98f061d9ff80a818efcaaa9e536ca41010bbb0f` |
| 桌面前端实际复制播放材料 | 42 个文件，18,574,333 字节，未压缩 |
| 原生开发版 Worker | 43,999,856 字节 |

相对[容器缓动阶段](container-easing.md)，桌面播放材料增加 4,753 字节，开发版 Worker 字节数相同。本次未新增依赖或 C/C++ 组件，也未测量安装包、发行版 Worker、FPS、RSS 或启动速度。

## 继续开放的范围

自动组的本次列表与时长编辑问题已修复，不代表任意 WPS／Office 动画互操作通过。此前在 WPS 直接绘制、缺省 `cBhvr.additive` 的路径文件仍明确返回 `MAPPING_NOT_IMPLEMENTED`；没有把缺省值猜成 `repl`。一般容器时钟、其他完整高级内容、PowerPoint、真实 Agent／Electron、多平台、历史迁移及性能／资源／安装包验收继续实施。

产品正式文件只更新两份材料锁及 Cargo SDK 路径，全部产品写入和验证仍在[独立 worktree](product-worktree.md)。默认引擎未切换，没有公开发行、部署或业务数据库提交。核心仍只负责文稿计算与薄接入。
