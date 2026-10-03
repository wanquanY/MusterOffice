# 编辑计算运行时与快照恢复

状态：计算增量已实现并完成下列专项验证；产品接入、固定集成发行与平台验收仍独立进行。

## 公开入口

组装后的公共入口 `createPlaybackRuntime` 现在返回 `createEditorPage()`。它用已初始化、已校验的
同一 WASM 模块创建独立 `PresentationEditorPage`，宿主不需要导入私有 binding。每页有自己的
当前 view 和释放责任；关闭一页不关闭其他页或播放 owner。显式传入共享 raster/shaping ports，
并在页生命周期结束时调用 `close()`。一个 Worker 只初始化一次运行时，调度和硬取消仍由宿主负责。

`editor.restore(SnapshotRecord)` 经过 Rust `Snapshot::restore` 验证格式、文稿和语义摘要，保留已保存
修订。普通 `initialize(document)` 继续创建初始修订，不能用于恢复持久 checkpoint。
恢复沿用 Initialized 响应，不制造事务、历史或新修订；随后编辑/撤销仍走普通候选计算。
合法快照不是权限凭证，宿主必须先验证其会话、文稿和材料来源，再进行权威重放与持久化。

## 已执行验证

- 真实 Node Worker 只导入组装公共入口：拒绝第二次初始化，独立页 owner、关闭后重建、保留原页的
  inspection/typed rejection、单元格能力与编辑、重绘像素变化、字素移动、普通撤销及 transfer 脱离均通过。
- 编辑客户端 20 项通过，包括真实 Native/WASM 恢复同一保存修订、篡改拒绝、恢复后继续撤销。
- Kernel 101 项通过，严格全目标 Clippy、TS 生成/类型和独立公共入口 TS consumer 通过。
- 当前修改 Rust 文件格式通过；已构建 schema 工具的合同一致性检查通过。

复现入口：`tools/verification/editor-runtime-bundle.mjs`、`editor-runtime-consumer.mts`、
`editor-client-tests.mjs`。公共包、运行输入与输出摘要见
[证据](../reviews/evidence/2026-10-03-editor-runtime.json)。其中 bundle 脚本默认使用忽略的
`.codex-work/editor-runtime/playback-sdk`，类型 consumer 也使用此组装路径；先按播放 SDK 工具组装
实际包，不能用源码 mock 替代。consumer 使用 `tsc --strict --noEmit --target es2022 --module nodenext
--moduleResolution nodenext --lib es2022,dom`。

Office 共享主工作区另有 creation metadata/图表开发，未由本任务修改或提交。后续重新编译 schema
工具时遇到其图表接口过渡中的编译错误；原已构建工具检查通过不代表当前混合工作树重编译通过。
证据保存实际成功产物和失败日志摘要，后续固定 SDK 必须重新建立一致源码、二进制与产品资格。
本次未宣称浏览器/Electron、跨平台、Office/WPS、发行或性能目标通过。
