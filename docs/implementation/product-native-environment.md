# Musterwork 原生导出环境

2026-09-28，在独立产品 worktree 中继续原生接入。本阶段把导出的 renderer、字体字节和公共 SDK `DeliverySettings` 归回产品宿主配置，模型只引用已发现的不可变环境。MusterOffice SDK、字体校验和渲染计算仍是唯一计算实现；没有新增内核权限、存储、页面或业务任务。

## 已实现的调用边界

新增待验收 `device.v52 / artifact.presentation.author@5`。`environment` 接收空对象，返回环境 ContentRef、名称、配置的字体族、默认字体及 renderer profile。`export` 只接收 `draft_id`、最新 `expected` 和 `environment_id`；宿主从真实草稿资源和环境派生 SDK Export action。创建、导入、原子编辑、查询和关闭继续使用原公共 SDK 文档语义。

| 调用 | Agent 提供 | 产品宿主负责 |
| --- | --- | --- |
| environment | 空参数对象 | 固定 renderer 身份、配置和字体字节的不可变引用 |
| begin / apply | 原生文档或操作、已授权资源 ID | 内容解析、Invocation 限额、原草稿事务 |
| export | 草稿、产品版本条件、环境 ID | 字体/资源精确绑定、计算、Artifact 原子发布 |

这是继任合同，不重写 `author@4` 已发布字段。Device 同时注册 @4、@5 的精确处理器；当前活动目录仍是 `device.v50 / author@3`。不能将待验收工具注册等同于默认功能已替换。

## 宿主配置与复用

Device 原有 `office_runtime` 配置新增可选 `environment`：

```text
office_runtime:
  worker: 产品提供的固定 worker 绝对路径
  worker_sha256: 对应 worker SHA-256
  environment:
    name: 环境名称
    delivery: { path: 公共 DeliverySettings JSON 绝对路径, sha256: 文件 SHA-256 }
    font_bundle: { path: 字体 bundle 绝对路径, sha256: 文件 SHA-256 } 或 null
```

以上为字段说明，不是可直接运行的样例。产品负责组件和字体的安装、许可及配置文件生成，模型参数不接受这些路径。启动时对文件种类、路径、读取上限及实际 SHA-256 做检查；字体清单与 bundle 必须同时存在或同时缺省。共享进程配置通过 `Arc` 持有字体字节。配置载入不会扫描系统字体或下载字体。

当前配置输入上限为环境元数据 1 MiB、字体 bundle 32 MiB；仍受原 Invocation 输出预算与计算材料预算约束。这是本接入轮廓的显式限制，不是全量字体分发完成或 RSS/包体承诺。是否存在缺字、字体名称/轴是否匹配由内核在实际使用时检查，发现结果明确标注 `configuration_only_until_kernel_use`。导出错误不得静默改用系统字体。

环境和字体使用现有 Invocation ContentRef/内容端口及普通 Tool 完成日志交付授权。相同调用的准备重试复用内容身份和预算；之后的导出可继续引用已发现的环境，不依赖最新配置名称。重启后宿主配置变化不会把已有环境引用偷偷解析为新字节；当前 worker 不匹配其固定 renderer 时明确拒绝。

## 计算与提交的完整证据

导出准备将环境加入实际读取集和原计算预算。生产者还会从该环境重新派生 ExportSettings，与真正执行的操作逐项核对；字体引用必须与解析输入完全一致。最终候选沿用原证据格式，原事务重查环境及其他依赖仍有效，再原子提交 Artifact、草稿、Tool 和 Ledger；不新增环境数据库或另一套任务。

真实专项覆盖环境准备重试、缺失字体授权、缺失配置、renderer 错配、重启后配置变化、损坏字体拒绝、操作与环境不一致，以及候选生成后撤销环境的原子回滚。两次实际导出的 PPTX 与显式配置的 @4 基准逐字节相同：首版 10,735 字节、第二版 10,729 字节，各 21 个 ZIP 部件，CRC/XML 检查通过。两版文件相同只证明本自有两页样本没有因接口收敛而改变，不外推为 Office/WPS 完整兼容。

本轮 12 项原生 Agent 专项（含已有 7 项回归）和 4 项 Device 宿主专项通过，共 16 项不同测试；桥接、Artifact、tool-executor、device-runtime 四个生产库的严格 Clippy（`--lib --no-deps -- -D warnings`）通过。

具体命令、源码范围、测试和失败记录见[本阶段证据](../reviews/evidence/2026-09-28-product-native-environment-verification.json)。不把构建耗时当作渲染性能，也不把 worker 开发文件大小当作桌面发行包体积。

## Skill 与剩余工作

产品新增原生 `musterwork-presentation-authoring-v8/SKILL.md` 草案，描述上述实际协议、两个版本条件、诊断与质量声明；保留历史 HTML Skill。它尚未加入活动 Skill release 或证明真实模型会话可用，不能称为已发布/已自动发现。通用 Codex Skill 校验器不支持产品现有的 `when_to_use`、`required_tools` 等控制字段；保留产品协议，不删除这些字段来获得通用校验通过。后续需要产品原生 Skill 包和可选择的接入验收组合。

原生模板、已交付 Artifact 的继续编辑入口、字体分发、实际 Agent/桌面会话、播放及 Shared Home/PostgreSQL 仍待对接；完整高级内容、Office/WPS 编辑往返、Native/WASM 与 P00/E0–E3 保持开放。工作继续使用当前独立 worktree，原 Musterwork 主工作区留给用户。
