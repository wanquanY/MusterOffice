> Musterwork 历史设计快照 · 迁入日期：2026-09-24。来源：`docs/architecture/agent-runtime/proposals/presentation-client-integration.md`。
>
> 原文中的“当前”“已采纳”和商业 SDK 选择只代表来源项目当时的状态，不是 MusterOffice 的决定或验收结论。内核以[现行设计](../../design/presentations.md)为准；来源、摘要和链接转换见[迁移清单](README.md)。

# PPT 客户端执行接入与版本合同

2026-09-23；接续批准的 `presentation-lightweight-runtime.md`。本文描述已经接入的开发路径。正式发行切换仍受原方案的授权、视觉等价、资源与性能验收约束。

## 执行所有权

Rust Device Runtime 继续拥有任务准入、参数授权、输入暂存、超时、取消、进度、组件版本固定、候选恢复和 Artifact 提交。Electron 只是它的本地执行端口。

```
Rust LocalPresentationEngine
  → 本机私有 TCP 端口 / 单请求连接
  → Electron service（准入与生命周期）
  → Node Worker（有界输入、bundle、摘要、文件持久化）
  → 一次性沙箱 BrowserWindow（DOM 布局）
  → 编译 Worker / 组件 Worker / 专用文件渲染 Worker
  → Rust 校验结果并提交
```

连接仅监听 `127.0.0.1`，端点文件权限为 `0600`，令牌每次启动轮换；端点不进入不可变组件身份。版本清单绑定 generation、字体与编译器身份，保持相同组件重启后的 checkpoint 可读。单请求连接关闭即取消，不存在后台第二个 Agent，也不回退服务端渲染。只接受 Rust 指定暂存根下的直接子目录，拒绝任意文件路径与命令。

Web 继续由 Cloud Runtime 授权 Browser Job，浏览器 broker 驱动隔离 Runner。共享 `presentation-client` 负责同一套编译、校验、缓存与结果合同。Web 与 Desktop 分别注入文件渲染宿主，不跨应用导入源码。

## 版本演进

| 合同            | 原有版本                                            | 新客户端版本                                                 |
| --------------- | --------------------------------------------------- | ------------------------------------------------------------ |
| native result   | `presentation-native-engine/1`，PDF 和创作 PNG 必需 | `/2`，仅最终页图，不允许 PDF 或创作 PNG                      |
| delivery proof  | `presentation-delivery-proof/1`，绑定 PDF 摘要      | `/2`，绑定实际文件字体清单、字符坐标与渲染器身份             |
| candidate state | `presentation-candidate-state/1`，PDF 引用必需      | `/2`，PDF 字段必须缺席；独立 v2 binding hash domain          |
| Artifact        | `/1` 为 reviewed、`/2` 为 compiled；两者均有 PDF    | `presentation-artifact/3`，可为 compiled 或 reviewed；无 PDF |
| 工作区预览      | 原有预览带 PDF                                      | `presentation-workspace-preview/2`，PNG 集合绑定 PPTX 摘要   |

不重新解释历史版本，不以 `null` 冒充字段缺席。Rust、Backend、TypeScript、生成后的 Electron 合同和前端证明校验同步演进。跨语言 known-answer 固定新候选封装的精确字节与 binding；新版本仍校验依赖闭包、原生对象、字体、字符位置、每页摘要、尺寸及数量。

Apryse 内部固定版式 PDF 仅用于提取实际字体身份，Worker 完成证据提取后丢弃；不是用户交付物。它不能被替换成“预期字体列表”，否则无法发现实际字体替换。

## 资源、缓存和响应

- 字体按 Document 的主题和显式字体声明加载，并包含声明的 fallback 家族；全量字体清单身份不变，PPTX 的完整可编辑字体嵌入不变。
- 客户端组件 v2 把字体拆为独立家族包。原始完整 face 使用 `musterwork.presentation-font-pack/1`，新构建使用 `/2`：每个 face 只分发一份预压缩的完整 EOT 字体部件，清单同时绑定原始 SFNT 的路径、摘要与大小。许可证文本完整保留，包摘要及 bundled 属性纳入组件 generation。`fonts.json` 仍保留完整清单，不把“未随基础组件安装”解释成“不支持该字体”。旧组件 v1 继续按原有文件列表读取。
- 字体压缩在受控资源构建阶段完成，使用与 PPTX 相同的固定 JSZip/DEFLATE 和归一化元数据。组件 Worker 先校验完整压缩文件，再有界解压、校验完整 SFNT 并提供原逻辑字体路径；不额外分发原始字体副本。编译器将 EOT 与实际受管字体重新生成的完整 EOT 逐字节比较，通过后复用压缩字节。原生字体检查、实际文件字体清单及交付证明每次仍执行；未知或损坏资源不会退回未验证路径。
- 字体归档仅允许一个指定部件，严格检查本地/中央 ZIP 目录、数量、路径、时间、权限、声明大小；拒绝重复项、ZIP64、附加字段或注释。解压前后均有限量检查。支持时使用平台原生解压与异步 SHA-256，旧非浏览器宿主可使用固定版本解压实现；压缩算法与导出字节保持不变。
- 资源 Worker 明确拥有校验和解压的临时数组：WebCrypto 获取输入快照、Blob 保存不可变字体后，支持时通过 `ArrayBuffer.transfer(0)` 释放这些私有缓冲。缓存仍是不可变 Blob，借用的输入、网络块及调用方数组不会被分离；旧宿主保留正常 GC 行为。不在产品中强制 GC，也不省略摘要校验。
- 默认 `Noto Sans CJK SC` 与 `Liberation Sans` 位于 `components/packs/<id>/`；其他家族构建到独立的 `font-packs/<id>/`。安装或静态发布时可分别分发；修改默认集合使用构建参数 `MUSTERWORK_PRESENTATION_BUNDLED_FONT_FAMILIES`（JSON 字符串数组），不能在执行过程中静默替换。
- Desktop 的 Node Worker 从部署配置中的目录或 HTTPS 静态源准备所需家族，逐文件限量校验，全部完成后原子激活本地缓存。取消不留下可读取的半包；缓存可以脱离原资源源复用。请求只能引用清单中的相对资源，不接受模型传入下载 URL。Main 以文件流交付字体，不整体读取大字体缓冲区。
- Desktop 单宿主启动准入前清理不属于当前组件的旧缓存包。当前组件全部字体包总量上限 256 MiB、单包上限 128 MiB；已开始的执行仍固定 generation，旧 generation 不在此宿主并行执行。暂存归属 Rust job，停止 Worker 不会激活未完成包。
- Web Worker 从同源的内容寻址静态路径按需读取并校验字体，缓存键使用包身份，可跨编译器更新复用不变的字体。开发静态挂载只服务预置目录，不替浏览器下载或代理上游资源。私有化部署可预置全部或选定包；运行时不访问厂商字体 CDN。浏览器页面及业务服务的完整离线运行不在此字体缓存结论中。
- `layout-cache` 保存原始页面测量值，最多 256 页、32 MiB、每页 1 MiB。键包括页面、主题、画布、语言、资源摘要、浏览器和字体身份。Web 由 Runner 持有，Desktop 由本地 service 持有，通过窄 MessagePort 请求读取。
- 布局命中仍重新遍历字体和文本、执行文档绑定及确定性检查；不缓存审批、质量结论或 Artifact 权限。
- 最终文件另有 64 MiB / 128 页的进程内 LRU。缓存键来自完成原生校验的实际 OOXML：逐页追踪布局、母版、图表、工作簿和图片的传递依赖，未归属页面的部件保守视为全局依赖，再绑定页序、画布、120 DPI 和组件 generation。活动外部引用不进入缓存，断裂引用明确失败。
- 页缓存仅保留真实文件渲染的 PNG 与字符坐标。当前 PPTX 仍由 SDK 打开，每次仍提取实际字体证据并执行完整质量规则。原生字体校验已计算的摘要仅在同一个未替换 ZIP entry 上复用，避免重复解压和哈希。未完成写入不进入缓存。
- Desktop 与 Web 使用同一缓存协议；大页图按 256 KiB 分块并逐块确认，元数据上限 256 KiB。缓存不持有任务权限或质量放行结论，组件版本变化不会命中旧页。
- Electron Main 仅负责窗口、受限静态资源、缓存和有界二进制流。大文件解析、bundle 编解码、摘要和写盘已移入可终止的 Node Worker。
- 每次任务销毁独立 renderer；字体、SDK 和文件资源由本机或受控静态源提供。SDK 资源目录必须是已登记的完整闭包，拒绝额外文件和符号链接。
- 同一版本的工作区文件预览复用 Rust 已有的摘要缓存；服务重启/组件变更不混用不同 generation 的资源。

## 开发选择与恢复

显式设置 `MUSTERWORK_PRESENTATION_CLIENT_CANDIDATE` 选择经过本地校验的候选清单，然后运行正常 `desktop:dev` 或 Web/Runner 开发命令。`presentation:dev:prepare` 与 Desktop supervisor 会验证该清单并跳过旧原生 PPT 组件的准备，不会在候选失效时偷偷重建或改用旧引擎。候选目录是不可变快照；修改共享编译或渲染源码后须重新构建候选并重启对应宿主。

正式发行构建拒绝开发候选。旧发行路径暂时保留，是原方案“取得资格后再移除”的边界，不是新客户端执行失败时的回退。

重连使用新的私有端点令牌；相同版本清单和输入 checkpoint 可继续复用。改变候选 generation 时，旧任务明确得到环境变化错误。尚未实现同时托管多个旧候选进程，不宣称任意跨版本热更新都可无缝恢复。

## 仍须完成的验收

1. 取得覆盖 Web、Electron、离线及客户私有化分发的真实 SDK 授权，并落入受控发布资源。
2. 用 PowerPoint/WPS 裁定并解决已发现的环形图几何、字重差异，完成模板/导入/编辑复存质量矩阵。
3. 已实现最终文件按页增量缓存及独立字体家族包；仍须在正式授权资源、目标低配置机器与最终安装载体上完成发行验收。
4. 本机 Web 与 Desktop 各完成 10 / 40 页每组 20 次全量页面渲染，P95 分别约 4.07 / 5.79 s 和 4.51 / 6.21 s；保留完整导出与质量步骤。临时资源缓冲已压降，进程树峰值仍约 1.8–2.0 GiB；这不是低配置目标机器的验收结论。
5. 上述条件通过后再删除旧发行组件，验证真实安装包体积、签名、离线运行和升级。当前未打包或部署。

实际证据见 客户端接入验收记录（`Musterwork:docs/testing/reports/2026-09-23-presentation-client-integration.md`）。
