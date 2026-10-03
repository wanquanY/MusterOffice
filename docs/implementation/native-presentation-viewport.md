# 原生放映视口切换

日期：2026-10-03。状态：内核、生成合同和 WASM 薄客户端已实现并完成本机专项验证；
Musterwork 的固定开发发行、窗口/DPR 接入与双端验收仍待完成。
本实现属于 [原生交互编辑方案](native-presentation-editor.md) 的播放基础，
不改变 [ADR 0009](../decisions/0009-product-editor-computation.md) 的内核/产品职责。

## 合同与状态

Author 和 Source session 都增加 `resize` 操作，载荷为原 `binding`、
`expectedViewportRevision` 和完整 `RasterViewport`，成功返回 `resized` 及当前 `info`。
`info.viewportRevision` 从 0 开始，每次成功递增，u32 耗尽明确拒绝，不回绕。
旧版本请求返回 `VIEWPORT_CONFLICT`；播放 generation、来源 revision 和 slide 均不改变。

视口的 origin、scale、像素上限与精度预算由 `RasterViewport::validate` 统一准入，
实际页面仍需通过几何/图片/文字合成的原精度验证。Source preflight 直接调用此准入，
不再为了验证空视口构造临时 raster 帧。有效视口并不承诺所有未来动画姿态都可渲染。

`planId` 继续标识实现配置和内容；A → B → A 可以恢复相同内容身份。
每次视口提交另外替换私有 owner epoch，完成阶段同时核对 epoch、plan 与 binding，
因此最初 A 的未完成帧不能在恢复 A 后提交。组件错误导致的隔离标记在拒绝陈旧帧时仍保留。
`advance` 继续用于播放代次切换，会清理时间缓存；resize 不调用它，也不重放输入历史。

客户端的 `Frame.viewportRevision` 在独占播放租期内捕获。对于已经完成计算、
进入宿主消息队列的帧，产品必须继续核验当前 owner/视口版本后才显示。
仅比较尺寸、planId 或动画 generation 不能解决 A → B → A 的晚到消息。

## 资源与原子提交

Author 在原文稿与 base placements 上验证新视口，不克隆整份文稿或替换时间采样器。
失败恢复原视口；成功仅更新视口和内容身份。

Source resize 再次接收确切原 PPTX 字节与 decoder，不接收字体字节，不需要 shaping backend。
包读取与来源摘要重新核验，但复用既有 SourceIndex、表格索引、时间图和已排版的局部文字路径，
不重新导入页面 XML、排版文字或上传字体。

图片资源沿用原资源准入与去重引擎，按照新视口重新确定解码需求。
静态图片继续按视口采样，动画会改变缩放/方向的图片及其 group 后代继续使用原精度策略。
多个对象共享图片时仍取最严格需求，既不把旧小图直接拉大，也不统一改成全尺寸解码。
新资源集合按原 visibility union 验证，包含动画可能显示的对象。

候选持有原 Owner 的独占借用；丢弃即撤销，不能提交到其他 Owner。
内核先用候选资源验证页面并计算新 plan 身份，再做无失败步骤的整体替换。
取消、错误来源、decoder 失败、无效视口均保留原视口、资源、草稿外部身份和时间状态。
准备期间旧/新资源可以同时存活，未完成帧也可能持有旧图片；本实现未声称峰值内存或缩放延迟达标。

## 薄接入

- `AuthorPlayback.resize(viewport)`。
- `SourcePlayback.resize(viewport, {source, decoder})`；宿主需要保留或重新提供已核验的 PPTX。
- WASM `PptxPlaybackSession.resize(request, source, decoder)`。
- Native 原 framed session 传输 Source 字节，字体通道保持空。

所有方法同步计算。已有 stepped execution 必须先 take/close；独占期内 resize 返回 `BUSY`。
成功后宿主用同一 `at/history` 取新帧；时钟、暂停、DPR 映射、合并 resize 请求和全屏手势由产品负责。
ShapingPort 同步补齐现有 register/unregister 与 registered batch 接口，防止类型允许缺失的运行时组件。
同时将既有 KernelRequest 中较大的历史事务/文字命令改为 Box，消除严格 Clippy 告警；JSON 合同未变化。

## 验证

本机 macOS arm64、Rust 1.92.0、wasm-bindgen 0.2.126。只使用仓库锁定组件和本地自有 fixture。
Native Skia 使用 `.codex-work/font-fallback-20261003/skia`，当前源码摘要经 build script 核验。
WASM Skia/PNG/JPEG 从当前锁定源码重建至 `.codex-work/presentation-viewport`；未使用摘要过期的默认缓存。

| 范围 | 结果 |
|---|---|
| Rust 核心与指定回归 | 214 项通过：Kernel lib 5、Author session 14、Source session 8、Compile lib 100、采样图片 6、Source playback 3、Source resource page 7、Raster lib 71 |
| JS 生命周期/错误接入 | 25 项通过，包括 resize 拒绝回执、帧尺寸、独占重入与 stepped 租期 |
| 真实 Native/WASM | 3 组 author/source/image/text/click 输入，9 组配对帧逐字段与像素一致；6 次新视口与重新准备同视口像素一致；resize 字体调用为 0 |
| 准入与回滚 | 所有取消检查点、无效尺寸/比例/精度/origin、错误来源、decoder 失败、丢弃候选、A → B → A 旧帧拒绝；静态图片需求从 400×400 增至 800×800 |
| 工程门禁 | 相关四个 crate 严格 lib Clippy、Schema/生成类型、TS 编译、Native worker 与 WASM release 构建通过 |

实际对照脚本为 [`playback-viewport-parity.mjs`](../../tools/verification/playback-viewport-parity.mjs)。
它核验 fixture pin，在执行前后检查代码/输入摘要，记录每帧摘要、viewport revision 和时间采样诊断。
最终本地产物：`.codex-work/presentation-viewport/parity-04/report.json`。
相关日志为 `.codex-work/viewport-{regression,client-tests,parity,clippy,contract-check,types}.log`。
旧路径 parity-01 是失败记录，parity-02/03 是中间结果，不作为最终代码绑定证据。

复现对照前先按开发指南构建上述当前组件、worker、Rust WASM/bindgen 及三个 TS adapter；再运行：

```sh
node tools/verification/playback-viewport-parity.mjs \
  .codex-work/presentation-viewport/new-result \
  .codex-work/wasm-node target/debug/mo-raster-worker \
  .codex-work/presentation-viewport/skia .codex-work/harfbuzz/release \
  .codex-work/presentation-viewport/adapters \
  .codex-work/playback-render/product.json .codex-work/source-playback/product.json
```

此处不是完整编辑器、导航反馈、独立全屏窗口、Windows/Linux、浏览器或性能验收；
固定产品发行与相应验收仍由 Musterwork 专项计划继续推进。
