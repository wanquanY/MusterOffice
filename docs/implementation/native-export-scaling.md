# 原生文字与形状完整导出测量

2026-09-30。状态：macOS arm64 的指定工作负载局部验收完成，**不是 W-S10/W-S40
混合内容门禁，也不是完整替换、安装包或 Agent 端到端性能结论**。

## 输入和计量

[自有创作输入](../../fixtures/presentations/native-text-shape-scale/compose.json)来自本项目
夜间基础验收样本；取第二页复制为 10/40 页，只重新分配页面和元素 ID、页面名称。
每页 12 个原生形状，其中 7 个文本框，包含中文标题、三张卡片与正文。页面内容相同，
不代表 40 页不同内容或复杂模板。字体、字号、几何、文本和默认预算没有降低。

实际调用公开 `compose`，再以新 CLI/Worker 进程执行完整 `export`：包括输入验证、
字体塑形、逐页 1280×720 预览、PNG/PPTX 编码、交付字节检查与本地结果发布。
计时从 CLI 启动到正常退出；不含 compose、外层测量程序的独立复验、网络或模型思考。
compose 单次观测分别为 0.025/0.035 秒，未对其声明 P95。

设备为 Apple M4 Max、36 GiB 内存、macOS 26.2；Rust 1.92.0 release，内核运行源码
`a86266c`。系统缓存未清空，每组排除一次预热，再顺序测量 30 次；没有跨进程保留
内核页面缓存，但操作系统页缓存可复用。产品、后台及其他应用保持运行，非隔离机器。
P50 为中位数，P95 使用 nearest-rank 第 29/30 个样本。

每 100 ms 采样 CLI 所属独立进程组的 RSS，实际观察到 CLI 和计算 Worker 两个进程。
表中内存为**采样 RSS 合计最大值**：共享页可能重复计算、短峰值可能漏掉，不能当作
物理内存峰值、产品总占用、空闲基线之上的增量或长期泄漏证明。

## 结果

| 输入 | 导出 P50 | 导出 P95 | 最慢 | 采样 RSS 合计最大值 | PPTX 字节 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 10 页／120 形状／70 文本框 | 1.506 s | 1.605 s | 1.641 s | 80.969 MiB | 21,687 |
| 40 页／480 形状／280 文本框 | 4.051 s | 4.281 s | 4.552 s | 122.984 MiB | 72,838 |

每组 31 次（含预热），共 62 次成功；每次都独立核对全部交付资源的长度与 SHA-256、
ZIP CRC、页数、每页 12 个 `p:sp` 和零 `p:pic`。全部页均有容量测量，10/40 页分别
检查 210/840 对文本范围，没有容量问题、相交候选或未检查项。全部 PNG 与 PPTX 在
相同工作负载的重复调用间逐字节相同。每次执行后，临时目录仅保留执行注册锁，没有
文稿、资源或执行租约残留。诊断通过不替代完整视觉、Office/WPS 编辑及高级内容验收。

另将实际 10 页 PPTX 和完整交付检查分别交给 Native 与 WASM：完整检查响应、十页
元数据、容量测量与 RGBA 全部一致。这是跨端质量回归，未把 WASM 耗时计入上表。

字体输入是显式 22,131,464 字节资源包，包含 13 份字体数据，Noto Sans SC 中文字体
本体为 17,772,300 字节；其他为既有 Liberation 字体。资源包 SHA-256 为
`69fadb07607ad4f9421e20c759c50a47ee5b9c452db8e1ba0c83046742b39e4c`。
PPTX 使用既有 `referenceOnly` 字体策略，不包含嵌入字体和整页截图；小 PPTX 不能当作
首次离线交付体积。字体资源与全部预览仍计入交付资产，完整字节总数见证据。

依赖及构建闭包保持：HarfBuzz 14.5.0、Skia `8d6d37b0`、libpng 1.6.56、
libjpeg-turbo 3.2.0、zlib 1.3.2，以及 [Cargo.lock](../../Cargo.lock) 中 Rust 依赖。
组件实际静态库、锁文件、CLI 和 Worker 均记录摘要。Worker 为 12,489,648 字节，
系统动态依赖为 libiconv、libc++ 和 libSystem；这些数字不是完整 Musterwork 安装包大小。

## 旧组件对照

保持相同 compose、字体、布局和预算，仅换为 `c0a7100e` Worker 及匹配的设置摘要。
旧组件在 10 页输入的第一页、对象 13 处触发 `native frame component work`，没有发布
输出，临时计算资源已回收。由于没有旧成功基线，**不报告提速百分比**。本次可确认的
收益是[字体作用域复用](font-residency.md)后，相同文稿在原有预算内稳定完成。
旧组件的失败耗时和较低的部分执行 RSS 不能用于成功导出性能比较。

## 复现及边界

[测量程序](../../tools/verification/native-export-scale.py)只依赖 Python 标准库与 `ps`。
调用者准备与固定设置匹配的有权使用字体包，并按 CLI 的显式资源绑定格式提供
`inputs.json`；不会自动读取系统字体、下载内容或启动产品服务：

```sh
python3 tools/verification/native-export-scale.py \
  --compose fixtures/presentations/native-text-shape-scale/compose.json \
  --slide-index 1 \
  --settings fixtures/presentations/native-text-shape-scale/settings.json \
  --inputs "$MO_INPUTS" --cli "$MO_CLI" --worker "$MO_WORKER" \
  --worker-sha256 873e0cbb254f2b5f940ae4d492be0fce63bc8f962fc7f310dbc8e1a5bd2a61a1 \
  --trials 30 --output "$MO_NEW_OUTPUT_DIRECTORY"
```

输出目录必须不存在。每次保存请求、结果、日志及 RSS 数值采样；不收集其他进程命令行。
外层超时只终止测量程序自行创建的进程组，不变更内核预算或真实产品进程。
详细样本、摘要、环境和失败对照见
[机器可读证据](../reviews/evidence/2026-09-30-native-text-shape-scale.json)。

后续仍须使用[正式混合工作负载](../design/presentations/runtime-performance.md)覆盖不同
内容的图表、表格、图片、复杂主题、媒体、动画和取消；最低配置设备、完整依赖安装体积、
模型调用时间、Office/WPS 往返分别测量。本次不修改或关闭原设计中的这些门禁。
