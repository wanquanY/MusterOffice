# 产品素材描述与原生导入接入

2026-10-07。Musterwork 实现提交 `dec25380fa0e29e36dc92db1cd163b464244db81`。此项把现有精确索引 SDK 接到原 Product 与 Agent 入口；完整视频方案仍在实施。

## 已接通的路径

产品采用原 FileQueryService 解析精确文件引用，使用同一份 VerifiedContentReader 为容器探测与 NativeIndex 提供有界随机读取。按真实容器轨道 ID 关联解码流、索引和 VideoTimingSummary。使用内核的精确 movie_end 形成视频 available 范围，保留 VFR、空白、重复编辑与 dwell；不在产品重新实现时间映射。全部 worker 关闭并回收后才释放计算额度。调用方退出触发取消，不能提前放出仍被子进程占用的物理额度。

`describe_media` 已进入手动 Product 命令和 Device `video_author` 的共享请求合同。输出包含内容摘要/字节数、固定组件身份、流信息与 indexed_video descriptor；source raw ticks 转成十进制字符串。导入仍需原 file_ref 和普通 SDK 事务校验，不把探测摘要变成文件权限，也不新增工程保存系统。Product 前后检查原运行时 authority；Agent 用原 Attempt、Invocation 和内容账户登记只读工具结果。

新不可变 Device v70 发布 `artifact.video.author@2`；旧 v69 和 v1 继续支持原调用恢复。技能 v53 发布视频创作 v2，使用当前工具做实际探测，禁止猜测音频时长或将索引结果当成解码证明。PPT 和 Cloud f52 合同不变。探测不会生成虚假工程卡片，新版本编辑仍复用已有卡片和右侧工程入口。

桌面主进程把开发组件目录转换为既有系统服务的 `video_runtime` 配置。后台 Rust 在激活产品前验证固定 worker manifest 与 SDK 绑定，再为手动编辑和 Agent 共用的素材宿主提供组件。重启应用或后台服务使用持久配置；不增加服务器媒体计算。没有组件时 metadata 编辑仍可用。正式生产打包和跨平台组件交付仍待完成。

## 验证范围

真实媒体为 VFR H.264/AAC、edit-list 变体和 signed CTS 变体，固定输入摘要沿用[原生时间映射](video-indexed-composition.md)的素材。实际验证覆盖精确区间、track 42 与 2147483649、晚权限撤销/描述变化、内容篡改、取消与异步丢弃后的物理额度，以及 descriptor 经原手动命令导入真实 SDK/SQLite 工程和恢复原回执。实际 Agent 检查经过已准入工具、原 Invocation 内容账户与 Tool journal；FileQuery authority 使用可控 fixture，不等于已用真实登录模型完成验收。

SDK 和 worker 仍固定 `0.1.0-video.20261007.2` / profile 5，没有在产品引入核心源码依赖或临时修改 registry 包。开发源码稳定标签 `v0.1.0` 不变。

最终 **103 项定向测试通过**：真实媒体/手动导入 6、Agent authoring 7（含真实媒体调用 1）、发行合同 17、ProductSnapshot 37、Device 配置 2（含实际组件装载）、大整数 wire 1、持久化 5、桌面配置 15、消息呈现 6、Backend 发行/技能 7。Backend 另有 7 个 subtest，不重复计数。显式原生检查 0 ignored；类型检查、工具合同和技能历史核验、生产库 strict Clippy 通过。没有运行整个产品全部测试套件。

检查结果与源码/日志摘要记录在产品 `.codex-work/video-inspection-report.json`。安装配置测试使用受控系统服务调用，并没有真实安装或停止用户的 launchd 服务。默认 ignored 的原生检查只在显式提供固定素材和组件并实际通过时计入。

## 尚未完成

- 精确音频映射、其他容器、图片导入，以及面向用户的完整素材导入交互。音轨当前返回明确 unavailable，不能伪造可用 descriptor 或无提示地移除原声。
- 帧与音频到真实播放器、预览调度、音画同步、导出任务/候选提交/下载 UI，以及 Browser 本地媒体实现。
- 长素材存储突破现有产品内容上限、性能/体积目标、跨平台与真实 provider 计费，以及整套本地用户验收。

本次 indexed_video 描述的是精确索引结果，不保证每个压缩样本均可解码、每种色彩声明都可展示或已经生成可播放成片。完整退出条件继续以[最终验收矩阵](../design/video-editing/acceptance.md)为准。
