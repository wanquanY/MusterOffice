# 已交付文稿的公共播放输入

2026-09-28。针对产品已有 PPTX、字体和交付设置却缺少直接播放输入的问题，在内核提供同源计算入口。产品无需重写字体绑定、页面顺序、OPC 部件解析或尺寸换算。权限、读取、Worker、时钟、界面与结果展示继续由接入产品拥有。

## 实现

`ReceivedDelivery::playback_inputs(width, cancelled)` 复用交付检查已经核对的 PPTX、字体、设置和页面索引，返回按文稿顺序排列的请求及精确资产身份。它不重新打开资源，不渲染像素、不创建播放会话。字体清单只返回一次；页面路径来自实际 OPC 关系，不能用页序号拼接。

Native SDK 已通过 `delivery` 模块暴露这一入口。便携协议增加 `DeliveryPlaybackRequest/Response`、WASM `prepare_delivery_playback`、TS `WasmPlayback.prepareDeliveryInputs` 和 CLI `delivery-playback`。便携输入沿原交付检查核对全部字节与上下文，沿用 128 MiB 二进制输入限制；原生接入可复用已检查对象，避免打包或重复检查。

输出的文档 `revision` 保留编辑版本来源；现有来源播放 owner 的 `binding.revision` 使用 `source.sha256`，两者不可混用。接入方把指定页的请求、公共资源渲染 profile、共享字体清单和对应 PPTX/字体字节交给原 `prepareSource`，随后复用现有同步或分步采样、取消和释放接口。样例 Worker 已加入派发，不引入第二套 Agent 协议。

输入检查成功不代表全部动画受支持；实际播放准备仍执行时间线与资源语义检查，也不升级交付的质量声明。

## 验证

[机器证据](../reviews/evidence/2026-09-28-delivery-playback-verification.json)，SHA-256：`04efa67853f6a705d0e19d71c334177f833d4cbc690d3a46d45e7f54cfcd2dd9`。

- 28 项相关 Rust 测试及 21 项现有 TS 播放生命周期测试通过。交付及 kernel API 的全部目标严格 Clippy、schema/TS 和格式检查通过。
- 10 组新入口 Native/WASM 对照包括三份有效交付、六类拒绝及拒绝后复用。现有交付检查的 36 组 Native/WASM 回归通过。
- 三份交付包含固定自有样本和[实际 Device Agent Loop](product-native-model-flow.md)提交的两个版本；PPTX 与产品保存结果逐字节一致。原生单次渲染、真实 Node Worker 中的 WASM 保留采样、WASM 分步采样对照 12 帧，全部像素一致；保留采样的字体上传和文字组件调用均为零。
- 实际公共包有 66 个文件，解包总计 18,077,546 字节；三份代码模块被显式固定，当前重建的 WASM/glue 与测试包逐字节一致。这是本地开发输入，`releaseCleared: false`，不是 Musterwork 安装包尺寸或性能验收。

前两次本地 Rust 测试分别暴露测试断言使用不支持相等比较的诊断枚举、以及夹具预期遗漏 OPC 路径前导 `/`；均修正断言后通过。首次 codegen 命令缺少输出目录，补齐后生成与检查通过。失败记录保留，没有改变生产规则来迎合测试。

## 剩余工作

这一阶段验证了小型静态文稿的来源采样，未验收完整动画、转场、音视频、SmartArt、公式或 Office/WPS。它没有修改 Musterwork 的固定 SDK/worker、默认引擎或生产查看器。下一步在独立产品 worktree 中接入经固定的公共 WASM SDK、原内容读取权限、Worker 生命周期和已有查看器；避免把全份交付检查或字体上传放进逐帧路径。

Musterwork 主工作区不写入。完整桌面链路、历史迁移、跨平台、全部质量、性能和发行体积门禁仍开放，完整替换目标保持进行中。
