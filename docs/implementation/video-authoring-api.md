# 视频工程计算接口与独立 WASM

2026-10-06 · 开发合同，尚未发行或完成产品接入。上位：[技术合同](../design/video-editing/technical-spec.md)、[实施追踪](video-editing-progress.md)。

## 职责与实际接口

`mo-video-operations` 组合现有模型、编辑 reducer 和编译索引；不另建一套编辑规则。`mo-video-wasm` 只依赖这套视频计算合同，与 PPT、字体组件、原生 codec 和业务存储分离。`packages/video-client` 是生成类型上的 JSON/对象生命周期薄封装，适合在宿主持有的 Worker 中运行。

| 接口 | 实际行为 |
| --- | --- |
| create / open | 校验工程和预算；打开时重算内容摘要，拒绝损坏快照 |
| snapshot / summary | 可保存的独立工程；精确时长、帧数、样本数与对象计数 |
| list | 分页读取序列、轨道、片段、资源、联动组；可按半开时间范围或片段筛选 |
| prepare | 返回编辑候选、读写集合、条件逆操作及失效范围；不修改旧对象 |
| prepareInverse | 按指定内容摘要执行现有条件逆操作 |
| frame | 固定摘要和绝对帧序号上的同源计算计划 |
| resources | 所有声明资源的身份、内容摘要和长度；不授予读取权 |
| close / free | 显式释放不可变工程句柄；TS 重复 close 安全，释放后调用拒绝 |

`musteroffice.video.operations/1-draft` 表示当前计算档案。`musteroffice.video.snapshot/1` 包含 `format/documentDigest/document`，不包含宿主 revision、epoch、账号、文件路径、任务或账单。生成 schema 和 TypeScript 合同由既有 codegen 维护。

## 保存与冲突

`MutationCandidate` 是计算结果，不是提交回执。产品需先在权限范围内解析资源，再在同一持久事务中核对 head、epoch、幂等键及请求摘要，原子写入快照、历史、回执和 outbox。只有该事务成功，UI 和 Agent 才能获得“已保存”的正式回执。此 crate 不拥有宿主数据库，也不把广播事件当作提交。

分页 cursor 绑定 `documentDigest + queryDigest + offset`；换版本或筛选条件后不能接着使用旧 cursor。offset 指向底层对象位置，避免每页从头重扫已消费行。过滤查询仍可能扫描当前页之后的对象，并受访问预算约束；没有声称范围查询已全部使用区间索引。

## 预算、错误与取消

快照输入默认 34 MiB，输出默认 64 MiB；查询另有既有节点、访问、深度和输出预算；默认每页最多 256 条。候选快照同时受可重新打开的快照预算约束。错误包含分类、操作序号、对象身份和有界校验诊断，不回传原始输入或内部堆栈。

Native 调用接收取消检查函数。独立 WASM 当前是同步调用，宿主须在专属 Worker 执行并用终止 Worker 实现抢占取消；尚未接入产品 Worker 调度，不能宣称已完成浏览器取消链路。

打开工程会经过编辑快照和编译校验，保留编译索引用于后续读取。当前重新打开仍有线性验证/摘要开销，不以持久树共享或保留索引宣称整个事务已经增量化。

## 开发 SDK 与验证

`tools/video-sdk/build.py` 离线构建独立 WASM、web ESM 和薄 TS 客户端，记录源码输入、依赖树、工具版本与逐文件摘要。输出新目录；外部 manifest SHA-256 必须与完整文件清单及内容一致。目录内增加、修改文件或符号链接均拒绝。

SDK 入口 `createVideoRuntime(WebAssembly.Module)` 只接收宿主明确编译的模块，没有默认 URL、下载或隐式加载。每个宿主 Worker 初始化一次；失败后也要求新 Worker。产物 `private: true`、`releaseCleared: false`；它是本地接入用的不可变开发包，未替代正式发行的 SBOM、许可清单和平台验收。

复现入口：

```sh
python3 tools/verification/current.py --groups video-authoring,contracts,typescript \
  --output .codex-work/video-authoring-check \
  --bindgen /explicit/path/to/wasm-bindgen \
  --stretch-dir /explicit/path/to/prepared-native-dsp
```

测试涵盖真实 Native/WASM 编辑候选、逆操作、分页、帧计划、资源闭包和错误一致；实际打包 ESM 的加载、编辑、释放和重复初始化；宿主字节预算、损坏快照、旧 cursor、未知/重复 JSON 字段与包完整性拒绝。原生 conformance 测试驱动含其他视频测试模块，因此构建该驱动需要既有 DSP；独立 WASM SDK 自身没有 DSP 或媒体依赖。

本地证据存放 `.codex-work/video-authoring-gate-final/` 和 `.codex-work/video-authoring-clippy-final.log`。这些证据不代表浏览器媒体设备、真实用户存储/断电恢复、时间线 UI、完整视频效果或整体验收已经完成。
