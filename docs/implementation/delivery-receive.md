# 交付包接收与共享字节验证

本阶段为 SDK/产品接收方增加共享 Rust 验证入口，避免产品自行重写模型、资源和证据的绑定规则。实现位于 `mo-presentation-delivery::inspect`，Native CLI 与 WASM 经过同一计算入口。它验证实际字节及引用一致性，不授予权限、不发布 Artifact，也不将生产者声明升级为 Office/WPS、视觉或播放验收。

## 产品接入位置

再次只读观察 Musterwork 时，工作区基线为 `5750dd6378dc941dc8fc0948d54d4269eb154256`。现有 `presentation-html-artifact.ts` 仍解释旧 HTML 文稿交付，`delivery-proof.mjs` 仍绑定 PDF 字体证据。因此新 bundle 不能塞入旧 discriminator。此次没有复制私有产品实现或修改其业务路径；仍按[适配合同](../design/implementation/musterwork-adapter-spec.md)推进 successor、Content Store 映射和单一提交 owner。

接收链路为：宿主保留已授权不可变内容 → 核对已接收操作的文稿/revision/semanticDigest/settingsDigest/renderer pins → 共享内核读取交付字节 → 返回只读接收报告 → 产品校验 Invocation、generation、fence、取消与当前 pin → 原子提交全部资源和 Artifact。宿主不得用待验证交付包自身的值取代原操作 pins，也不能把可反序列化的报告当作授权或执行证明。

## 实际检查

在打开任何资源前检查版本、计数、重复身份、逐资产和总量预算。`DeliverySource` 只提供不可变的受权范围读取器；每份文件必须与声明长度、SHA256 完全一致。读取器及其 bytes 在校验与后续提交期间必须保持同一身份，保留和访问控制由宿主实现。

模型经过既有语义校验和 semanticDigest 重算，并核对外部 pin。随后验证模型/PPTX/逐页预览的引用闭包、资源角色及 MIME、字体包/清单、实际能力注册表、设置摘要、渲染器身份以及所有资源是否被引用。当前明确支持现有 `1-draft` 交付 profile，未知版本或不受支持的质量声明失败，不猜测结构。

PPTX 从实际内容重新进入已有 OPC 包图和来源索引校验，核对页面数量与尺寸。质量记录绑定同一 PPTX、模型、上下文与页面顺序；逐页证据绑定源 slide、隐藏状态、图片描述、尺寸和像素摘要。当前 profile 只能产生结构检查与其余四项 `not_proven`；把这些记录改成应用/播放通过会被拒绝。

预览按当前 Writer 的确定性 PNG profile 检查：RGBA8、非交错、sRGB、Sub filter、块顺序、CRC、zlib 精确范围及解压长度；重建预乘 RGBA 后核对渲染证据中的像素摘要。实现逐块和逐行读取，不额外分配完整帧。临时存储为有界块缓冲、解压器状态、块范围表和一行像素；这不是通用 PNG 导入解码器，也不构成完整交付 RSS 上限。损坏 deflate、宿主读取故障和取消保持不同的错误路径。

这些检查证明交付文件之间的字节与引用绑定，不证明一次不可信远程执行确实运行了声明的 renderer，也不证明模型到 PPTX 的全部内容/视觉等价。生产宿主仍须绑定受信执行器和原操作，并运行相应质量门禁；完整外部编辑及目标应用证据保持未验收。

## 调用与语料

Rust 产品宿主直接提供 `DeliverySource`，无需第二个数据库或任务队列。`ReceivedDelivery` 只能由实际校验构造；公开报告明确将质量字段命名为 `declaredClaims`。

```sh
target/debug/mo-cli delivery-inspect \
  fixtures/presentations/delivery-receive/request.json \
  fixtures/presentations/delivery-receive/assets.bin
```

WASM 导出 `inspect_delivery(requestJson, contents)`；开发桥采用有界连续二进制范围，拒绝遗漏、重复、越界、重叠和未绑定字节。它不创建浏览器 owner，生产原生宿主可直接使用逐资源读取器而不拼接整个包。两个新增运行 Schema 与 TS 类型由 Rust 生成。

[自有接收语料](../../fixtures/presentations/delivery-receive/README.md)来自真实 TS/native 导出的两页文稿，12 个文件合计 69,623 字节，包含 15 个原生对象、八段合成字体文字及实际 PNG；没有产品私有文稿或系统字体。新增故障测试修改实际文件后重新计算描述，确保验证不仅依赖外层摘要。

## 验证结果与成本

全仓 855 项 Rust 回归、严格 Clippy、111 份 Schema 与 TS 检查通过。随后完善 PNG 错误分类，区分损坏压缩数据与真实宿主读取失败；受影响库的全部 19 项专项测试重跑通过。初次包损坏负例曾假设错误文本包含 `ZIP`，实际得到缺少 central directory；断言改为具体 OPC 错误类型，旧失败日志保留。

36 组实际 Native/WASM 调用完整响应一致，含三组成功接收和 33 组故障输入。独立 Python 对所有响应执行 Schema 检查、对三组成功结果重算规范 bundle 摘要，并从真实 PNG 解码、恢复预乘值，核对两页像素摘要。接收语料的全部 12 份文件与之前真实 TS/native 导出逐字节一致。既有导出 17 组、来源读取/编辑 33 组和模型事务 22 组双端回归保持冻结的历史结果。

当前锁定 release 构建的 Rust WASM 为 10,049,547 字节，gzip-9 为 2,722,193 字节；上个计算构建分别为 8,764,177 / 2,425,295 字节，增加 1,285,370 / 296,898 字节。这里只比较 Rust 模块，未含 JS、字体、绘制/塑形组件或产品包装；没有新增第三方组件版本，`flate2`/`crc32fast` 沿用已有锁定依赖。此次证明接收能力与一致性，未证明体积、延迟或 RSS 已达发行目标。

验证程序为 `delivery-receive-workspace.py`、`delivery-receive-parity.mjs`、`delivery-receive-reference.py` 和 `delivery-receive-regression.py`，都位于 `tools/verification/`；输出写入独立目录，不覆盖旧阶段产物。源码、构建、调用、反例及边界由[阶段证据](../reviews/evidence/2026-09-26-delivery-receive-verification.json)绑定。实际产品 successor/shared parser、实时 Viewer/Player、完整高级内容与 E0–E3 替换门禁继续开放。
