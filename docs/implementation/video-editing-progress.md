# 视频创作与剪辑实施记录

2026-10-06 · 目标执行中，尚未达到整体验收。

## 固定基线与开发边界

- 已发布 [MusterOffice v0.1.0](https://github.com/wanquanY/MusterOffice/releases/tag/v0.1.0)，固定源码 `a4901290c6ccf6660aabe38d42d2dd650428219f`；对应 CI 44 项步骤通过，发布前源码摘要一致，三个公开附件的远端 SHA-256 与本地一致。这是稳定源码基线，未提升产品组件频道或声明全平台预编译包。
- 用户已明确授权稳定版发布和完整视频方案实施。MusterOffice 主工作区功能分支 `codex/video-native-workbench`；产品保留现有独立 worktree。生产部署、Musterwork 公开发布不属于本目标。
- Musterwork 原 worktree 仍有可复用的 Admin、生成协议/计费和多模态请求绑定；已保存原 HEAD、工作区/暂存区二进制补丁和 42 个未跟踪文件的完整本地快照。保留该 worktree，按公共操作合同替换旧 CE.SDK 路径，不整库丢弃有效代码。
- 当前开发决定见 [ADR 0012](../decisions/0012-agent-video-domain.md)。纯内核候选不代表宿主保存成功，测试辅助绑定不作为产品 SDK 发布。

## 权威方案与工作包

[产品与架构](../design/video-editing.md) · [技术合同](../design/video-editing/technical-spec.md) · [完整验收](../design/video-editing/acceptance.md)。完整目标不缩减；本文件只记实际进展。

| 工作包 | 当前状态 | 退出条件 |
| --- | --- | --- |
| W1 视频领域与合同 | 基础已实现，未全部完成 | 独立模型、精确时间、受限解析与校验、Native/WASM 公共合同向量；补全效果、字幕、分析和工程包模型 |
| W2 媒体与合成 | 待实施 | 原生/浏览器媒体与合成、字体/色彩/音频及真实输出 |
| W3 编辑与历史 | 原子事务基础已实现，未全部完成 | 全部编辑操作、候选、冲突与有条件逆操作及增量索引 |
| W4 产品执行 | 待实施新内核接入 | Device/Browser Host、恢复、权限、任务和不可变组件消费 |
| W5 创作与交互 | 保留原 worktree，未完成替换 | 整理现有生成/Admin/计费、技能、人机共享编辑与消息卡片 |
| W6 完整验收 | 仅有下列内核基础证据 | 完整验收矩阵与可供用户实际操作的本地环境 |

## 已实现的计算基础

### 精确时间

`mo-time` 从原 PPT 时间线抽取有界精确有理数运算。PPT 保留原合同与正除数约束的薄适配，视频可用反向映射所需的有符号除法。时间转换区分精确可表达与显式量化；不能表达时返回错误，量化返回舍入残差。支持四种明确舍入策略、负时间和分数帧率的绝对帧边界计算。

### 工程模型

`mo-video-model` 已有独立工程、序列、视频/音频/图形/字幕轨、片段、媒体/字体资源、流选择、嵌套序列、联动组，以及线性分段/倒放/定格时间映射。基础效果包括恒定变换、增益和显式相位范围的淡入淡出。剪开片段时平移效果相位，不重新开始淡入淡出。

校验覆盖身份与所有权、丢失引用、流/轨兼容性、半开区间重叠、源素材范围、时间映射、关联组、嵌套 DAG 深度和循环。JSON 拒绝未知字段及重复键，具有字节/节点/流/锚点/诊断/算术预算与取消检查。资源字节、URL、绝对路径和业务任务不进入模型。

顶层对象表使用有序持久树共享快照。10 万对象的计数测试证明：复制集合不复制记录，修改一个记录只复制该记录；这不是完整工程交互性能或内存峰值的验收。组件版本和许可见[固定来源记录](../../components/persistent-collections/component.json)。

### 原子编辑与逆操作

`mo-video-edit` 提供快照、`prepare` 和 `prepare_inverse`，实现标题、序列/轨道/资源管理、插入、删除、移动、分割、裁切、波纹插入/裁切、slip、roll、slide、基础效果/来源替换、关联/解除关联，以及覆盖、区间删除和显式音频策略的变速。波纹轨道集合、关联策略与分割后的新身份必须显式给出。区间/变速规则见[详细合同](video-editing-transactions.md)。

所有操作先在共享结构的候选中执行，完成后统一验证、计算摘要、读写集合、条件逆操作和预览失效范围。错误或取消不改变基线。逆操作按对象及关联依赖检查冲突，允许保留其他轨道的独立修改；新建关联、解除关联后成员被修改等情况不能被撤销静默覆盖。嵌套序列依赖和 solo 导致的其他轨道变化进入失效传播。

持久事务、幂等、版本号/epoch、outbox 和存储仍由产品实现。当前完整校验、摘要与失效传播是工程规模线性工作；不可把局部持久树更新的复杂度当成整个事务的性能。

### 合同与验证入口

`contracts/generated/video-*.schema.json` 从相同 Rust 类型生成，包含工程、编辑请求、逆操作和校验结果。它们是开发中的合同，还未独立发行或接入真实产品。

`tools/mo-video-conformance` 是专用测试驱动。相同 Rust 代码编为 Native 和 WASM，比较完整候选文档、摘要、错误、读写集合、失效范围和逆操作；不是另写一套 TypeScript 语义。默认 CI 已加入长序列测试和视频跨端一致性步骤。

## 当前验证证据

| 检查 | 实际结果 | 不能推断的结论 |
| --- | --- | --- |
| 原 PPT 时间线 | 168 项既有测试通过共享算术适配 | 不等于新的媒体渲染已实现 |
| Rust 工作区回归 | 一次完整运行 1,675 passed、28 ignored；后续视频限定修正另跑专项 | ignored 中需要独立环境/worker 的检查不伪装通过；不是所有 CI 分组重跑 |
| 最新时间/模型/事务专项 | 55 passed、1 个长序列测试默认 ignored 并另外执行 | 不代表所有最终编辑功能已实现 |
| 既有 MCP 纯计算入口 | 17 项测试通过，独立 Cargo.lock 已纳入共享 mo-time | 未增加视频 MCP 工具，legacy-host/HTTP 组合本次未全部重跑 |
| 精确时间生成测试 | 4,096 组有理算术性质；107,892 帧的 30000/1001 时间及 48k 音频累计边界 | 这是样本调度数学，未播放或编码真实一小时视频 |
| 事务长序列 | 固定种子 100,000 次；47,422 成功、52,578 拒绝、47,422 次逆操作精确恢复（已扩展区间与变速语料） | 语料覆盖当前操作子集，不足以关闭完整 SEM-06 |
| 原子失败/取消 | 每个观察到的取消检查点拒绝候选，基线不变；中途失败、锁轨、缺素材、超预算验证通过 | 未检验数据库断电和业务 outbox |
| Native/WASM 实际运行 | macOS arm64 Native 与 Node v23.5.0 的 WASM，25 步：20 次成功、5 次拒绝，结果完全一致；重复键两端均拒绝 | 不等于浏览器媒体、WebGPU/WebCodecs、Windows/Linux 原生或产品验收 |
| 跨端 fixture 摘要 | `568babf4de9b90e46fca80bee7377ef437167f9ae1053254355c736fd008be5a` | 绑定本次基础语料，不是最终媒体档案 |
| 跨端结果摘要 | `3bc203a14d17be214aec45b5c9588f221296ab9b6229f28c8518adb5b126195a` | 同上 |
| 代码与合同 | strict Clippy、WASM 编译和既有合同检查通过；新增视频 schema 使用相同生成/检查入口 | 未发布新版 SDK、组件频道或 UI |

长序列复现：

```sh
cargo test --release --offline --locked -p mo-video-edit --test generated_sequences -- --ignored --nocapture
```

跨端复现使用 `tools/verification/current.py --groups wasm`（包含原有 PPT wasm 门禁），需按既有构建文档提供固定 native 组件与匹配的 wasm-bindgen。视频测试脚本为 `tools/verification/video-domain-parity.mjs`；它要求本次构建的 Native 程序和 Node wasm-bindgen 输出，不接受用一份预录结果冒充其中一端。

## 明确尚未完成

- 当前已支持锚点线性变速；完整曲线变速/反查、转场、关键帧、字幕词锚点、分析与互操作等最终语义仍待补全。音频策略已保存，实际保调/重采样等处理仍待实现。
- 没有新内核的真实解码、seek、音频混音、GPU 合成或容器输出；FFmpeg、浏览器 I/O 与相关组件尚未接入。
- 生产 SDK/MCP/CLI、Device/Browser Host、Agent 技能、新时间线 UI、保存恢复和项目包尚未接入新内核。
- CE.SDK 仍存在于产品旧 worktree，未宣称依赖退役；旧 scene 数据须审计、迁移或保留可导出的原件，不能仅换 engineId。
- 完整功能、媒体质量、性能、体积、多平台与真实 provider 计费没有完成验收；当前不能交付为“用户已可验收全部视频功能”。

## 本地材料

发布核验材料：忽略目录 `.codex-work/stable-release-20261006/`。产品旧实现快照：其 worktree 的 `.codex-work/video-native-migration-20261006/`。

测试记录：`.codex-work/video-foundation-workspace-tests-02.log`、`video-domain-tests-final.log`、`video-edit-generated-100k-02.log`、`video-domain-conformance.json`、`video-foundation-clippy-workspace.log`、`video-final-component-clippy.log`、`video-schema-check.log`。区间编辑增量记录：`video-range-tests-final.log`、`video-range-generated-100k.log`、`video-range-conformance.json`、`video-range-clippy-final.log`、`video-range-schema-check.log`。这些本地记录不进入发行依赖。
