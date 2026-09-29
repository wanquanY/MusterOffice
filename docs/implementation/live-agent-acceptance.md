# 真实模型创建与修改验收

2026-09-30 03:30（Asia/Shanghai）。在用户授权的主工作区，使用正在运行且已认证的
Musterwork Device Runtime、产品原始消息提交/内容读取接口和配置中的
`z-ai/glm-5.3-flash`，完成自有三页中文 PPT 的创建、导出、发布，以及同一 Artifact 的二次修改。
没有人工代模型构造文稿或伪造 Tool 回执。本项没有操作桌面 UI 或 Office/WPS。

产品源码 `85427e2e7`，Skill `musterwork-presentation-authoring@11`，Tool
`artifact.presentation.author@7`。测试开始前同时核实实际安装二进制与 Worker：

- Device：`45a4f53f753dc60577e5bc0b65477c258bf0e01c5f120f4e9bd78fa2d9f5e749`。
- Worker：`3aab449853fecf845614ee2dc955a3d0c09016b73d95406f0dca1c4d9e251290`。

## 已有证据

| 指标 | 创建三页 | 修改同一文稿 |
| --- | --- | --- |
| 观测的 run 开始至完成 | 914.908 秒 | 104.863 秒 |
| 已计量模型调用 | 42 | 6 |
| Tool 请求 / 失败 | 53 / 10 | 10 / 1 |
| 记录的输出 token，包含 reasoning | 67852 | 5314 |
| 实际 PPTX | 9227 字节 | 9217 字节 |

这是一次包含模型推理、依赖等待、错误修正、导出和提交的端到端观测，不是内核渲染
benchmark，也不是可承诺的平均或 P95。首轮另有一次模型响应无效，运行时自动重试。

第一版摘要为 `bb8b8eac9ebe2dd37603fd39b510bd8c058e87cfcebe37da5bfa113c5a8f27d7`，
第二版为 `d13725e22a56827a1762c9dc3a5b04d1ddaaef5d55baa02dd8ae359dffb8dfee`。
从产品授权内容入口下载后独立检查 ZIP/XML：三页 16:9；分别含 3、5、7 个原生 `p:sp`，
没有 `p:pic`。第二版只修改第一页副标题和第三页流程文字，实际包只有
`ppt/slides/slide1.xml` 与 `ppt/slides/slide3.xml` 改变，Artifact 身份保持不变。

首轮模型确实自行纠正了字符串参数、缺少产品 revision pin、原生段落间距不可精确表示、
尚未实现的 decorative 扩展与不换行渲染等问题；这些失败保留在证据中，没有放宽内核校验。
旧失败会话不因新会话成功而倒记为通过。

## 尚未通过的质量门禁

逐页查看实际 PNG 发现第三页第一个色块有文字超出底边。对应内核证据已经给出正的
`verticalExcess`，但导出 Tool result 目前没有直接返回这些测量信息，模型也未完成视觉修正。
质量文件如实标记 `layoutQualityProven=false`；成功导出不能代替视觉验收。

接下来一方面用[高层创建](compact-authoring.md)减少字段拼装与类型查询，另一方面将
已校验的内核布局诊断反馈给 Agent，并验证模型修正后再交付。诊断不得被产品重新计算、
升级为目标应用认证，或通过静默裁剪/缩字抹掉。

Office/WPS 打开、编辑、保存，完整原生图表/高级内容、两份 14 页模板整体转换、跨平台
安装包、容量与性能验收仍未通过。这一闭环证明当前 Agent 可以实际创建并修改基础 PPT，
不代表完整一期或全部替换目标完成。自有内容及完整运行日志保存在忽略目录；不提交凭据、
模型私有推理、个人文件或主机配置。
