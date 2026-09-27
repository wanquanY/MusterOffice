# 可恢复的绘图组件状态

2026-09-27 · 针对[浏览器取消实测](browser-playback-sdk.md)暴露的长同步调用，先将 Skia 适配层改为显式持有状态的计算对象。此阶段提供准备阶段与完整绘制操作之间的让出边界；**尚未实现单个 Skia primitive 内部让出，也尚未接入 Rust 播放 sampler 的异步编译—绘制—收尾流程**。取消 P95 与强终止门禁保持开放。

## 计算接口与边界

[C ABI](../../components/skia/mo_skia.h)增加可选 execution extension 1：`begin`、`step`、`take`、`drop`。原有同步 raster/raster_images 也驱动同一状态机完成计算；不会维护两套绘制语义。已有 V4–V12 输入、状态码、完整像素与 profile 不变，新增接口不修改文稿合同。

begin 独立验证完整输入，再建立计算对象。任务持有画笔、图片、路径、文稿裁剪栈、前缀快照、输出表面和渐变数值预算；步进不会清空累计预算或重复解释文稿。每个工作单元推进一项准备、最多 4096 条路径指令、一项绘制准备或一个完整 Skia 绘制操作。`step(workUnits)` 接受 1–4096 个工作单元；这不是像素预算或时间上限。大对象扫描转换、clip、快照复制、验证及清理仍有同步临界区，必须继续拆解和测量。

C 调用方必须在 drop 前保持请求/图片借用不可变且有效。句柄属于单个组件、单个所有者，不跨进程序列化或接受网络提供的指针。没有后台线程、隐式时钟、文件、网络、全局任务注册表或业务存储。只有 complete 后 take 才能转移完整帧，且只能成功一次；失败和提前 drop 不交付部分像素。实际组件 trap/分配失败仍按原合同销毁其 Worker/进程，不能把正常任务取消解释为可复用失效组件。

[TS owner](../../packages/raster-component/src/execution.ts)通过 `RasterComponent.beginRaster` 创建。输入只复制到组件私有内存一次，随后宿主原数组可以 detach；owner 保留对应分配直到完成或关闭。活跃任务期间同一组件拒绝第二项绘图，step/take/close 拒绝重入；正常 close 释放任务和借用后允许健康组件继续工作。任务不会自行开启 Promise、计时器或帧队列。

宿主可在两步之间处理取消和调度，例如在 Worker 中：

```ts
const started = raster.beginRaster(frameWords, imageBytes);
if (started.status !== 0) return started;
const task = started.execution;
try {
  for (;;) {
    if (hostCancelled()) return {cancelled: true};
    const next = task.step(1);
    if (next.status !== 0) return next;
    if (next.complete) return task.take();
    await hostYield(); // 接入产品提供，让该 Worker 的消息队列可以前进。
  }
} finally {
  task.close();
}
```

这只是组件执行示意，不是完整 PPT 播放 SDK 已异步化。现有 Rust→组件回调仍是同步 ABI，不能在回调里放入 Promise 假装暂停 Rust 调用栈；后续须显式拆开 frame preparation、组件执行和 Rust 验证/完成，并保持版本、计划及资源身份关联。

## 画质反例与正确方向

曾实验用整数裁剪矩形对同一对象分块绘制。即使全局坐标不变、块不重叠，Skia 解析式抗锯齿仍受到局部裁剪上下文影响。自有 `source/new/group-inherited-control` 用例出现 **1402 个通道字节不同**，没有通过逐字节检查。失败日志、旧算法源码与输出保留在[阶段证据](../reviews/evidence/2026-09-27-stepped-raster-verification.json)。没有采用像素修正、增加容差或更换基准。

最终实现撤销这种裁剪分块，完整保留 Skia 的 draw、clip、stroke、颜色和合成语义。在 primitive 内部继续细分时，必须保存真实扫描转换/着色状态，或提供能中断长临界区的明确协作接口；不重新裁剪几何来伪装同一计算。当前状态机是这项工作的生命周期基础，不是将最终目标降为仅对象间取消。

## 验证范围

固定旧语料共 554 组，包含渐变/精度预算、图片、剪裁、合成、来源对象和无效帧。原生用 3 种 workUnits 完成 1662 次状态码与完整像素对照；WASM 的同步入口和 3 种步进方式也逐字节核对。步进不改变数值预算失败结果。ASan/UBSan 另外执行 2216 次，含提前取消后重新准备；不是额外的 2216 组独立语料，macOS 本次没有 LeakSanitizer 验收。

WASM 的 TS owner 共创建并释放 1611 个实际 C++ 句柄，桥接输入分配最终归零。验证了输入提前 detach、未完成 take、重复 take、完成后的 step、无效工作批量、任务独占、多个准备/绘制边界的提前关闭，以及健康组件真实复用。桥接计数不代表 Skia 内部所有分配计数或进程 RSS 已回落。

重新构建的是维护的 C++ 适配层；上游 Skia/codec 固定归档按历史摘要验证并复用，没有声称重新编译了全部上游。原生链接检查、已有原生 SDK/CLI/兼容宿主的播放与导出集成，以及 Chrome 中 71 帧播放回归分别执行，详细命令、构建输入和日志以阶段证据为准。Chrome 这部分仍调用同步播放 sampler，不能算作新 step 接口的端到端播放验收。

## 剩余工作

状态机尚不提供跨版本序列化 checkpoint。完整文稿导入/解析/布局/塑形/导出仍须独立实现可让出与取消；单个复杂 primitive、快照和准备过程的最坏临界区必须控制。宿主收到取消、内核停止、对象释放与实际物理内存回收分别取证，不以本轮用例运行时间承诺 SLA、60 FPS 或整体性能提升。

完整高级内容、Rust/WASM 高层接口、Musterwork Viewer/Player/Artifact 与历史迁移、Office/WPS 互操作和完整替换验收按原目标推进。
