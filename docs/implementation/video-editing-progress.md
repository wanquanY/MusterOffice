# 视频创作与剪辑实施记录

2026-10-09 · 目标执行中，尚未达到整体验收。

## 本批增量：受控修改与原子效果判定

产品提交 `9faf62366` 将受控修改接入原 Office prepare/inverse 和 Product 的 head/history/receipt 事务。schema 9 只增加请求准入和效果状态；明确取消与提交在同一个本地事务中竞争，保存成功后丢失回执仍可查询原成功结果，取消先落盘则阻止迟到候选。查询不会执行修改，网络中断不冒充原操作取消。失败发布也检查原有写入 epoch/owner，旧执行者不能干扰新执行者恢复；没有第二份工程、历史或租约权威。

66 项 Host、22 项 Browser 测试通过。独立构建的真实 Chrome 154 使用产品应用层、Worker、IDB 和实际 Vue 编辑器，验证两个页面看到同次修改、人工后继修改不被重放覆盖、共同撤销/重做、刷新后取消操作不复活。另在独立 schema 8 数据库中核对合成工程的原记录摘要，升级到 9 不变。源码集合摘要 `4fbb19b8d8afb63260e4cdda94cbf0c644e2825a2bbce596b1ede3f64d0775bb`；产品证据目录 `.codex-work/video-routed-edit/`。

Web 类型/正式构建与体积门禁、新 Host 测试类型和 20 文件 lint 通过；公共入口 gzip 507,361 / 524,288 bytes。183 个原生成/Admin/计费 WIP 保留，Office 计算核心、组件 pin 和稳定 `v0.1.0` 未改。这是修改执行和恢复基础的模块验证，公开 Gateway 修改/控制协议、Tool/Skill、导出与消息交付仍待接通；不能宣称整个方案或真实 Agent 修改已可验收。

## 本批增量：公开 Agent 读取与真实人机往返

产品提交 `ac1ee9415` 发布 Web `artifact.video.inspect@1`，通过 cloud f53 / Skill v56 / 视频创作 v5 和真实 Cloud composition 接入原 Tool 生命周期。目录、能力与快照共同支持“读取原 provider 任务记录”的重放合同，不按视频名称开例外；旧工具及 Skill 不改义。Office 原固定内核负责有界摘要/分页计算，Product 负责消息目标、授权、任务、结果完整性及恢复。

正常 Chrome 154 页面、正式密码登录与实际在线聊天模型已完成三轮：读取选中工程 revision 2；用户在右侧修改并保存后，新消息读取同一工程 revision 3；下一轮按明确版本读取实际 sequences 页。元数据分别为 1,460 / 1,473 / 1,134 bytes，无媒体上传和页面异常，原 Runtime 遥测确认三次 Tool 和三个 Run 成功。没有注入模型回复或内部测试任务。报告在产品 `.codex-work/video-public-read/normal-agent-roundtrip-02/`；这验证空工程的真实读取与人工修改后重读，不能代替 Agent 修改、成片或生成计费验收。

219 项相关测试、独立 Rust workspace/all-targets、受影响库 strict Clippy、Python Ruff 和验收脚本 ESLint 通过。完整依赖 strict Clippy 仍有 PostgreSQL 模块 8 项既有告警，失败证据保留。已通过原模型配置服务补齐隔离测试库缺失的默认聊天绑定，主开发库只读。183 个既有生成/Admin/计费 WIP 原件保留；Office 核心、组件 pin 和稳定 `v0.1.0` 未改。详情见产品 `docs/architecture/browser-video-agent.md`。

下一步继续原生统一读取、公开 Agent 修改/导出与本地成片消息附件；最终 20 项产品需求及完整验收目标仍在执行中。

## 本批增量：有界读取与结果校验

产品提交 `558331a74` 增加内部 `browser.video.read@1`，以同一个固定 Office 内核读取摘要与分页对象，查询及版本在原调用的本地准入中绑定；cursor 后续页必须使用明确版本。独立协议、home/组件路由、迁移 224 与 Web broker 已接通。原检查协议不改义；媒体和快照仍留在本地，只回传最多 64 KiB 元数据。

真实 Chrome 正常登录产品页后，原 current 选择 revision 3，人工修改为 4 后原调用不重做、不上传新结果，新调用读取 4，明确版本读取 2；序列页通过相同 Gateway/Worker/IDB 返回。另一个浏览器不能抢占，关闭面板仍可执行，登出关闭连接。四次新调用回传 1,499 / 1,489 / 1,499 / 1,134 bytes，旧固定检查回归通过。两个报告位于产品 `.codex-work/video-agent-read/`，源码摘要均为 `f6ccddc04faa1c10c560976bfc758f18148fec035257a6057e4ac0b49785d793`。

Runtime 候选读取执行器已加入原消息目标解析、原 Job 恢复、ContentRef 与本地准入摘要校验，以及固定 Office SDK 生成的请求/结果 schema 验证。没有在云端复制剪辑或分页算法。公开 catalog/composition/Skill 及原 Tool journal 验收尚未接通，不能视为公开 Agent Tool 已交付；Agent 编辑/导出、本地消息卡片及完整最终方案继续实施。

217 项相关测试通过，另 1 项既有脱敏 PPT 语料测试 ignored；独立 workspace/all-targets 编译、相关库 strict Clippy、类型/lint 和 Web 构建预算通过。公共入口 gzip 507,371 / 524,288 bytes。183 个既有 WIP 完整保留；Office 核心、组件 pin、稳定 `v0.1.0` 不变。详情见产品 `docs/architecture/browser-video-agent.md`。

## 本批增量：原调用的版本化 Gateway 检查

产品提交 `ba48c28da` 增加显式内部 `browser.video.inspect@2`，将 current/明确 revision 选择与原 invocation/job/conversation 接入本地持久准入；旧 `@1` 固定版本含义不变。独立 engine/program、原 home/组件路由、领取/执行双重校验和 additive migration 223 已实现。通用代码 Runner 拒绝两类视频请求；只回传最多 64 KiB 元数据，媒体和快照保留本地。

真实 Chrome 正常产品登录后关闭面板，新调用选择 revision 3；人工修改到 4 后原 Job 重放仍返回 3、无新执行/上传，新调用返回 4，明确版本返回 2；另一个独立浏览器不会接手原 home。旧协议回归仍返回原 revision 2。三个新意图分别回传 1,484 / 1,474 / 1,484 bytes，退出登录关闭连接，没有页面错误。源码摘要 `5981098307f13acfd1dcddb74996fd3c09317704cc0d4b486d8b576cbcb5ba81`，产品 `.codex-work/video-agent-inspection-v2/` 保存两个正常页面报告。

独立 workspace/all-targets 编译、82 项 Rust/真实数据库/Web/Browser 测试、Web/Browser 类型、变更 lint、迁移/协议锁和正式 Web 构建预算通过。相关 Rust 库 strict Clippy 通过；扩展范围仍存在原 capability physical_execution 测试及 Postgres 模块告警，未作为全部通过报告。183 个原 WIP 完整保留。详细验证和复现见产品 `docs/architecture/browser-video-agent.md`。

这是实际内部执行链，不是公开 Agent Tool 发布；原公开 Tool 的请求/结果校验、编辑/导出和本地消息附件仍待接通。Office 核心、固定组件和稳定 `v0.1.0` 未变，最终目标继续执行。

## 本批增量：双端输入框选择剪辑工程

产品提交 `cea54a874` 已把工程选择接入真实 Web/桌面布局：选中视频创作技能后，可在编辑器点击“用于本条消息”，输入框显示可移除的工程名称。目标来自实际打开的工程及宿主身份，名称只是显示信息；关闭面板不改变选择。草稿按账号与会话保存，刷新恢复；移除技能清除目标。发送、召回、重跑与消息投影携带同一目标，异步回执和会话切换不能覆盖新草稿。

同时修正桌面延迟草稿写入使用新账号键的问题：持久化始终绑定已激活的草稿所有者。召回在异步撤回期间切换会话后，不再把原消息写入新输入框。相关行为有回归验证。

独立源码快照通过 Web/桌面类型检查、114 项相关测试及 44 个源文件静态检查；正式 Web 构建与预算通过，公共入口 gzip 507,338 / 524,288 bytes。正常登录页面的真实 Chrome 154 验证了技能选择、创建/改名工程、选择、关闭面板、刷新恢复及移除后刷新；没有调用模型或上传媒体，页面错误为空。报告位于产品 `.codex-work/video-editing-ui/browser-final/report.json`，源码集合摘要 `89d40aba20f31ec51636ad4f5e63739c6017f1a9148a701808681c06f4c30d95`；之后仅调整两处缩进，报告不冒充最终整个工作树摘要。183 个既有 WIP 文件均保留，详细独立验证与保护记录见产品架构文档。

这是交互与原消息目标的接通；公开 Agent 浏览器读取、修改、导出及消息成片交付仍继续实施。Office 计算核心、组件 pin 和稳定 `v0.1.0` 不变，完整目标尚未达到验收。

## 本批增量：消息指定剪辑工程

产品新增独立的工程上下文，浏览器定位绑定 workspace/home/project/authority/固定组件；桌面定位绑定 workspace/project。提交、消息事件、冷热投影、Tool journal 与执行校验保存同一目标；不携带媒体、凭证或永久固定的 current head。用户新消息未选择工程时不继承旧工程；无新用户消息的续跑/委派只沿 canonical parent Run 链继承，并在最近的用户选择（包括未选择）处停止。桌面 prepared command 更换目标会被拒绝。

产品提交 `35edb7039` 的独立源码快照通过 Rust workspace/all-targets 编译，协议/应用与专项 150 项、真实 PostgreSQL/SQLite 2 项、原 PPT 继承 2 项、Product 客户端 57 项、Electron 27 项、Python 12 项测试；Web 类型与相关 lint 通过。原 182 个生成/Admin/计费 WIP 有逐文件保护及独立生成证据，本次没有提交它们。内容传输 DTO 按职责拆分，保持 wire 身份；新增 authored source 均在 2000 行限制内。

这是消息协议与持久上下文基础；输入框选择已由上节接入，公开 Agent 检查/修改/导出和消息成片仍在实施；不能据此声称完整功能已可验收。计算内核、组件 pin 和稳定 `v0.1.0` 不变。详细合同及证据索引见产品 `docs/architecture/browser-video-agent.md`。

## 本批增量：原调用的持久版本选择

产品 Host 新增 current/明确 revision 的读取意图。原 invocation/job/conversation、home/workspace/project、authority/组件在同一 strict IDB 事务内绑定，current 首次抵达本地时选定版本；后续人工修改不会改变原调用的重放，新调用才观察新 head。schema 8 只新增准入记录，不复制工程、不缓存核心结果、不修改原编辑历史。随后仍由固定 Office WASM 重新验证快照与计算摘要；原 `browser.video.inspect@1` 的固定版本合同保持原义，新 current Gateway/公开 Tool 尚未接入。

41 项 Host 和 17 项应用适配器测试通过；真实 Chrome 静态构建中，原第 1 版调用在人工编辑至第 6 版及刷新、双标签页重放后仍相同，新调用取得第 6 版。源码集合摘要 `68acb976c778450b704f027c70c3d2729deb39c9e74ca84534f37b2d57a6c960`。全 Web 类型、正式构建/体积与变更 lint 通过；Office 组件及稳定标签不变。

正常产品旧 schema 7 profile 的副本升级到 8 后，工程/素材/历史/回执/导出记录及 home 内容摘要不变。后续下载/重载在该旧 Chrome 配置出现主进程 SIGSEGV，独立静态下载页在旧副本也可复现；全新 profile 首次下载正常，直接重开后再次下载也复现，不能仅归因于复制损坏。仅调整诊断副本的浏览器历史/设置可解除复现，但没有作为产品修复或完整验收。保留成功的数据升级检查点及失败记录，完整旧配置下载/重载仍待调查。产品 `docs/architecture/browser-video-agent.md` 记录合同、复现和准确证据边界。完整目标继续执行，公开 Agent Tool、编辑/导出投递及消息成片尚未贯通。

## 本批增量：浏览器固定版本检查实际执行

产品已将内部 `browser.video.inspect@1` 接至认证应用的本地 Host。正常 Web 打开剪辑后，实际 Worker/WASM 创建并重开临时文档、计算摘要、复核 IDB 所有权与浏览器传输能力，再注册本地 home/authoring pin。根应用保有连接，关闭面板不撤销执行能力；任务临时租用同一工作区，用后回收；退出/更换登录代次立即销毁旧执行器和重试。

工程检查固定原 revision/authority/digest/组件，并重新由 Office 核心验证快照与计算 summary，不使用缓存摘要替代，不读取更新后的 UI head。仅有界 JSON 经既有 Content 端口回传，视频/音频和工程快照留在浏览器。暂存前后校验字节与不可变引用，30 秒截止覆盖元数据上传。Office 计算组件和稳定 `v0.1.0` 均未改变。

真实 Chrome 154 的静态构建检查通过，多页、重载、编辑到 revision 6 后原版检查仍一致。正常产品 `/workspace` 密码登录后，人工保存 revision 3 并关闭面板，实际 PostgreSQL/WebSocket Gateway 两次投递固定 revision 2，实际 Worker 重算、认证 HTTP 暂存与持久 terminal receipt 确认通过；后注册的同账号独立浏览器不接手旧 home，退出登录连接关闭。两次均仅 1,016 bytes 元数据，摘要 `62946a0059d8e000198ea3180d90223ad9391dab7249fdce4a057cb11d54ef11`。产品报告 `video-inspection-normal-route-final/report.json`，源码集合摘要 `13c7164fbd2c2ec70cdb98e67c820eec18070e8549d1dffdfeba89766a81a877`。

Host 32、适配器/生命周期 17、Web 39、真实 PostgreSQL 3 项通过；Web/Host 类型、变更 lint、正式 Web 构建/预算、migration lock 与专用验收发行器 strict Clippy 通过。公共入口 gzip 507,336 / 524,288 bytes。联调修复了开发库已经应用 221 初稿、后续源码改变导致的新代启动拒绝：恢复与已应用 checksum 完全一致的原 221，以新 222 加强约束并锁定两份迁移，未重建数据库或修改迁移记录；正常开发 supervisor 已升级并恢复就绪。

**这仍不是公开 Agent Tool 验收。** 原请求由明确标注的本机测试发行器调用实际 Gateway 产生，没有新增生产后门 API。公开 Tool 原工程选择/准入/结果验证、人机共同编辑/导出、消息附件及全部功能矩阵继续实现。产品 `docs/architecture/browser-video-agent.md` 为执行合同与复现记录；本仓库没有新增业务持久化、身份系统或产品界面。

## 前批增量：产品 Gateway 精确定位

产品新增内部固定版本工程检查协议与 Gateway 路由：用户、浏览器本地 home、authoring 组件、运行版本和 capability 全部匹配才投递。PostgreSQL 221 迁移保存固定目标并验证索引与原请求一致；原页面换 home 不接手旧队列，旧 fence 明确失效。浏览器在领取前再次校验。检查合同仅允许 64 KiB 工程元数据，不能运送素材或成片。

真实隔离 PostgreSQL 3 项、Rust 16 项、浏览器客户端 32 项、Web 22 项通过，Web 类型检查通过。该批完成时产品尚未注册这一能力；本地 Host 检查适配器随后按上节接通，公开 Agent 工具与修改/导出/消息结果仍待接通，不作为完整 Agent 剪辑验收。产品 `docs/architecture/browser-video-agent.md` 记录合同、边界、检查和已有非本轮 Clippy 告警；Office 计算组件不变。

## 本批增量：浏览器本地工程定位

产品 Host schema 7 新增每个登录主体的持久本地 home。同一存储的多个页面共享身份；独立浏览器存储分别持有身份。绑定 target 的元数据事务再次校验 home，错误目标不能读取/创建工程、重放回执或打开素材 writer。路由身份不进入原编辑语义摘要，旧操作仍可幂等重放。Office 计算组件与稳定标签没有改变。

Host/PCM/任务/身份 29 项及适配器/生命周期 16 项通过；静态构建的真实 Chrome 多窗口/独立上下文验证通过。上一版 schema 6 真实产品 profile 的私有副本升级到 7 后，工程、素材、历史、回执和 4 个导出任务的内容摘要不变，原成片仍能播放/拖动/下载。新身份绑定的正常产品操作与真实导出/刷新恢复/关闭面板/退出登录回归通过，媒体仍留本地。产品 `docs/architecture/browser-video-agent.md` 记录源码摘要和复现方式。

这完成本地定位基础，尚未接通云端 Agent 定向投递、回执与消息文件来源。现有 Gateway 按最近在线页面选择执行器，必须在原路由权威增加固定 home 和恢复约束，不能用改名的通用 Runner 假装支持本地工程。

## 本批增量：浏览器应用级导出生命周期

产品应用层接管本地任务：面板的预览/素材读取释放后，已经启动的导出继续；从保存意图之前开始持有执行，避免点击后立即关闭产生孤立待执行任务。切换应用内页面和工作区不撤销原导出；退出登录或登录代次变化同步撤销全部 Host。最后一个视图及任务结束后立即回收 Worker，不累积空闲工作区。

7 项生命周期边界和原 8 项编辑端口、App 5 项入口/导航检查通过。正式 Chrome 154 登录页面明确等待持久 rendering 后刷新，再以原任务 epoch 2 重试；关闭面板切至任务页仍完成；空闲 Worker 全部退出。另一任务在退出登录后停止，再登录为 interrupted，没有成片；点击后立即关闭的新请求仍完成。源码集合摘要 `3624908688dc548a0de4c4d0023529eba5ca6830d50d419c3a1d0b1fe27e2455`，主成片经独立 FFmpeg 完整解码，91 帧、3.003 秒、144144 音频样本的容器时钟准确。Web 类型/正式构建/体积与 ESLint 通过；媒体没有上传。

证据修订：上一轮普通页面报告的异步 waitForFunction 不能可靠等待数据库状态，仅能证明导出期间刷新后可恢复。现改为有期限的 evaluate 外部轮询，以上新报告才证明中断发生在实际 rendering。详细证据、任务状态与复现保存在产品 `docs/architecture/browser-video-export.md`。此增量不改变 Office 计算组件和稳定标签，消息附件/所有会话状态与未读、Agent 浏览器执行、完整最终目标仍未完成。

## 本批增量：产品浏览器持久导出与真实页面验收

共享核心新增有界候选检查，精确校验 MP4 电影/轨道/逐包时钟、短尾和 AAC priming/padding；新 Worker 通过公共接口读取实际成片并抽样解码。核心负责格式语义，产品负责身份、文件、任务和原子发布，详见[核心检查合同](video-browser-output-inspection.md)。

产品将同一保存工程的 GPU I420 与 mixer 接入实际 H.264/AAC 编码，新增固定意图、Web Lock/epoch、持久取消、封存恢复和候选回收。正式 Web 右侧输出仅显示播放器、文件名、下载与返回。真实登录页面导出期间刷新，原任务重试为 epoch 2，取消任务不会复活；仅一份成功成片，1920×1080、91 帧、3.003 秒。独立 FFmpeg 完整解码、精确容器/样本时钟、播放到结束/seek/下载摘要一致。正常页面源码集合摘要 `a0406a39716712c164f91be2cb4f2f0f2d68b8c804ff9e6e56debe49703a10fc`，未调用模型或上传素材。

静态构建 Worker 的正放/倒放工程分别输出 stereo/mono/omit；60 帧，有声轨 96000 样本。独立源 PCM 对照最佳偏移 0，mono/stereo 误差不高于同码率参考 AAC。导出时修改工程不改变输出的冻结版本。22 项 Host/PCM/任务、8 项编辑端口、组件完整性、Web 类型/构建通过。产品 `docs/architecture/browser-video-export.md` 记录完整 pin、复现命令和证据范围。

上述是浏览器本地导出的实际产品进展；后台应用级生命周期的后续完成见上方记录；消息成片附件、浏览器 Agent 通道和完整最终验收继续实施。生产回读采用有界抽样，不将其冒充全帧质量、长素材或跨平台验收。

## 本批增量：浏览器本地编码、精确封装与成片回读

已实现 WebCodecs 视频编码 + 共用 FFmpeg AAC/MP4 核心的浏览器档案，库只编译固定必要闭包。实际 AudioEncoder 调查发现本机首包时间戳没有反映 AAC 的 2112 样本延迟，因此没有写死补偿，改为复用已有明确 priming 的音频与封装实现。共享 Rust 计划继续拥有精确时钟，薄适配器有界绑定 packet、PCM、取消和生命周期。详见[实施选择、合同与证据](video-browser-encoding.md)。

4 个真实 Chrome Worker 成片覆盖分数帧率、短尾、48/44.1 kHz、单双声道和无音轨，独立解码的完整帧数、时间、颜色及音频零偏移通过；14 类 leaf 拒绝和 7 类实时失败/资源回收检查通过。浏览器播放器实际加载、进度跳转、播放到结束通过。同源 Native 6 个真实工程成片和 5 项编码行为回归通过。该增量是编码专项，测试输入为确定性画面/PCM；真实工程视觉/音频执行器、产品持久导出与正式文件提交继续接线，未宣称产品导出或全部目标完成。

## 本批增量：共享导出调度与浏览器编码输入

已把 Native 会话的精确音画交错提取为共享 `ExportSchedule`，新增独立导出规划 WASM；原生实际会话与浏览器规划共用有限帧/样本、短尾及确认规则。实际公共 WASM 的 4 个工程、239 个步骤与 Native 完全一致，一小时分数帧率与音频分块检查通过。WebCodecs 微秒仅用于传输，保留精确时间和残差，不能表示的短尾提前拒绝。

公共视觉组件新增共享 GPU I420 输出，直接复用核心背景合成、SDR/HDR、电视传输与 Lanczos 色度滤波。Chrome 154 / Apple Metal 的 25 组画面共 109080 个有效样本与 CPU 逐字节一致，实际 VideoFrame 保持原始样本；预算、协议错误及取消回收检查通过。原生 6 个实际 MP4（含硬件 H.264/AAC）独立回读回归通过。详见[实现、复现与未完成项](video-browser-export-foundation.md)。当时编码/mux 尚未接通，后续进展见上方记录；持久导出任务及产品文件交付继续实施。

## 本批增量：正常 Web 页面的本地素材与剪辑工作台

产品提交 `026833961` 将本地工作台接入正常 Web 首页与会话右侧面板，只有选择视频创作技能后出现入口。Host 对实际本地文件完成摘要核验、独立 Worker 媒体探测、PCM 物化及持久原始轨道准入；保存 head/历史/回执的同一事务核对真实流描述。相同文件在同一工程中重复导入可通过公共 `MoviePcm.open` 资源绑定能力复用已封存 PCM，物理 pack/时钟/记录保持不变，不能凭摘要获得跨工程权限。本批 Office 公共组件及稳定标签未改变。

Chrome `154.0.8037.98` 在正式 Web 首页、真实隔离后端和正常密码登录下完成：技能选择与 Auto、新建、文件选择、关联音画导入、播放、分割、连续撤销重做、页面刷新后恢复到 revision 8（4 个片段）并继续播放、关闭后 3 个视频 Worker 实际退出、移除技能后隐藏入口和参数。导入至关闭未发现素材上传请求，未调用模型；没有注入 store 或挂载测试页面。应用源码集合摘要 `03746d3e92673b062f81e3748aa62bb3cf6a911faf130656fd39ea2a3d819b2c`，固定真实 B-frame/VFR 素材摘要 `ca7211a9b5e1bbdb66c7bd98e515484d7b6bc5ee8bbeeadb6cabb06d90bd73a6`。

另有开发/静态构建模块检查证明错误素材描述拒绝、刷新后映射恢复与 PCM 复用，两次源码集合摘要均为 `0d87a29e05a4da139d942df05cbe63a21ea1a3f4747ea5b66ae3f14e4d70c7b9`。Host/PCM 17 项、Browser 编辑端口 8 项、共用编辑器 53 项、Desktop 素材端口 4 项、Web 首页/PPT/布局 14 项通过；两端类型、Web 构建与入口体积门禁通过。产品的 `docs/architecture/browser-video-workbench.md` 保留复现命令与证据边界。

浏览器本地导出、Agent 浏览器执行、生成结果进入同一素材工程、图片/字幕/效果/完整编辑属性、跨端工程交接、性能/浏览器矩阵和供应商实际计费继续实施。本次普通页面验收覆盖首页链路，不能代替所有会话/设备、物理音画同步或整个最终方案的验收。

## 本批增量：产品浏览器编辑投递与共享历史

产品提交 `ab1f240cc` 增加浏览器编辑端口、本地工程目录、保存版本分页查询和持久原始编辑意图。临时写入 fence 与语义请求身份分离，旧回执格式保持兼容；刷新、取得新锁或从另一对话打开工程仍恢复原请求，数据库原子保存 head、逆历史、回执、事件和概览缓存。登录/工作区 owner 可以关闭 Host 与 Worker，正常 Web 页面接线继续实施。

8 项新增实际 authoring WASM/IndexedDB 测试与原 16 项 Host/PCM 回归、strict 类型检查通过。Chrome `154.0.8037.98` 在开发和真实构建后静态加载均通过双标签冲突、提交后刷新恢复、原回执重放，以及共用 Vue 编辑器改名/撤销/重做跨页同步；两次源码集合摘要均为 `5d89ae5b5d54a2ff9b59917c3010bc12d3e9687d1168c705369a054070f43cfe`。本批不修改 Office 组件或稳定标签，不代表普通页面、素材导入、导出或整体验收已完成。

## 本批增量：产品浏览器保存工程的真实音画预览

产品 Host 已用同一保存 revision/documentDigest 开启独立画面与声音 Worker。授权源映射及 OPFS 完整校验仍由产品持有；Office 精确选帧、raw color、GPU 程序与 ImageBitmap 输出保持公共组件能力。正常显示不做整帧 JS 回读，bitmap 消费 ACK 之前保留预约，快速拖动只排一个最新意图；超时/失败关闭两条通道。

Chrome 154 的真实 OPFS/IDB 工程在开发加载和构建后静态加载均通过：B/VFR 正反向画面、回访摘要、独立音频、100 次拖动合并、改新版本不改变播放旧版本、取消/错误绑定/退出登录，以及现有 Vue 播放器实际 AudioWorklet 播放/暂停/逐帧/静音。53 项编辑器与 16 项 Host/PCM 回归通过。当前仍是实际产品模块的隔离集成验证；普通 Web 会话页面、持久导入映射、Agent 浏览器执行、图像/字幕/完整效果、导出和整体验收继续实施。内核组件和稳定标签本批未改变。

## 本批增量：AVC 帧解释与随机访问参数

共享 Rust 已按 slice 的 PPS→SPS 引用解析色度/颜色声明和帧信息，容器索引分批扫描、缓存参数变化，并给浏览器新 decoder 恢复目标位置之前的 SPS/PPS。实际生成的中途参数/CABAC 切换素材、六种色度位置和 B/VFR 素材已进入原生/WASM/FFprobe 一致性及新 decoder 随机访问检查；GPU 颜色 gate 不再写死 fixture 的 left。详见 [AVC 记录](video-browser-avc-metadata.md)。完整颜色冲突策略、工程图像 Worker 与音频联合预览、正式页面和导出继续实施，不将该增量算作整体验收。

## 本批增量：浏览器真实媒体颜色与 GPU 图片

已接通原始 VideoFrame 平面→共享 Rust 参数/WGSL→线性合成输入，以及明确的 SDR/HDR 输出与 GPU ImageBitmap 交付。真实 H.264/VFR 的全部 77 帧加回跳与原生逐帧一致；44 组多格式/HDR/alpha、裁切方向和资源回收通过。发现浏览器隐式颜色转换差异后，将完整颜色解释保留在核心，未通过放宽 SDR 误差阈值绕过。详见[实现与验证范围](video-browser-color.md)。生产色度探测、统一宿主、正常页面及完整导出仍未完成。

## 本批增量：浏览器 GPU 帧编译

已将 CPU 像素载体与共享几何/合成语义解耦，实现 `mo-video-gpu-plan`、独立工程视觉 WASM 和浏览器 WebGPU 执行器。原生与浏览器共用同一 WGSL、112 字节定点参数 ABI；异步资源回复仍校验完整绑定与精确时间区间。实际 WASM/native 指令完全一致，Chrome 的 Apple Metal GPU 合成与 CPU 参考最大线性差 `5.961e-8`，原生 29 项硬件门禁也通过。详见[实现、所有权与证据](video-browser-gpu-program.md)。下一步连接真实媒体颜色规范化、GPU 输出与产品统一预览；没有将合成专项视为浏览器播放器验收。

## 固定基线与开发边界

- 已发布 [MusterOffice v0.1.0](https://github.com/wanquanY/MusterOffice/releases/tag/v0.1.0)，固定源码 `a4901290c6ccf6660aabe38d42d2dd650428219f`；对应 CI 44 项步骤通过，发布前源码摘要一致，三个公开附件的远端 SHA-256 与本地一致。这是稳定源码基线，未提升产品组件频道或声明全平台预编译包。
- 用户已明确授权稳定版发布和完整视频方案实施。MusterOffice 主工作区功能分支 `codex/video-native-workbench`；产品保留现有独立 worktree。生产部署、Musterwork 公开发布不属于本目标。
- Musterwork 原 worktree 仍有可复用的 Admin、生成协议/计费和多模态请求绑定；已保存原 HEAD、工作区/暂存区二进制补丁和 42 个未跟踪文件的完整本地快照。保留该 worktree，按公共操作合同替换旧 CE.SDK 路径，不整库丢弃有效代码。
- 当前开发决定见 [ADR 0012](../decisions/0012-agent-video-domain.md)。纯内核候选不代表宿主保存成功，测试辅助绑定不作为产品 SDK 发布。

## 权威方案与工作包

[产品与架构](../design/video-editing.md) · [技术合同](../design/video-editing/technical-spec.md) · [完整验收](../design/video-editing/acceptance.md)。完整目标不缩减；本文件只记实际进展。

| 工作包 | 当前状态 | 退出条件 |
| --- | --- | --- |
| W1 视频领域与合同 | 基础已实现，未全部完成 | 独立模型、精确时间、受限解析与校验、Native/WASM 公共合同向量；补全效果、字幕、分析和工程包模型 |
| W2 媒体与合成 | 原生解码/采样、CPU/Metal 合成、音频/保调图、候选导出及独立真实素材 SDK 闭环已实现；原生已认证导出/正式文件交付已验证；Browser 真实媒体、GPU/音频预览及持久工程导出已接通并经正常页面验收，完整音画/平台矩阵待实施 | 原生/浏览器媒体与合成、字体/色彩/音频及真实输出 |
| W3 编辑与历史 | 原子事务、条件逆操作及 Native/Browser 持久连续撤销重做已实现；Agent 与人工共享历史，全部编辑范围尚未完成 | 全部编辑操作、候选、冲突与有条件逆操作及增量索引 |
| W4 产品执行 | 独立 Native/WASM SDK、Browser Host 基础、Native 计算、SQLite 工程、Agent authoring、共享手动执行、认证 RPC/IPC、工程订阅、基础时间线、消息工程卡片/右侧面板、待确认命令恢复与物理退出等待已实现；固定媒体 runtime/worker、原授权导入、真实原生画面/声音与持久共享历史已接通；原生导出执行/恢复与正式文件内部链已接通；Agent 原调用内真实编码、恢复、原子文件/工具发布端口已接通；正式 export 工具/技能/Device 装配及原 CooperativeExecution 已接通并通过真实编码/重放/取消；手动导出入口、按原请求恢复、版本化重试、简洁状态及原文件跳转已接通；正常登录整体验收待实施 | Device/Browser Host、恢复、权限、任务和不可变组件消费 |
| W5 创作与交互 | 视频技能与显式工具装载、无生成模型的剪辑准入已接通；旧 CE.SDK 页面/依赖已退役，成片简洁预览与原文件下载已接通；完整工作台仍待验收 | 整理现有生成/Admin/计费、技能、人机共享编辑与消息卡片 |
| W6 完整验收 | 已有下列内核与部分产品路径证据，整体验收未完成 | 完整验收矩阵与可供用户实际操作的本地环境 |

## Browser 媒体输入增量

2026-10-08 新增独立 `mo-video-mix-wasm`，通过持有不可变 CompiledDocument 的共享 AudioMixer 接通浏览器工程声音计算；MoviePcm 公开精确资源描述。公共 WASM 与原生叠轨/淡入淡出/变速/倒放/嵌套输出逐 bit 一致，19 项基础测试通过。产品已消费固定组件，接通本地持久 pack、按需素材准备、持续 Worker 混音与现有 AudioWorklet/设备时钟，实际 mono/stereo 工程波形、重载、播放/暂停/跳转通过。完整音画预览、保调派生源、导出、页面和跨浏览器稳定性仍待完成；见[工程音频组件](video-browser-project-audio.md)。

正式 `BrowserMediaInput` 已接通原始 AAC 连续物化、独立 PCM runtime、共享音视频准入账本与可独立关闭的完成对象。真实 mono/stereo 解码、随机读取、写入失败、取消竞态、未完成 I/O 计账和旧视频游标回归通过；仅 EOF flush。产品 `48d8ce464` 已固定消费三个组件，并完成受授权素材读取、Worker OPFS 紧凑 PCM pack、IDB 原子登记、页面重载恢复及半成品回收；实际单/双声道检查通过。详见[影片音频物化接入](video-browser-audio-materializer.md)。该记录之后已接通设备输出/工程混音；合成/导出和完整浏览器体验仍待完成。

影片音频的原生映射/DSP 已提升到共享 PCM 层，新增独立 PCM WASM；真实浏览器对 48 kHz 单声道与 44.1 kHz 双声道 AAC 完成 raw 解码、EOF tiles、影片裁切和随机读取，与 FFmpeg 逐样本比较通过，原生原调用回归通过。发现并拒绝以中间 flush 控制 AAC 队列，保留 decoder 实际 priming/padding 覆盖；详见[共享浏览器 PCM](video-browser-pcm.md)。生产媒体调度、总预算、OPFS 和正式播放/导出接线仍待完成。

ISO BMFF 分页索引及 `IndexedMovie` 已从原生 wire/worker 提升到公共媒体层，新的独立 WASM 直接共用原生 MovieTimeline/VideoTiming。真实 Worker 对 3 个素材共 117 个查询、657 个压缩包记录与原生及独立参照一致；原生 231 帧/1,050 个影片查询回归通过；见[共享影片索引](video-browser-index.md)。后续原始包接线已通过 5 个实际素材共 1,402 次影片查询、1,360 次画面核验，包含多 edit、负 CTS、错误 sync 候选和真实 open-GOP；影片时钟不再重复叠加第三方偏移。串行定位等待前次有界读取退出，取消立即关闭 decoder，见[浏览器影片解码](video-browser-movie.md)。顺序游标/定格复用已将同一正向语料的 decoder 创建从 1,360 降为 6，并验证每次借用独立失效、显式逐出、反压与未完成读取的额度，见[连续解码记录](video-browser-cursor.md)。影片音频、合成/导出及正式页面仍待完成。

独立 `video-browser-media` 组件消费固定 Mediabunny 原始源码与 WebCodecs，直接接受 Rust 的精确时间/边界方向。真实 Worker 的 77 帧 B-frame/VFR 视频、16 个边界定位、144,384 个音频采样值与独立 FFmpeg 对比通过；修正 AAC 从中间 key 包直接开始导致的波形差异，并验证回调不结束时的超时和物理资源释放。新增 progressive AVC 解码容量/共享字节准入、压缩包预准入与有界输出队列，真实暂存 VideoFrame 峰值由 42 降到 3；100 次顺序音频窗口复用 1 个 decoder。SDK 包含完整对应源码与许可，按外部 manifest pin 验证。其他视频 codec 的容量模型、随机音频 seek/总音频预算、影片音频映射、Web 合成/导出及正式页面接线仍未完成；见[记录与复现](video-browser-media.md)。

## 成片预览与旧引擎退役增量

产品已移除 CE.SDK 页面、依赖和旧 JS scene 领域，保持 MusterOffice Rust 领域与 Native/Browser Host 为唯一工程权威。旧源码先保存，浏览器历史库和原素材没有删除；没有声称私有 scene 已无损迁移。

现有共享文件预览采用简洁原生播放器，支持播放、暂停、进度跳转、下载、错误重试及关闭后释放媒体。消息卡与会话文件面板共用原文件 Owner；桌面下载固定最初的会话、文件版本与 ContentRef，生成视频 JSON 解析到该版本声明的实际原文件，并发复制不会覆盖已有文件。Web 沿用原下载能力。这些属于产品接入，没有向 Office 内核增加账户、文件权限或 UI 职责。

70 项前端/文件归属检查、66 项 Electron/生成合同检查以及两端类型/依赖边界检查通过。真实 Chrome 154 加载原生导出的 3 秒 H.264 视频，完成播放、暂停、定位、窄面板、错误和释放检查；与独立解码同一呈现 PTS 比较，RGBA 平均绝对误差为 3.687622。该检查为生产组件与隔离媒体源，不是正常登录会话整体验收。产品原文件大媒体传输、Browser 编辑媒体、完整编辑属性和供应商计费仍须继续完成。

后续实际登录隔离开发库，经过原 Device 注册、授权素材读取、音画单事务导入、导出准入/重放、正式 Artifact 发布和原文件下载，得到 361895 bytes 的 H.264/AAC 成片。独立全片解码通过：128×96、76 帧、3.003 秒、48 kHz 双声道；测试身份、临时资源已清理。该验证不调用生成供应商，窗口自动操作因宿主控制权限缺失仍未完成；没有将无 Renderer 联合证据算成正常窗口验收。

## 产品素材导入增量

已将精确视频素材描述接到原 Product 命令、Device 视频工具 v2 和后台组件配置，保留原文件权限与物理计算额度。视频创作技能按实际探测结果形成导入声明，消息呈现继续复用工程卡片，纯探测不生成虚假卡片。详见[产品媒体接入记录](video-product-media-import.md)。103 项定向测试、类型检查与生产库 strict Clippy 通过，实际检查结果在该记录和产品本地报告登记；播放器、完整导出 UI、精确音频和 Browser 媒体仍未完成。

产品 `e63ba6a83` 接通共享文件选择器和原生探测后的单事务画面导入：一次撤销、原授权、精确尾段与冲突/不确定回执恢复均沿用已有机制。30 项定向测试及真实浏览器检查通过，浏览器产生的事务另经实际 SDK/SQLite 保存并渲染首尾帧；详见上述媒体接入记录。该轮仅导入画面。

后续产品 `f391b4bb4` 固定消费 `.20261007.4`，接通精确音轨声明、原声选择、音视频关联导入和整组手势编辑。94 项定向测试通过（含 8 项显式真实媒体测试）；另有 1 项既有 library stress 测试 ignored，不计作通过。浏览器交互和原生消费实际浏览器事务分别验证，仍不是登录 Electron 端到端。PCM 持久物化、图片、声音播放、音画同步和完整导出继续在目标内。

## 产品原生画面预览增量

产品 `4b93c4fac` 已接通固定 SDK/原权限存储到本地双向流、Electron 和共享右侧面板，支持精确定位、画面播放和物理回收；当前明确为无音频画面预览。136 项定向及协议测试通过，实际 Chrome 组件验证通过但未代替登录产品整链路。详见[产品预览记录](video-product-preview.md)。

## 已实现的计算基础

当前产品增量：`f7e585669` 已通过真实已认证本地服务、原生 worker、Electron、共享 Vue 和 AudioWorklet 的隔离联合播放检查；`60dccdee3` 已接通 Native/Browser 持久连续撤销重做及 Agent/人工共享历史。见[最新播放证据](video-product-preview.md#真实原生联合检查)和[共享历史证据](video-product-history.md)。下列各阶段中的“待接入”描述保留其当时边界，以本段和工作包表判断最新状态。

`276e2ab26` 已通过真实 SQLite 工程到本地硬件 H.264/AAC 的冻结版本导出候选，包含原权限、共享媒体源、实际输出校验、取消/物理回收和独立整片解码；详见[产品导出候选证据](video-product-export.md)。后续持久任务存储与启动恢复已接通，详见[导出生命周期](video-product-export-tasks.md)；[真实候选持久保存与恢复](video-product-export-content.md)也已接通，[手动正式文件原子发布](video-product-export-publication.md)内部端口随后接通；后续[原生执行与恢复](video-product-export-execution.md)已串联真实媒体到正式文件的执行链和 resident 装配；后续[Agent 原运行归属与原子提交](video-product-export-tool.md)内部端口已实现；后续[原调用内真实原生导出](video-agent-native-execution.md)已通过原文件引用解析、取消/恢复及真实编码验证，公开工具/原 CooperativeExecution、消息卡片和公开 UI 仍待接线。

### 容器音频时间映射

已提取音视频共用 MovieTimeline，新增真实解码 PCM 检查、原始音频到 movie 时间的空白/重复/变速映射，以及隔离索引 worker 的有界编辑表接入。实际 AAC 从 145,024 个 raw 采样帧排除前置 1,024 帧，得到精确 3 秒的 144,000 帧，与固定 worker 新解码的正常播放结果逐样本相同。66 项常规回归和 1 项显式原生检查通过；strict Clippy 和 WASM 编译检查通过。干净 `636ad72` 已构建本地 `.20261007.4` 开发 SDK，独立 registry archive 消费者又验证了实际解码和 32 个随机窗口。详见[音频时间映射记录](video-audio-movie.md)。产品固定 SDK/原声声明与关联导入已接通，设备播放及音画同步未完成。

### 面板预览的独立像素网格

共用 CPU/GPU 合成已支持在保持工程坐标、源时刻和嵌套裁切语义的情况下直接渲染目标网格。新用例还修正了 GPU 在透明边界的微小插值权重丢失，档案更新至 GPU f32 v2。54 项常规检查、Native/WASM 15 查询、实际 GPU 96 组与原 29 组回归、五份素材的 1,420 张缩小预览及 710 帧原尺寸导出验证通过。独立 SDK `.20261007.3` 已从干净 `dd64f86` 打包并由 Musterwork `bedef797d` 固定消费，独立 archive 媒体消费者与 43 项产品检查通过。该轮尚未接入产品播放器；后续原生画面预览进展见上节，音频/完整导出仍未完成。计算网格细节见[预览网格记录](video-preview-viewport.md)。

### 视频高层接口与独立 WASM

`mo-video-operations` 已实现不可变工程创建/打开、摘要、分页、候选编辑、条件撤销、帧计划与资源闭包；`mo-video-wasm` 和薄 TS 客户端共享同一合同，独立于 PPT 和媒体 codec。宿主 revision、epoch、幂等持久提交仍由产品负责，候选不等于保存成功。已有不可变开发包构建和实际 Native/WASM/打包 ESM 检查；尚未发行生产 SDK 或接入工作台。详见[接口及实际边界](video-authoring-api.md)。

### 产品 Browser Host 基础

Musterwork worktree 提交 `82aaa621b` 已接入固定的独立视频 SDK（上游源码 `45ef17cdda471fef732412cf118829fbb325f035`；manifest `634c9790b861b8ddfc232a879952264f93eed949c708075729ba95edf7eecf98`）。产品新增宿主包，持有登录 session/作用域、严格 IDB 事务、租约/epoch、CAS、幂等回执、条件撤销历史和 outbox；OPFS 素材分块写入并验证后登记，提交时核对资源闭包。内核没有增加业务存储职责。

8 项真实 Rust WASM 驱动的宿主单测通过（该组 IDB 为 fake-indexeddb）；另在 macOS arm64 / Chrome 154.0.8037.98 实际执行 Worker、IDB、OPFS：并发只提交一个版本，2,097,169 bytes 素材在页面重载后完整恢复，epoch 1→2 拒绝旧执行器，Worker 终止拒绝待执行请求。产品本地报告 `.codex-work/video-host-browser-current/report.json` 绑定源摘要 `f153cb4b09cc2e925d28e32bdf6372193219b4ddba0f90a7ab6e62b942ac446b`。

这仍是产品宿主基础，未连接现有时间线/Agent 工具或浏览器媒体设备；桌面必须由 Device Runtime 实现权威，不能直接复用 Browser IDB 作为桌面工程。当前完整快照历史、孤儿素材回收、容量管理、备份、断电恢复及跨浏览器尚待完善。CE.SDK 页面还未退役，不以这些测试宣称工作台可完整验收。

### 原生 SDK 与产品计算接入

上游 `c4c0d8d0390e743c261244396c15849c654c0d13` 已构建独立六 crate Cargo 开发包，版本 `0.1.0-video.20261006.1`、manifest `8281dac6009e7bb5b4836211d4e8b759b7603b6ee47e832266031ed334f8507f`。构建来自干净源码，包含 LICENSE / NOTICE 和组件许可；无 PPT、codec 或宿主依赖。实际独立消费者仅使用 registry archives 构建，运行创建、编辑、条件撤销、查询和拒绝检查；干净包报告 `.codex-work/video-rust-consumer-clean-c4c0d8d/report.json` 通过。生产发行资格仍为 false。

产品提交 `b7c5a102c` 的 Native adapter 已通过真实固定 SDK、SQLite、原 Attempt/Invocation 的六项集成测试：轨道/片段与帧计划、精确输入读集合、先预留全部输出再写入、验证落盘字节、确定性重试、权限/额度拒绝、条件撤销/重做、部分预写失败恢复和异步丢弃后的并发额度保持。PPT 与视频按不同来源各自核验闭包，旧 PPT pin 未改变。产品 Runtime 持有权限和私有输出，内核没有增加业务存储。相关 Office/SQLite 回归共 43 项通过（其中包含视频用例）；另 50 项依赖专门渲染/素材的 ignored 用例未在该轮运行，不计作通过。

该计算适配器仍只返回候选，不直接创建 head/history 或提交 Tool / Artifact。另有下述 SQLite 持久端口，产品预写和 Agent 原调用提交已有底层接入；完整资源授权与公共执行器仍未完成。视频 Agent 工具、桌面 IPC、UI、流式大媒体和播放/导出接入仍待完成。产品对原始 read_refs 和完整资源绑定的最终授权不得以计算摘要替代。

### Device 工程持久化端口

产品提交 `790b7ddf0` 新增独立 VideoProjectId / VideoOperationId 和私有用户工程作用域，SQLite schema 196 原子保存 head、不可变历史、幂等回执、完整资源引用库存与 outbox。持久 authority UUID 保持跨重启稳定；epoch 直接复用现有 Device Runtime writer generation，未创建第二套租约系统。旧进程的读写均被现有 fence 拒绝。同一操作在新进程重试可找回原回执，改动素材、组件、来源或基准版本会导致幂等冲突。

产品与 Agent 来源进入统一历史合同，但产品提交端口明确拒绝 Tool 来源，避免绕过原 Invocation / Ledger 事务。按操作 ID 可直接恢复丢失的回执；事件可按连续序号补读。更新通知由原 Device Runtime 的物理 writer 释放回调持有，各适配器共享通道，事务回滚不发通知，watch 值不冒充持久事件序号。引用登记使用按 ContentId 索引的集合 SQL，避免在写事务内逐项执行大量查询。隐私删除继续生效，历史证据不被当成已删除媒体的访问授权。

存储和迁移验证覆盖并发唯一胜者、事务末尾失败完整回滚、作用域隔离、规范字节/索引/摘要核对、内容身份/删除、旧进程失效、重启后的原回执、事件缺失拒绝和真实历史 schema 195 升级。这里的内容字节是存储 fixture；实际 SDK 语义由前述独立计算集成验证，不将 fixture 声称为媒体播放证据。

另有真实固定 Rust SDK 与存储组合验证：保存创建和修改后的工程，重启后逐一打开原/新快照、恢复逆操作报告，并核对没有创建虚假 Run / Invocation。10 项存储/迁移、该组合测试、6 项提交回调、6 项原计算、39 项原生编辑/重试及 1 项桌面组件控制回归共 63 项通过，无 ignored；SQLite lib 及依赖 strict Clippy 通过。产品证据 `.codex-work/video-native-project-persistence-report.json` 固定提交、组件摘要及日志 SHA-256，并明确测试包含原 worktree 尚未提交的视频选择改动，不声称是干净 HEAD 或完整产品验收。

完整 Host 仍未交付：手动编辑输出预留、Agent 原调用与工程同事务提交、受授权资源准备、公共执行器和桌面 IPC 已由后续增量补齐（见下文），时间线仍须接入。历史目前引用完整快照，历史压缩、工程删除/保留政策、GC、大媒体和断电/磁盘满验证仍待完成。产品详细边界见 worktree `docs/architecture/native-video-project-persistence.md`。

### 手动产品命令的原生预写

产品提交 `cc6ac3c7f` 将手动编辑绑定到原 CommandId，新增不可变命令内容账户及输入/快照/报告角色预留；不为 UI 编辑伪造 Run 或 Invocation。Native 计算仍共用原 OfficeHost 的并发预算，Agent 保持原 Invocation 路径和输出身份。SQLite schema 197 保留全部既有 schema 和稳定 authority。

输入与输出实际写入前后核对私有作用域、精确当前快照、expected head、原 Runtime writer epoch 及持久额度。重试只能复用原身份；读取限于该命令已 ready 的对象与当前工程快照。真实慢输入期间发生另一项工程提交时，旧命令不能登记 ready 或成功回执；I/O 不占用 SQLite 写事务。重启后新进程可在同 head 上恢复原账户，旧进程失效。

新增 6 项真实固定 SDK 与 SQLite 集成验证，包含创建、编辑、保存、私有读取、错误所有者、候选输入/输出绑定、原子额度、错误字节和并发版本变化；连同迁移、Agent 计算、工程存储、原流式 I/O、Invocation 与 PPT 预写共 43 项通过，无 ignored。SQLite/MusterOffice lib 及依赖 strict Clippy 通过。产品 `.codex-work/video-product-command-report.json` 固定提交和各日志摘要；保留原 worktree 未提交的视频选择/生成修改，不冒充干净 HEAD 全量测试。

这是内部预写端口及 typed 候选绑定，还不是公共手动编辑命令、素材授权执行器或页面。完整 SDK 资源清单匹配与 Agent 原调用的同事务提交现已补齐（见下节），素材当前访问权限与公共执行器仍须接入。产品详细边界见 `docs/architecture/native-video-product-commands.md`。

### 完整资源身份与原 Agent Tool 提交

产品提交 `f0cb26fdb` 将真实 SDK 的完整资源清单带入计算结果，要求工程绑定逐项匹配资源 ID、字节长度与 SHA-256。未使用素材与字体也必须保留，漏项/重复/替换/乱序均拒绝；超过 4,096 项的清单在任何输出预留前失败。7 项产品命令、7 项原计算与 1 项持久组合测试通过，无 ignored；其中真实 SDK 的 4,097 字体声明验证超限路径。身份匹配不是访问授权，素材导入与授权执行器仍需实现。

产品提交 `de975a0c8` 由原 `SqliteFencedToolInvocationJournal` 实现视频提交。工程 CAS/revision/引用/outbox、原 Tool 成功、Ledger、原执行活动、对话内容根和幂等提交身份在同一物理事务内保存。继续使用原 Attempt、Invocation 和 liveness owner；没有另一份视频任务或工具完成状态。私有工程 actor/workspace 与原 Run 的真实请求用户核对，schema 198 用联合外键绑定 tenant/run/invocation。

实际验证包括：提交末尾故障时整笔回滚且不发工程通知；原工具结果精确恢复；head 前进后仍返回原操作的回执；真实产品预写和 SDK 手动编辑先保存后，旧 Agent 候选冲突且不产生工具成功；旧 Attempt/epoch、其他用户、未预留结果、取消、已经普通完成的 Tool 均不能收养工程写入。素材继续由工程历史保留，不因工具成功向对话发放全部素材权限。

本轮去重后 32 项测试通过，无 ignored；联合外键收紧后的 5 项提交和 1 项迁移复跑不重复计数。SQLite/MusterOffice lib 及依赖 strict Clippy 通过。证据分别为产品 `.codex-work/video-resource-closure-report.json` 和 `video-tool-commit-report.json`，固定提交和日志摘要，并声明原 worktree 未提交修改的边界。提交测试复用已有已准入 Tool fixture，不能当成正式视频技能/工具已经发布。

这些提交记录的是内部宿主端口；后续正式 Agent 执行器、技能、素材授权及用户命令/IPC 进展见下文。原生媒体 Host、消息/右侧面板及时间线仍待接入，完整本地产品验收尚未达成。详细约束见产品 `docs/architecture/native-video-tool-commits.md`。

### 正式 Agent authoring 与 Device 装配

产品提交 `fcc171179` 新增 Device v69 的 `artifact.video.author@1`（wire `video_author`），仅在 desktop surface 注册。完整 SDK 编辑 schema 从固定的 registry crate 导出，产品增加工程 pin 和已有文件引用合同；旧工具/技能材料保持不可变，Cloud head 仍为 f52。原 material source、授权、Attempt、停止和 journal 负责实际调用，Device 不需要配置 PPT 渲染 worker 即可进行视频 metadata 编辑，两种办公场景共用 native compute 并发额度。

工具支持 create、inspect、list、frame plan、edit、undo 和 history；查询与模型回复有界。新素材经原 FileQueryService 验证精确文件身份和版本，计算后再次核验；不接受裸 ContentRef/URL/文件路径作为导入权限。撤销由宿主读取同一私有工程历史的原 inverse，验证原始基线与结果；删除素材的恢复只使用该历史前驱保留的资源。候选仍由原 Tool/Ledger 同事务提交，无新的视频任务 owner。

新增 5 项实际 SDK/SQLite/正式工具合同集成测试，加 63 项 catalog、3 项 Device manifest、3 项 Office 生命周期、2 项诊断回归及 5 项 Python profile，共 81 项通过；adapter、Tool executor、Device runtime strict Clippy 通过。Office 恢复使用实际固定发行 worker，显式运行集成测试而非跳过；产品提交 `726a470d6` 同时修正了该旧测试前置条件和大尺寸诊断错误的内存表示。

证据为产品 `.codex-work/video-agent-authoring-report.json`，绑定日志摘要和提交；原工作树未提交的生成/视频选择改动继续保留。集成 fixture 沿用原合成 Run compute placement，素材权限变化使用受控 FileQuery authority 注入；这不是登录桌面或真实模型验收。工具 `frame` 返回合成计划，不是视频图片/文件；媒体导入/probe、大文件、播放和导出仍待接入。技能自动装载、用户手动入口、桌面 IPC 与右侧工作台未完成。详细合同见产品 `docs/architecture/native-video-authoring.md`。

后续产品提交 `af81cfc6f` 已补齐源码中的技能装载：Skill v52 新增 `musterwork-video-creation@1`，以可选依赖连接本地 authoring、视频/图片生成和文件读取。原 ProductSnapshot 的显式技能选择现在依据精确 capability identity 装载本次已授权的工具；缺少可选能力不封锁剩余流程，也不补造权限。旧 Skill 材料不改写。

输入框 Auto 在无生成模型或目录暂时不可用时仍能发送已有素材的剪辑请求，参数仍只在技能选中时出现；手动指定模型的版本和组合继续严格校验，未静默改成 Auto。Backend Auto 消息准入不依赖生成数据库；真正付费生成继续走原签名能力/用户约束/计费边界。37 项 ProductSnapshot、13 项 Skill、7 项 Python、10 项发行校验器及 5 项 Vue 控件测试共 72 项通过；strict Clippy 和共享 UI 类型检查通过。全部 93 份历史包材料校验通过，当前 HEAD 锚定检查包含新增包；本地报告为产品 `.codex-work/video-skill-report.json`。这些是实际源码与端口/组件验证，不是已完成账号启动、模型对话、右侧工作台及整个本地验收。

### 手动执行、共享历史与认证桌面命令

产品提交 `a00b9a14b` 新增 `NativeVideoProductCommands`，与 Agent 共用请求解析、精确版本规划、SDK 调用、FileQuery 素材授权和历史 inverse 验证。用户编辑使用原 CommandId/产品账户，不伪造 Run 或 Invocation；双方提交进入同一工程、资源库存和历史。原命令的精确字节及作用域绑定不可变 operation identity，丢失回应后即使 head 已前进，仍可恢复原回执。

`LocalVideoProjectService` 在原认证 UDS 上装配，resident product authority 绑定 User Actor，外部请求不能自行指定 tenant/principal。桌面主进程复用原协商 channel；IPC 注入真实 renderer sender，取消、页面关闭和 Runtime shutdown 传播到原执行。Renderer API 固定原命令身份和字节，结果不确定时不自动重发新命令。公开请求 metadata 上限4 MiB、结果64 KiB、执行60秒；媒体不通过 JSON 传入，也不依赖 PPT worker。

四项真实 SDK/SQLite 手动执行测试、五项 Agent 回归和一项实际人机交错测试通过：用户先保存后，已准备的旧 Agent 候选被拒绝；Agent 读取最新 head 继续修改，双方能相互撤销原历史。五个工程 revision 仅有三个真实 Agent commit，Run 仍为原来的一个。连同30项 Device transport、80项 Electron 通信/命令/IPC scope、两项 Renderer、15项协议/跨语言检查，共137项通过，无 ignored；相关 Rust production lib strict Clippy、Renderer API 类型检查通过。扩大到 all-targets 的检查在既有 artifact 测试 `unwrap_used` 上失败，没有宣称通过或增加 lint 抑制。

产品 `.codex-work/video-product-execution-report.json` 固定源提交、原 SDK manifest 和日志摘要。协议检查包含原 worktree 未提交视频选择字段及对应生成锁；本次只提交独立 local_video 合同，暂存中的88个 proto 另经实际 vendored protoc 构建并与暂存 descriptor 逐字节一致。受控 gRPC fixture 验证通信，不能冒充真实登录产品。该提交时，时间线页面、工程变化订阅、待确认命令恢复 UI、媒体 probe/大文件/播放/导出和浏览器跨端仍待完成；观察订阅的后续增量见下节。未发布新 SDK 或宣称完整验收。详细合同在产品 `docs/architecture/native-video-product-execution.md`。

### 原生工程变化与桌面观察

产品提交 `ffc306321` 将同一 SQLite 工程的 head 更新接到 `LocalVideoProjectService.Observe`、Electron 和 Renderer API。Actor 仍由可信 resident 绑定；小型 head 查询保持原用户/租户/workspace 隔离和 writer fence，不读取完整快照或新建另一份工程存储。

首次订阅先注册再读取，避免提交竞态；后续通知可合并，以原库当前 revision/digest 为准。原生/桌面订阅有容量和取消边界，桌面采用 listener 安装后激活、单条待确认消息加最新值合并，避免复杂连续编辑挤满 Renderer 队列。断线只恢复读取当前状态，不回放编辑；uint64 全程精确保留。慢视图处理不会覆盖后到的断线状态。

真实 SQLite、实际 SDK 人机交错、Native transport、Electron UDS/IPC、Renderer 和协议共122项检查通过；相关 production lib strict Clippy、Device 编译及 API 类型检查通过。暂存的88个 proto 独立重建，与提交 descriptor 字节一致。证据为产品 `.codex-work/video-observation-report.json`，详细合同为产品 `docs/architecture/native-video-project-observation.md`。

这完成的是工程观察通路；尚未连接真实时间线、播放、声音、导出或浏览器界面。不会以这些通信与存储测试替代完整本地产品验收。原本的 CE.SDK 工作台仍待有迁移保护地替换。

### 共享时间线组件与原生桌面端口

产品提交 `7de84c29d` 新增 `product-video-editor`：基于固定 SDK 操作与分页类型的 Vue 时间线、短生命周期视图状态，以及绑定原认证命令/观察 API 的桌面容器与端口。只有宿主持久化工程；前端不建立另一套文档 reducer 或历史。图层和片段按可见范围读取，同一视图必须绑定同一 pin，查询中发生更新时不拼接不同版本。

当前组件支持改名、轨道/时间范围分页与缩放、选择、移动/修剪、逐帧键盘调整、分割、删除、共享撤销/重做、添加轨道与图形色块。拖动中冻结原 pin；Agent 新提交先提示，松手仍以原 pin 尝试一次原子操作，冲突后重读。结果不确定时保留原 CommandId/字节恢复，不能用旧回执回退当前视图；失效订阅可重新连接。

8项共享 session/实际 Rust WASM 集成与8项桌面适配/原命令观察测试通过。真实 Chrome 154 使用生产 Vue 组件及显式内存 Host fixture 验证鼠标移动/修剪、键盘微调/Esc、撤销重做、分割、改名、缩放、密集视图恢复和窄布局；浏览器脚本没有连接真实 Native resident/SQLite，关联选择另由实际 WASM 集成测试验证。类型检查、前端依赖边界和从暂存清单独立执行的冻结/离线依赖锁检查通过。

证据为产品 `.codex-work/video-editor-report.json`，截图与浏览器报告在 `video-editor-browser-final-02/`，合同及复现说明在产品 `docs/architecture/native-video-editor-surface.md`。Native 源与 Browser SDK 源的本次使用 schema 已比对一致；没有新发 SDK。

该组件提交时尚未进入消息工程卡片/正常路由；后续接线见下节。真实素材 probe、像素/声音、导出、完整属性与 Browser Host 仍未接通。该阶段待确认命令仅保留于活跃 session，后续持久恢复进展见下文。预览缺失和未装配的动作不会被显示为可用；CE.SDK 旧路径及场景数据仍待迁移，不能据此宣称全部产品功能可验收。

### 消息工程卡片与正常右侧面板

产品提交 `fac4ea520` 将原生剪辑工程接入正常会话：成功创建/修改/撤销的工具事实生成回复下方的简洁工程卡片，点击经原 Surface 调度和会话路由打开现有原生时间线。真实视频文件仍走文件播放入口；工程不是 `.mp4` 或 Artifact，不伪造播放/下载地址。失败、结果未知及只有输入参数的调用不能显示成功工程卡片。

同一回复按精确 uint64 revision 去重，旧回执不能覆盖新版本；长回复最后一个虚拟分段仍能展示较早工具产生的工程。历史卡片只携带工程身份，打开后由原受认证端口 inspect 当前 head。面板持久化 project/conversation 和原会话 scope，不保存另一份文档、历史或旧 pin。

工程 scope 直接来自原 `AgentConversationView.workspace_id`，明确区分工作区 UUID、个人范围 null 和尚未解析的状态，不能把 UI 默认目录当成旧个人会话的工程归属。不同 scope 不复用面板 view state/payload。Web 看到桌面本地工程入口时明确提示在创建它的桌面端打开；生产 Browser Host 仍待接通。

99项共享展示/PPT/旧工具检查与84项 Desktop 路由、组件、原会话归属检查通过，共183项。贯通检查运行真实消息组件、面板调度、路由、原生编辑器与命令准备，验证最终发出原 scope 下的 current-head inspect；仅 IPC 端点使用测试 fixture，未宣称连接真实登录 resident。共享与相关 Desktop 类型检查、前端边界检查通过。schema 展示审核覆盖已登记的历史及开发 release，未修改全局激活配置或发行 SDK。

证据：产品 `.codex-work/video-project-surface-report.json` 绑定提交、33个源码路径及6份日志摘要；细节在产品 `docs/architecture/native-video-project-surface.md`。旧工作树修改保留，检查基于含这些既有修改的工作树，本次仅提交自身文件/片段。整体验收仍未完成：真实媒体预览/声音、素材导入、导出、完整属性、Browser Host、CE.SDK 迁移和真实 provider/计费仍须完成；命令恢复增量见下一节。

### 待确认命令持久恢复与原生执行生命周期

产品提交 `b614f52cb` 在正式 Desktop 编辑端口接入写前命令意图：IndexedDB 严格事务先保存原 CommandId、原 JSON 字节和原作用域，再发送原生调用。工程/历史/回执的权威仍是原 SQLite；客户端不保存第二份文档或判定提交成功。Web Locks 串行同账号同工程的不同窗口，其他命令不能覆盖未确认意图；已由另一窗口确认的旧命令对象不能被重新放回槽位。关闭面板后可重开恢复，允许用户离开等待界面。

记录按稳定账号隔离，适配器额外受当前登录 session 约束。同一账号重新登录可恢复原意图，旧 session 和其他账号不能接管。存储失败或超额时不发送新编辑；响应未知、取消或成功后的槽位清理失败都保留原命令。恢复旧回执后重新读取当前 head，保留 Agent 后续修改；撤销类型也持久化，恢复后仍可进行受版本约束的重做。

原生 RPC 不再用等待超时直接丢弃正在提交的 future。有限在途表合并精确相同请求，最多8项物理执行、每项8个等待者；最后一个等待者消失仅请求协作取消，物理工作直到实际结束仍占额度。TaskTracker 在调度前登记任务，服务退出封闭准入并等待物理工作结束；超过退出等待期限明确报错，不能把连接结束称作全部编辑结束。原持久回执、SQLite CAS、epoch fence 与 Agent 提交归属未改变。

40项 Device transport、4项真实固定 SDK/SQLite 手动执行、15项共享 session/实际 Rust WASM，以及8项 Desktop 端口/工程作用域/正常消息入口检查通过，共67项。真实 Chrome 154 的11项 IDB/Web Locks 检查覆盖刷新、Renderer 崩溃、浏览器进程重启、多窗口、账号变化、作用域拒绝、quota 故障注入和容量边界；另13项生产 Vue 时间线交互回归通过。Rust production lib strict Clippy、共享及相关 Desktop 类型检查、前端/Docker/桌面依赖边界全部通过。

报告 `.codex-work/video-pending-edit-report.json` 绑定提交、30个路径（含改名旧路径）与10份日志摘要；详细合同在产品 `docs/architecture/native-video-edit-recovery.md`。浏览器重启使用同一 profile/origin；没有冒充操作系统断电、真实磁盘耗尽或登录桌面全应用重启/升级验收。原生物理生命周期由受控阻塞端口验证，SDK/SQLite 单独集成验证；真实媒体和完整本地产品验收仍待完成。下一工作重点为固定媒体 SDK/worker 消费、原素材导入/probe、像素与声音预览及本地导出。

### 固定媒体运行时、worker 与授权探测

本仓库提交 `bfc9398` 新增 `mo-video-runtime` 公共入口，复用原有工程、采样、合成、音频和导出类型，默认无 native/GPU feature。独立 `runtime` registry 档案包含 19 个自有包，原 6 包 authoring 档案继续存在；组件构建、消费和 source/index/checksum 检查不引用产品或相邻源码。独立消费者从真实素材创建工程、修改叠加层、定位、预览并导出 75 帧，独立解码的 1,382,400 字节 YUV 与编码输入完全相同。另验证 AAC 144,000 样本；导出无音轨，GPU 在此消费者只编译，不扩大验收结论。详见[媒体 SDK 记录](video-runtime-sdk.md)。

Musterwork 提交 `89c9c1d7e` 固定 `0.1.0-video.20261007.1`、manifest `d26836cfaa20f356de1d87917cc5b3d6435fb6255bcbc7cda75e44667afe9227`。PPT 固定版本与包身份保持不变；新旧视频发行的 6 个编辑领域 crate 源码相同。实际产品检查包括 12 项准备器/版本测试、4 项真实编辑/权限集成、1 项持久重开及适配器 strict Clippy。

产品提交 `2f40ad4fd` 新增明确来源的 worker 安装器及 `NativeVideoMediaHost::probe`。已构建并安装 macOS arm64 release worker，7 个二进制共 8,152,280 bytes；这不包含应用、GPU、字体和模型。逐文件、相对 loader alias、目录与 SDK 绑定验证后原子安装；缓存损坏明确失败。原生 SDK 在启动子进程前再次核验。21 项安装完整性/并发测试通过；上游消费者使用**产品已安装的 release worker** 重跑实际导出和独立解码，结果相同。

probe 通过原 FileQueryService 和 VerifiedContentReader 读取原授权文件，并在返回前复核来源及权限，不接受 digest/路径代替授权。计算共用原 Office semaphore/WorkScope；调用方离开后，物理工作结束才释放额度。2 项显式实际 SDK/worker/SQLite 测试通过（0 ignored），覆盖流信息、权限晚撤销、来源变化、输入限额、取消、验证后字节被修改和阻塞读取期间调用方退出；既有 4 项编辑回归与生产/新测试 strict Clippy 再次通过。共 40 项不同的产品检查通过，重复运行不累计。

当前仅有内部授权探测入口，不是正式 probe RPC/技能或用户导入完成。容器 duration 不证明连续呈现区间；通用 demux/edit-list 索引、内容存储大素材支持（当前沿用 264 MiB + 12 bytes 上限）、真实像素/PCM 播放、输出校验发布、完整属性 UI、Browser 和 CE.SDK 替换仍须完成。报告为产品 `.codex-work/video-media-runtime-report.json`；保留原 worktree 未提交的生成/Admin/多模态代码，不宣称干净 HEAD 全应用验收。

### 容器原始样本与编辑时钟索引

`mo-media-isobmff` 新增基于有界 `ReadAt` 的非分片 ISO BMFF 解析；跳过 mdat，保留 DTS/PTS、压缩字节范围、初始化描述、原始 edit list、显示矩阵及各自独立的声明/解码时长。未支持的加密、外部引用、分片、composition shift 等语义明确拒绝，不推算连续呈现范围。7 项 Rust 检查覆盖预算、取消、畸形/截断数据、超过 8 GiB 的稀疏虚拟来源及 256 组独立布局对照；strict Clippy、WASM 编译与 6 项验证驱动测试通过。

3 份真实 MP4/M4A 共 390 个压缩样本与独立 FFmpeg demux 的时间、位置、大小、同步标记和字节摘要一致。发现参考工具会填充 nominal packet duration，验证通过实际 DTS 差和独立解码区间比较原始 stts，同时保存差异。容器声明、解码总时长和编辑终点不能混用。证据、限制与复现见[容器索引记录](video-container-index.md)。本次尚未接入受限 worker、产品导入或 Browser Host；现有 SDK 和产品固定发行保持原状。

后续提交 `3c95c60` 已接入受限 worker：`NativeIndex` 与 decoder 共用原读取/取消/回收 owner，在子进程保留不可变索引，以 128 条记录或 4096 字节分页查询，继续使用 64 KiB 控制消息上限。协议升为 profile 3，源码 runtime 包闭包增为 20；产品原 19 包 SDK/profile 2 worker 配对未改变。干净提交构建的 macOS arm64 release bundle 实测 8,270,552 二进制 bytes。3 份真实素材的所有分页结果与原解析/独立 demux 一致，查询不重读来源；预算、取消、失败与物理回收测试通过。22 项普通 Rust 检查、10 项原媒体实际回归、3 种工程输出独立校验及相关编译/strict Clippy 通过。详见同一容器索引记录；该提交时 edit mapping、连续区间证明和正式产品导入/预览仍待实现。

### 容器呈现映射与显式解码时钟

提交 `3c0cf1a` 已补齐原始 PTS 排序、最终显示样本终点及精确 elst 映射，支持空白、重复素材、正分数速率、定格及边界两侧查询。固定 decoder 新增 raw ISO BMFF 模式和真实 track ID；共享索引 worker 惰性计算映射，宿主校验有界结果。解码 C ABI 升为 2，worker profile 升为 4；当前产品 SDK/worker pin 尚未替换。

四份实际视频共 306 帧的像素与 PTS、1,290 次精确时间查询和 42 次随机定位同独立工具/oracle 一致；两份为保持压缩 mdat 不变的受控元数据变体。新 release bundle 来自干净提交，7 个二进制共 8,424,776 bytes。原视频/AAC 解码、三种工程导出、相关 strict Clippy/编译与打包闭包回归通过，详见[呈现时间记录](video-presentation-time.md)。仍须把容器时钟接入通用素材采样、授权导入与实际播放；未将这些内核检查作为浏览器或完整产品验收。

### 索引驱动的真实工程采样

提交 `e639172` 已将容器映射与原始 decoder 接入 `IndexedVideoSampler`。每次返回精确映射及经过 PTS/轨道/时钟核对的画面；空白不解码，缓存按原始画面复用并受帧数/字节预算约束。候选访问点同时满足解码顺序与显示顺序，解决实际全边界拖动中暴露的 B 帧和开放 GOP 定位问题。时间校验下沉为 Native/同进程共用实现，worker 协议升为 profile 5；产品旧 SDK/worker 尚未升级。

五份素材的 426 个源画面、2,502 次取样与独立像素、精确映射和候选 oracle 一致，包含 436 次 seek 与 1,314 次缓存命中。新旧采样、解码/导出及取消回收回归通过，见[索引采样记录](video-indexed-source.md)。后续精确区间/空白合同和资源提供方已接入合成/导出，详见下段；产品播放/导出仍须连接，整体验收尚未完成。

### 索引画面共用预览与导出

`107ce8a` 将精确区间与 Before/After 侧提升到共享时间内核，合成资源明确区分视频帧、真实空白与静态图片；空白透出下层，不能掩盖缺失帧或授权失败。`IndexedVisualSource` 提供有界标准化图像 LRU，预览与导出共用，错误/取消同时释放索引和解码能力。

五组真实视频正放/倒放共 710 帧、10 个导出文件，Python 分数 oracle、预览、编码输入及独立 FFV1 解码一致；两方向共 24 个空白帧正确露出背景。254 项相关回归通过，6 项默认 ignored 不计入；另显式执行真实矩阵、Clippy、WASM 编译和 schema 检查。详见[合成与导出记录](video-indexed-composition.md)。

独立 SDK `.20261007.2` 已从干净 `76a1621` 构建 20 包，并通过只消费 archives 的真实工程编辑/定位/75 帧导出检查；SDK manifest `d0345e52d66a399ef47b541ef72e6f636acaffe41beace7a1e7a1fc3724d50bd`，同源 worker profile 5 manifest `4a4b194f09fdf2893b749dd5d3c4ba8ad54a1891a3648945341192522ab66fb3`。产品 worktree `dd5e1c7b6` 已升级并验证同一 pin，27 项安装与 7 项产品集成通过，原有 192 项工作区状态保留。这是计算接入进展，Browser、产品播放器和完整验收尚未完成。

### 精确时间

`mo-time` 从原 PPT 时间线抽取有界精确有理数运算。PPT 保留原合同与正除数约束的薄适配，视频可用反向映射所需的有符号除法。时间转换区分精确可表达与显式量化；不能表达时返回错误，量化返回舍入残差。支持四种明确舍入策略、负时间和分数帧率的绝对帧边界计算。

### 工程模型

`mo-video-model` 已有独立工程、序列、视频/音频/图形/字幕轨、片段、媒体/字体资源、流选择、嵌套序列、联动组，以及线性分段/倒放/定格时间映射。基础效果包括恒定变换、增益和显式相位范围的淡入淡出。剪开片段时平移效果相位，不重新开始淡入淡出。

校验覆盖身份与所有权、丢失引用、流/轨兼容性、半开区间重叠、源素材范围、时间映射、关联组、嵌套 DAG 深度和循环。JSON 拒绝未知字段及重复键，具有字节/节点/流/锚点/诊断/算术预算与取消检查。资源字节、URL、绝对路径和业务任务不进入模型。

顶层对象表使用有序持久树共享快照。10 万对象的计数测试证明：复制集合不复制记录，修改一个记录只复制该记录；这不是完整工程交互性能或内存峰值的验收。组件版本和许可见[固定来源记录](../../components/persistent-collections/component.json)。

### 原子编辑与逆操作

`mo-video-edit` 提供快照、`prepare` 和 `prepare_inverse`，实现标题、序列/轨道/资源管理、插入、删除、移动、分割、裁切、波纹插入/裁切、slip、roll、slide、基础效果/来源替换、关联/解除关联，以及覆盖、区间删除和显式音频策略的变速。波纹轨道集合、关联策略与分割后的新身份必须显式给出。区间/变速规则见[详细合同](video-editing-transactions.md)。

所有操作先在共享结构的候选中执行，完成后统一验证、计算摘要、读写集合、条件逆操作和预览失效范围。错误或取消不改变基线。逆操作按对象及关联依赖检查冲突，允许保留其他轨道的独立修改；新建关联、解除关联后成员被修改等情况不能被撤销静默覆盖。嵌套序列依赖和 solo 导致的其他轨道变化进入失效传播。

持久事务、幂等、版本号/epoch、outbox 和存储仍由产品实现。当前完整校验、摘要与失效传播是工程规模线性工作；不可把局部持久树更新的复杂度当成整个事务的性能。

### 工程编译与采样

`mo-video-compile` 已实现验证后编译、平衡区间索引、绝对帧/音频样本时钟、分组视觉计划和分层音频点查询。精确采样携带 before/after 侧；倒放与多级嵌套翻转该侧，不使用 epsilon，也不将中间有理数强制量化到作者时基。嵌套实例按视觉/音频轨道明确投影，透明度保留在分组，逐级音频保调/反向/定格策略不被扁平化。

VFR 呈现窗口按真实样本覆盖区间选择源帧，区分已知 gap 与尚未索引的 outsideWindow，验证内容摘要和流绑定。该纯计算索引只做元数据运算；demux/seek 由下述媒体组件承担；音频块处理的后续实现见下文，非原速保调的连续工程图见下文。接口、预算、边界及下一层责任见[编译实施记录](video-editing-projection.md)。

### 原生媒体叶子组件

已独立构建固定 FFmpeg 9.0.2 的 LGPL 动态库和原创受限 C ABI。源码 archive 摘要与官方发布密钥签名已核对；固定关闭 GPL/nonfree/version3、网络、协议和自动依赖探测。媒体输入只经宿主 read_at/取消回调，嵌入外部资源打开被拒绝，接口不接收路径或 URL。

已实际解码 H.264 B 帧/VFR 和 AAC，提供显式 stream selection、send/receive/drain、seek generation、原始平面复制、色彩/时间戳/声道元数据及预算/失败状态。没有默认系统 FFmpeg 依赖，没有把 C 原始帧转换成伪造的播放器输出。源码、许可、精确边界与复现见[媒体组件记录](../../components/ffmpeg/README.md)。

后续已实现 `mo-media-ffmpeg` 的 Rust RAII、回调 panic/错误边界、类型约束的单次流选择、线程归属与独立拥有的帧平面。构建时逐字段核对 C/Rust ABI，核实实际加载别名指向的固定组件；任何操作错误关闭 native context，取消/失败不返回部分帧。[执行边界记录](video-media-native.md)包含具体生命周期、panic 语义与复现。

可终止 worker 已完成：`mo-media` 分离无 codec 链接的数据，`mo-media-wire` 提供有界头与原始平面协议，`mo-native-media` 持有宿主资源和进程。总 deadline 不随读往返重置，错误/取消/崩溃会停止并回收进程；开发包校验固定摘要和 loader 闭包。详见[worker 实施及实际证据](video-media-worker.md)。连续区间的真实帧已接通工程采样计划；产品 GPU 显示、浏览器和编码导出仍未接入，不能把 worker 测试成功当成产品已经可用。

`mo-native-limits` 已提供后续 worker 启动使用的进程资源限制，实际验证 CPU 超限终止、地址增长拒绝、文件增长失败和继承限制。该模块已在视频 worker 接收素材前安装并读回核验；地址增长额度不等于 RSS 硬限额，也不是文件/网络沙箱。[平台边界与测试](native-worker-limits.md)明确保留这些区别。

### 真实源帧采样

`mo-video-source` 默认无 native 依赖，按精确源时间和 before/after 侧选择实际帧，使用有界缓存及 lookahead；可选 native adapter 复用既有 worker。真实素材暴露出 decoder duration 与相邻 PTS 重叠的问题，当前以宿主有证据的连续区间为前提，以相邻 PTS 定义呈现覆盖，保留原始元数据。seek/倒放不按平均帧号寻址，过冲先重读而不伪造 gap；取消/失败丢弃缓存并释放 decoder。[详细语义与局限](video-source-presentation.md)。

### 色彩与参考合成

`mo-color::rgb` 和 `mo-video-render` 已接通原始平面解释、明确显示 transfer、BT.2020/D65 线性预乘合成、SAR/显示矩阵、作者变换和组透明度。真实 worker 帧经过工程采样输出合成 PNG；CPU 原生/WASM 小向量包含 f32 bits 的完全一致验证。未知色彩解释、HDR 缺 tone-map、缺资源、预算及取消不会返回伪造成功。当前只提供参考过滤和简单显式 tone-map；文字与最终效果/导出仍待实现，GPU 后续进展见下一节。详见[色彩与合成记录](video-color-composition.md)。

### GPU 合成

`mo-video-gpu` 已通过共用 plan executor 接入真实 Metal GPU。CPU/GPU 使用一致的定点栅格坐标决策，修复浮点矩阵在最近邻边界选不同像素的问题；WGSL 执行同一分组/透明度语义。29 项硬件对照包含 1080p、最大宽度、HDR 线性值和四帧真实素材，并验证取消、预算、callback panic 后的恢复。当前返回未提交的 GPU 候选，仍需宿主显示、时钟和设备丢失处理；不代表实时工作台已可用。详见[GPU 实施记录](video-gpu-composition.md)。

### 音频块处理

`mo-video-audio` 已接通 PCM 归一化、命名声道/显式矩阵、有理采样时钟、带限重采样、倒放、增益/淡入淡出和嵌套混音。分块编译按片段和时间映射锚点调度，保持各层音高政策；窗口缺失不会伪装静音。真实 worker AAC 输出经五类实际 DSP，并与独立 CLI 和数学包络核对；Native/WASM 的完整 PCM bits 一致。工程保调图已接通；尚缺生产 Host 存储和声音设备；详见[音频块处理记录](video-audio-dsp.md)。

### 标准 PCM 与本地随机读取

`mo-video-pcm` 已实现连续解码 PCM 的内容身份、不可变分块候选、校验和、受限 LRU 窗口读取和 `CanonicalAudioResource`。真实 AAC 定位实验发现，通用 seek/flush 无法代表位精确的完整解码状态；本地派生块让定位/倒放和导出复用相同 PCM。两个真实素材的 264 次随机/反向读取以及完整倒放混音逐 bit 一致；Native/WASM 完整/部分物化 14 查询一致。生产本地/OPFS 存储与后台准备仍由产品 Host 接入，首次物化及冷定位性能尚未达成验收。详见[连续解码与分块读取](video-canonical-pcm.md)。

### 保调顺序计算

`StretchStream` 已实现固定 128 帧量子、整数输入时钟、两侧延迟补偿、有限源尾部和可重试缺源。已用固定 Signalsmith Stretch/Linear 接入 Native 与 WASM，12 组实际跨端 PCM 输出一致，并验证短片段、音高、重试、取消和分配失败。工程混音器已通过连续派生图调用该能力，并保留裁切/分割上下文；不随窗口查询重建 DSP。详见[保调实施记录](video-pitch-stretch.md)。

### 工程保调图和上下文

`AudioMixer` 已通过 `RetimeRequest`、`RetimeMaterializer` 与 `CanonicalRetimedAudio` 使用连续派生 PCM。分割和裁切保存 `audioContext`，拉长恢复隐藏速率锚点；同向速度段共享 DSP，反向/停帧分开。音频依赖摘要区分名称/视觉变化与上游声音变化，保留每层音高政策。真实 Native 集成和 Native/WASM 完整图比较见[工程保调记录](video-audio-retime.md)。生产存储/后台调度/设备播放仍待接入。

### 原生编码、封装与候选输出

`mo-media-ffmpeg::Encoder` 已实际导出 H.264 VideoToolbox/AAC MP4，另提供 FFV1/float PCM MP4 无损审计档案。精确帧/样本时钟、短尾、有限 packet 交错、短写和失败关闭已实现；硬件帧重排问题在固定上游组件中按 SPS 上界修正。12 组真实文件经独立工具核验，ASan/UBSan 和既有媒体 9 步回归通过。详见[编码实施记录](video-native-encoding.md)。工程合成到 I420 和有限导出会话见下一节；编码 worker 见下文，宿主校验发布和产品导出仍待连接。

### 工程合成到编码输出

`mo-video-export` 已将不可变工程、精确 CFR/音频样本时钟、CPU 合成、I420 输出和音频 mixer 接到编码接口。缺音频派生资源可以等待并从当前进度继续；错误和取消关闭候选。6 个真实 MP4、96 帧和 185,408 个 PCM 值的 Native/WASM 位级对照通过。详见[工程导出记录](video-project-export.md)。当前测试图形/程序 PCM 不代表真实素材全流程或产品验收；编码 worker 见下一节，宿主提交仍待接通。

### 编码进程与候选写回

`mo-native-media::NativeEncoder` 已实现 `VideoEncoder`，复用固定组件包、进程额度和既有 Session。私有协议升级 `/2`，有界 I420/PCM 输入及候选 WriteAt 写回、总 deadline、关闭阶段禁止写入、完成计数校验及 kill/reap 已实现。真实工程经受控进程导出并回读，故障路径与既有解码回归已验证；详见[编码 worker 记录](video-encoding-worker.md)。宿主持久化/校验发布和用户 UI 仍待接入。

### 合同与验证入口

`contracts/generated/video-*.schema.json` 从相同 Rust 类型生成，并生成对应的草案 TypeScript 类型，包含工程、编辑请求、逆操作、校验结果、视频帧/音频点计划/音频块计划、标准 PCM 块引用和呈现窗口。它们是开发中的合同，还未独立发行或接入真实产品。

`tools/mo-video-conformance` 是专用测试驱动。相同 Rust 代码编为 Native 和 WASM，比较完整候选文档、摘要、错误、读写集合、失效范围和逆操作；不是另写一套 TypeScript 语义。默认 CI 已加入长序列测试和视频跨端一致性步骤。

## 当前验证证据

| 检查 | 实际结果 | 不能推断的结论 |
| --- | --- | --- |
| 原 PPT 时间线 | 168 项既有测试通过共享算术适配 | 不等于新的媒体渲染已实现 |
| Rust 工作区回归 | 一次完整运行 1,675 passed、28 ignored；后续视频限定修正另跑专项 | ignored 中需要独立环境/worker 的检查不伪装通过；不是所有 CI 分组重跑 |
| 最新时间/模型/事务/编译专项 | 78 passed、1 个长序列测试默认 ignored 并再次单独执行通过；同次另跑 PPT 时间线 168 passed，总计 246 passed | 不代表所有最终编辑功能已实现 |
| 既有 MCP 纯计算入口 | 17 项测试通过，独立 Cargo.lock 已纳入共享 mo-time | 未增加视频 MCP 工具，legacy-host/HTTP 组合本次未全部重跑 |
| 精确时间生成测试 | 4,096 组有理算术性质；107,892 帧的 30000/1001 时间及 48k 音频累计边界 | 这是样本调度数学，未播放或编码真实一小时视频 |
| 事务长序列 | 固定种子 100,000 次；47,422 成功、52,578 拒绝、47,422 次逆操作精确恢复（已扩展区间与变速语料；本轮时间采样改动后重跑，计数保持一致） | 语料覆盖当前操作子集，不足以关闭完整 SEM-06 |
| 原子失败/取消 | 每个观察到的取消检查点拒绝候选，基线不变；中途失败、锁轨、缺素材、超预算验证通过 | 未检验数据库断电和业务 outbox |
| Native/WASM 实际运行 | macOS arm64 Native 与 Node v23.5.0 的 WASM，25 步：20 次成功、5 次拒绝，结果完全一致；重复键两端均拒绝 | 不等于浏览器媒体、WebGPU/WebCodecs、Windows/Linux 原生或产品验收 |
| 跨端 fixture 摘要 | `568babf4de9b90e46fca80bee7377ef437167f9ae1053254355c736fd008be5a` | 绑定本次基础语料，不是最终媒体档案 |
| 跨端结果摘要 | `3bc203a14d17be214aec45b5c9588f221296ab9b6229f28c8518adb5b126195a` | 同上 |
| 采样编译实际跨端运行 | macOS arm64 Native 与 Node v23.5.0 WASM；15 查询、11 成功/4 拒绝，完整输出与统计一致；分组/字体绑定/图像/音频策略/VFR 结果另有预期断言 | 使用合成元数据，不能代表真实解码、GPU、声音质量和产品环境 |
| 编译 fixture / 结果摘要 | `2020028ae82f19c49e65c5e35958f8794a3393a7be7b216c11d1e4a480b78360` / `5b691d973f12abe93476db5250fa24033e979b0b2c2960ca3648ab0e853bb986` | 对应上述 15 查询语料 |
| 区间索引 | 2,048 组随机查询与独立扫描一致；10 万稀疏区间的四处查询均少于 64 节点访问 | 不是全工程编译、播放帧率或 UI 延迟基准 |
| 原生真实媒体 | macOS arm64 固定 FFmpeg 9.0.2；77 帧 B-frame/VFR 的时间/像素 MD5 与独立 CLI 解码一致；前后 seek、13 字节短读、EOF、取消与预算通过 | 仅这一实际素材/平台，不是浏览器、硬件、多 codec/多平台矩阵 |
| 原生 AAC | 已知 3 秒/48k 源返回 144,000 样本且时间连续；与参考 PCM 最大差 `2.24e-8`、RMS `3.32e-9` | 已明确处理参考旧版 CLI 多出的尾 padding；不是完整混音/保调/音质测试 |
| Rust 原生所有权 | 8 项行为测试与 3 项 compile-fail 通过；真实 WAV 逐字节往返、资源 drop、C 内回调 panic、逐检查点取消、独立拥有帧和预算 | 包装不能抢占阻塞回调或限制全部 vendor 私有分配；worker 证据单独记录 |
| Rust 真实媒体 | 显式执行 1 项真实素材测试并独立核验：77 帧像素 SHA-256/PTS、前后 seek 及 BT.709 标签一致；144,000 AAC 样本与参考误差同上 | 不是 renderer/浏览器/产品执行，原生 adapter 只在本机 macOS arm64 验证 |
| 进程限制基础 | 6 项父测试实际启动隔离子进程，验证地址增长拒绝、内核 CPU 终止、继承限制、非法参数、文件描述符与文件增长受限 | 已与视频 worker 连接；不是 RSS、系统调用沙箱或完整产品隔离验收 |
| 原生媒体 worker | 默认协议/宿主 8 项，真实 worker 6 项、包完整性 3 项、真实压缩媒体 1 项均通过；77 帧/144,000 音频样本与参考一致，1.5 MiB 大媒体包经短读成功；宿主无 codec 链接 | macOS arm64 开发包；无完整系统调用沙箱、浏览器或用户时间线验收 |
| 真实工程源采样 | 10 项时间/行为测试；90 正放 + 90 倒放 + 8 随机定位均与独立像素/PTS/覆盖区间一致，实际使用 Rust 工程和 worker；缓存最高 221,184 active bytes | 仅已知连续区间语料，无通用 edit list 解析、GPU、音频 DSP、浏览器或产品播放 |
| 色彩与参考合成 | 19 项单测（含后续定点坐标检查）及显式真实用例通过；77 帧的 2,838,528 个线性通道值与独立 zscale 最大误差 `4.768e-7`、RMS `6.967e-8`；真实计划输出四帧及旋转/透明度合成 | CPU 参考档案，不是高质量缩放、GPU、全 HDR 或成片导出 |
| 像素跨端一致 | Native/WASM 9 查询、7 渲染/2 拒绝，f32 bits、RGBA8 和诊断一致；编辑 25 步/编译 15 查询回归摘要不变 | 未执行浏览器 WebCodecs、WebGPU 或实际播放器 |
| 实际 GPU 合成 | Apple M4 Max/Metal，29 用例最大线性误差 `7.153e-7`，RGBA8 最大差 1；26 取消点、三类预算、callback panic 恢复通过 | 未验收 Windows/Linux/浏览器、设备丢失恢复、实时 FPS、零拷贝或产品显示 |
| 音频块处理 | 17 项音频基础测试、18 项编译测试；真实 AAC 五个完整输出；Native/WASM 8 查询的 PCM bits/计划/统计一致，任意分块与大块切片一致 | 生产 PCM Host、设备播放与浏览器未完成；单一素材不等于完整音质/性能验收 |
| 标准 PCM 随机读取 | 10 项默认测试；两段真实 AAC 各 144,000 样本/36 块；共 264 查询与两个完整倒放输出逐 bit 一致；Native/WASM 14 查询一致 | 测试文件 store 不是生产 Host；冷定位/长文件准备、OPFS、设备播放和完整产品未验收 |
| 保调计算 | 7 项原生行为；12 组实际 Native/WASM 全 PCM 输出一致；35 个分配故障点与 ASan/UBSan 通过 | 工程变速图与派生 PCM 另有集成测试；浏览器设备和产品播放器未完成 |
| 工程保调与派生 PCM | 13 项真实 Native 集成；5 个派生任务、124,555 个跨端比较样本逐 bit 一致；分割/拉长/嵌套政策/失效/候选重试与有界缓存通过 | 测试内存 Host；生产持久化、浏览器设备、声音试听与工作台未验收 |
| 原生编码候选 | 12 组实际文件/258 视频帧；短尾/分数帧率/时钟/独立解码通过，无损像素与 PCM 相等；两档案 ASan/UBSan、失败/预算/取消通过 | 编码 leaf adapter；未接工程渲染、worker 导出、浏览器或产品交付，AAC 质量与复杂内容单独验收 |
| 媒体内存检查 | wrapper 与 FFmpeg C 实现分别构建 ASan/UBSan 档案，同一真实检查通过 | 汇编与平台框架不因此被插桩；不是 hostile-media 隔离保证 |
| 原生媒体库体积 | 本机 release 配置 5 个 FFmpeg 库及 C wrapper 共 7,392,248 bytes | 不包含 Rust worker、字体、GPU、额外 codec 或系统框架，不是最终应用体积 |
| 代码与合同 | 整个 Rust 工作区 strict Clippy、WASM 编译、全部 schema/TypeScript 合同检查通过；新增视频 schema 使用相同生成/检查入口 | 未发布新版 SDK、组件频道或 UI |

长序列复现：

```sh
cargo test --release --offline --locked -p mo-video-edit --test generated_sequences -- --ignored --nocapture
```

跨端复现使用 `tools/verification/current.py --groups wasm`（包含原有 PPT wasm 门禁），需按既有构建文档提供固定 native 组件与匹配的 wasm-bindgen。视频测试脚本为 `tools/verification/video-domain-parity.mjs`；它要求本次构建的 Native 程序和 Node wasm-bindgen 输出，不接受用一份预录结果冒充其中一端。

## 明确尚未完成

- 当前已支持锚点线性变速；完整曲线变速/反查、转场、关键帧、字幕词锚点、分析与互操作等最终语义仍待补全。音频策略已保存，实际重采样/混音基础已实现，非原速保调及工程派生图已实现，生产 Host 接入和完整音频处理仍待完成。
- 原生已认证工程播放/导出与正式文件内部链、Browser 正常页面本地媒体/预览/持久导出已接通。全部会话消息交付/后台状态与未读、浏览器 Agent 执行、物理音画同步/长时播放、原生 GPU 显示/纹理互操作及跨端矩阵仍未完整验收。
- 已有独立开发 SDK、Browser Host 基础、Device metadata 工程执行、Agent 技能/桌面 RPC/IPC、基础时间线、待确认命令恢复及固定媒体 runtime/worker 和授权 probe；生产 SDK/MCP/CLI、完整媒体 Host、属性 UI、完整应用恢复验收和项目包仍待接入。
- CE.SDK 页面与依赖已从产品 worktree 退役，旧源码和浏览器数据保留；旧 scene 数据仍须审计、迁移并保留原件，不能仅换 engineId。
- 完整功能、媒体质量、性能、体积、多平台与真实 provider 计费没有完成验收；当前不能交付为“用户已可验收全部视频功能”。

## 本地材料

发布核验材料：忽略目录 `.codex-work/stable-release-20261006/`。产品旧实现快照：其 worktree 的 `.codex-work/video-native-migration-20261006/`。

测试记录：`.codex-work/video-foundation-workspace-tests-02.log`、`video-domain-tests-final.log`、`video-edit-generated-100k-02.log`、`video-domain-conformance.json`、`video-foundation-clippy-workspace.log`、`video-final-component-clippy.log`、`video-schema-check.log`。区间编辑增量记录：`video-range-tests-final.log`、`video-range-generated-100k.log`、`video-range-conformance.json`、`video-range-clippy-final.log`、`video-range-schema-check.log`。这些本地记录不进入发行依赖。

编译与采样增量记录：`video-projection-tests-final.log`、`video-projection-conformance.json`、`video-projection-edit-regression.json`、`video-projection-workspace-clippy.log`、`video-projection-generated-100k.log`、`video-projection-schema-check-final.log`、`video-projection-repository-check.log`、`video-projection-types-check.log`。对应复现脚本为 `tools/verification/video-projection-parity.mjs`。

真实媒体增量记录：`.codex-work/media-native-checked/report.json`、`media-native-sanitized-current/report.json` 和 `media-native-gate-current/report.json`。素材 SHA-256 为 `ca7211a9b5e1bbdb66c7bd98e515484d7b6bc5ee8bbeeadb6cabb06d90bd73a6`。检查脚本 `tools/verification/media-native-check.py` 已加入当前 CI 的 media 分组；没有宣称该提交的远端 Linux CI 已经运行通过。

Rust 接入增量记录：`.codex-work/media-rust-gate-final/report.json`、其 `media-rust/report.json` 和 `.codex-work/media-rust-workspace-clippy.log`。media 分组现在顺序执行 C/Rust 真实媒体门禁，固定组件缺失仍报告失败。

进程额度增量记录：`.codex-work/media-process-limits-evidence-final.log`。父测试自动拉起专用 child fixture，完成后等待回收；该 fixture 的 ignored 标记不表示六项真实子进程检查被跳过。

跨进程媒体增量记录：`.codex-work/media-worker-gate-final/report.json`、其 `media-worker/report.json`、`.codex-work/media-worker-workspace-clippy-final.log`。media 分组顺序执行 C、Rust、构建 bundle 和实际 worker 四步；详细协议、额度与发行边界见 [worker 记录](video-media-worker.md)。

工程源采样增量记录：`.codex-work/video-source-gate-final/report.json`、其 `video-source/report.json`、`video-source-workspace-clippy-final.log`、`video-source-wasm-check.log`。media 分组已增加第五步实际工程采样检查；WASM 此处只验证编译，不宣称浏览器解码通过。

色彩与参考合成增量记录：`.codex-work/video-render-gate-final/report.json`、其 `video-render/report.json`、`video-render-parity.json`、`video-render-domain-regression.json`、`video-render-projection-regression.json`、`video-render-workspace-clippy.log`。media 分组已增加第六步真实色彩/合成，WASM 分组增加独立像素向量；边界与具体证据见[实施记录](video-color-composition.md)。

GPU 增量记录：`.codex-work/video-gpu-gate-final/report.json`、其 `hardware.json`、`video-gpu-color-regression/report.json`、`video-gpu-{render,domain,projection}-parity.json`、`video-gpu-workspace-clippy-final.log`。独立硬件门禁为 `tools/verification/video-gpu-check.py`；通用 CI 的 ignored 不视为硬件通过。

保调增量记录：`.codex-work/stretch-gate-final/report.json`、其 `video-stretch-parity.json`、`.codex-work/stretch-asan/build.json`、`stretch-workspace-clippy.log`。新增 `stretch` 验证分组，使用实际原生程序、Rust WASM 和单独 DSP WASM，不接受录制输出。

工程保调增量：`.codex-work/audio-retime-gate-final/report.json`、`video-stretch-parity.json` 的 `retime` 字段，以及 `.codex-work/audio-retime-all-tests-final.log`。测试使用真实 DSP 和内存持久候选，不代表生产工作台完成。
