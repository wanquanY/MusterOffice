# 放映步骤与页面边界反馈

2026-10-03。内核已实现并固定接入 Musterwork 共享播放器，独立窗口导航验收另行推进。
这是[原生交互编辑计算](native-presentation-editor.md)的播放能力，遵循
[ADR 0009](../decisions/0009-product-editor-computation.md)的职责边界。

## 输入与回执

`InputEvent::PresentationStep { direction }` 表示一次产品放映手势。
Next 同时匹配全局 Click 和 Next 条件，Previous 仅匹配全局 Previous 条件。
编译期合并有序监听表，执行时使用既有时间线的同一份事件前状态和条件仲裁；
没有第二套动画调度器。对象目标的点击/导航仍由原始 `Click` / `Navigation` 输入表达。

`FrameState.presentationStep` 返回采样前缀内最后一个放映步骤的序号、精确时间、方向和结果：

- `consumed`：至少一个当前可用条件接纳事件，包括延迟执行的开始、结束或序列导航。
  接纳不要求立即改变像素；后续事件取消延迟动作也不追溯改写该结果。
- `pageBoundary { entry: "initial" }`：没有可用条件接纳，宿主可以沿该方向离开当前页。
  当前合同规定相邻页以新的页面 owner、零时间和空输入历史进入，包括上一页。
  不对无限时间线猜测末帧，不合成隐式点击或动画终态。

序列导航的接纳和执行共用内核游标决策；到达边界后不再排入一个延迟的空导航，
以免它在后来的 Previous 重置之后错误激活内容。已接纳的延迟导航执行时仍核验其所属激活。
MainSequence 的 Previous 先撤回已激活的点击组；通用 sequence 保留自身导航语义。
原始输入的调度行为不变。

回执由所在 FrameState 的 session、revision、generation 和摘要绑定。
它只保存最后一项，不随历史增长额外生成反馈数组。没有放映步骤时省略字段，
原始输入的既有线协议和帧摘要保持不变。时钟帧、resize 帧和后续原始事件可以携带
同一旧回执；宿主必须核对本次已提交步骤的序号、方向、时间及 owner 身份，不能再次执行翻页。
缓存键包含完整有效事件前缀，回看较早时间不会返回后来步骤的回执。

## 验证

新增 11 项明确结果断言：空白页、顺序推进与页内撤回、相同时刻有序输入、全局/目标路由、
延迟接纳、重复条件、关闭作用域、结束条件、并行条件、原始输入之后的旧回执，以及
所有取消点、旧 generation 和不合法未来事件。现有时间线回归同时通过。

真实 Native/WASM 构建使用当前 Rust 内核和锁定 Skia/HarfBuzz。
扩展现有 [`playback-viewport-parity.mjs`](../../tools/verification/playback-viewport-parity.mjs)
的可选 `presentation-step` 模式，保留原有模式并复用实际 Native framed transport：
3 组 author/source/image/text/click 材料，33 组帧逐字段及像素一致，
21 次步骤核验精确事件身份，6 个新视口独立准备对照，resize 字体调用为零。
最终证据 `.codex-work/presentation-step/parity-02/report.json`；
parity-01 是 Node 模块类型配置失败记录，不作为成功证据。

时间线累计 168 项测试通过（既有全量运行加最后补充的 3 项）。原始事件模式另外完成
9 组 Native/WASM 配对和 6 次新视口对照，证据为 `legacy-parity-01/report.json`。
Schema 与 TypeScript 类型已重新生成，TS 编译、28 项播放客户端测试、严格时间线、Kernel/WASM lib Clippy 通过。
Author/Source session、原调用入口和 Delivery 另外 32 项回归通过。
这不是浏览器全屏、跨页转场、性能、外部 Office 互操作或整个编辑器验收。

## 固定开发发行

本地完整发行 `0.1.0-dev.step.20261003.1` 绑定 27 个实际 registry 包、Native export worker 和 WASM/TS，
release SHA-256 为 `172e3ce1b55e6705ec8bb64896a675552129a88d385ac86d887a7dad745b9a12`。
原生 worker SHA-256 为 `984c80b4ac0e7f8407c15e71a115a72fc181b8215293d3ff8bc0b05e3e653821`。
独立消费者实际编译、导出和检查通过，证据 `.codex-work/presentation-step/release-check/report.json`。
只声明 macOS arm64 的本机开发验证，未进行远端组件发布。
