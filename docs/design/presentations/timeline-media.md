# 时间、事件、媒体与实时呈现

v0.4 设计提案 · 2026-09-24。依赖[文档模型](document-model.md)，宿主与资源边界见[执行规格](runtime-performance.md)，动态实现路线见[算法规格](../implementation/algorithms-and-components.md)。

## 1. 领域边界

动画和媒体不是静态渲染器旁边的可选脚本。核心定义开始条件、先后/并行、目标属性、插值、结束/保持、重播、跳转和媒体时间关系。宿主负责实际时间采样、输入、授权、解码、音频输出和显示；二者通过显式事件与意图连接。

OOXML 有页面时间信息、行为节点及转场结构，也可以针对对象和文字范围。Reader/Writer 必须保留这些语义；静态图片不包含它们。[Microsoft 动画结构说明](https://learn.microsoft.com/en-us/office/open-xml/presentation/working-with-animation)

## 2. 时间图与引用

| 模型 | 主要字段与约束 |
| --- | --- |
| TimingNode | 稳定 ID、seq/par 容器、开始/结束条件、持续时间、重复、restart/fill |
| Behavior | 对象/文本/系列/Diagram 子目标、属性、关键帧、插值、加速减速、路径/变换空间 |
| Trigger | 页面进入、点击序号、对象动作、节点开始/结束、媒体书签等类型化条件 |
| Transition | 页面间效果、时长、方向/参数、推进条件、两页状态及可选匹配映射 |
| MediaSpec | 资源、封面、片段、音量、静音、循环、书签、跨页行为和播放触发 |
| Action | 下一页、指定页、自定义放映、外部链接等受限动作 |

时间使用有理时间值和明确 timebase 表达，保留源格式精度；采样 API 可用整数微秒，但只在边界量化，不能逐帧累计舍入。`indefinite`、事件驱动和媒体实际时长分别建模，不能都用 0 或负数代替。媒体时间戳与演示时间转换保留映射和舍入规则。

编译时校验引用、先后依赖、持续时间和零时间循环。文档允许的重复效果按节点规则执行，不被简单“所有环一律禁止”误删；同一求值步的无限事件循环必须被检测并有明确错误。动态表达式只接受受限类型化格式表达式，不执行 JavaScript。

## 3. 播放状态机

```text
Created → Preparing → Ready → Playing ⇄ Paused
                                  ↓
                    AwaitingInput / Buffering
                                  ↓
                              Playing
Playing / Paused / AwaitingInput → Ended
any nonterminal → Failed / Stopped → Disposed
```

状态名称为提案；需要附带原因和恢复条件。例如权限要求用户手势属于 AwaitingInput，媒体暂未解码属于 Buffering，资源被撤销或格式不能解码属于可诊断失败。不能统一映射为“加载中”并无限等待。

PlaybackSession 绑定 `documentRevision + planId + resourceManifest + generation`。文档被修改时原会话继续基于原快照或由调用方停止并重建，不能在一帧内混用新旧对象。导航或 seek 提升 generation，旧解码结果和迟到事件被丢弃并释放。

核心入口概念为：

```text
evaluate(plan, checkpoint, orderedEvents, targetPresentationTime)
  → FrameState + MediaIntents + nextCheckpoint + Diagnostics
```

FrameState 包含页面/对象可见性、已求值属性、裁剪/效果及合成层；MediaIntents 包含 prepare/play/pause/seek/stop、目标媒体时间、音量和同步约束，不持有操作系统设备句柄。宿主报告实际媒体状态，不能擅自改写节点持续时间。

## 4. 事件和 seek 的精确定义

事件记录包括 session、generation、单调序号、演示时间、类型、目标与载荷。宿主将同一采样区间收到的事件排序后提交；核心拒绝序号倒退或错误 generation，重复事件去重。墙钟不进入文档与语义摘要。

点击触发具有状态依赖：相同时间点上点击一次和两次可能不同。故 seek 要求提供事件日志或可验证 checkpoint + 日志后缀。没有日志的 `seek(time)` 只允许定义清楚的自动播放轨迹；需要交互而事件缺失时返回缺少上下文，不能虚构点击。

向前/向后跳页、上一页返回、重放当前页、自定义放映切换分别定义进入状态、点击游标和跨页音频策略。离开页面时停止不允许跨页的媒体、释放页面特有帧，并更新仍在继续的音频。隐藏页不影响手动指定跳转的合法性判断规则。

测试例：标题淡入由第一次点击启动，视频在标题结束后播放，媒体书签触发图表系列强调。暂停期间不推进演示时钟；恢复后保持书签只触发一次。向前 seek 必须恢复视频目标时间和系列状态；seek 返回之前触发点后重放的行为由事件日志和重启策略确定。

## 5. 时间采样与音画同步

无音频播放时宿主提供单调时钟；有音频时建议以音频设备实际播放时间建立映射。核心只接收演示时间和媒体观测值，不读取系统时钟。暂停、后台挂起和速度变化通过显式时间映射改变，不能靠帧数估算时间。

媒体观测含 decoded/ready/playing/buffering/ended、实际位置和错误。若视频迟到，宿主只能按预先声明的同步策略丢弃过期显示帧或暂停时间推进，不得跳过关键时间事件。声音策略需避免重复播音；离线采样走固定事件轨迹而非设备实时状态。

帧队列按时间和内存双重有界。每个解码帧具有租约，合成完成、丢弃、导航和取消都会释放；解码器的内部缓冲也必须可统计或声明上界。资源不足时回压到解码/预取，不继续解完整视频。

## 6. 媒体宿主能力合同

```text
describeCapabilities → container/codec/profile/level/pixel/audio limits
probe(resourceHandle) → tracks, duration, timebases, seek index, metadata
prepare(spec, budget) → bounded session handle
apply(mediaIntent) → acknowledged state / required user action
acquireFrame(targetTime, generation) → leased frame / not-ready
release(frame), close(session)
```

资源句柄由宿主授权，只读、有摘要、长度和范围读取能力；外链是否下载、是否允许联网由宿主决定。媒体探测、容器索引与解码器可位于沙箱适配模块，核心仍拥有剪裁、循环、书签和同步语义。不可读取的链接不能伪装成可离线交付。

WebCodecs 可作为浏览器适配候选，但规范不要求实现任何指定 codec，也不自动解决容器拆包。因此不能写“支持 WebCodecs 即所有浏览器支持全部 PPT 媒体”。必须按配置探测，并把 demux、解码、音频设备、许可及离线分发作为完整闭包评估。[W3C WebCodecs](https://www.w3.org/TR/webcodecs/)

建议实验优先覆盖目标应用常见的 MP4 视频与独立音频组合，固定容器、codec、profile、像素格式、音频采样和设备；具体格式承诺仍待矩阵批准。无能力的浏览器不能静默转成海报帧并报成功；产品可以按已授权策略选择受管宿主，但需要相同功能和可见任务状态。

转码只能是显式资源派生操作：记录原摘要、编码设置、损失和派生摘要，验证目标应用播放，不覆盖原资源。自动播放权限阻挡不应触发转码；两者属于不同问题。

## 7. 实时合成及标准帧

CompiledPlan 将静态图层、动画图层、媒体表面、效果依赖和跨页合成分开。每帧只求值有变化的属性和受影响绘制片段，复用有效几何/字形和纹理。复杂动画改变几何时遵循对象模块规则，不能由宿主替代布局。

CPU 标准后端负责固定状态的可复现渲染；实时合成后端负责按时呈现，候选包含 GPU。缺少 GPU 时是否满足支持档案由工作负载验证决定，不能假设慢速逐帧 PNG 仍满足播放要求。

转场要定义离开页和进入页的取样状态、动画/媒体是否继续、过渡进度与遮罩顺序。Morph 已纳入一期设计，必须拥有对象匹配、文本/形状插值和原生格式映射；通用淡入淡出不能冒充等价。目标应用交集与具体效果按[能力矩阵](../implementation/capability-matrix.md)验证。

截图 API 必须携带 `viewMode + eventCursor + time + frameStateDigest`。预览缩略图、编辑态和实际放映初始态可不同，QA 不能将这些图无条件相互比较。

## 8. 原生导出、错误与验收

Writer 输出原生时间树、行为引用、媒体关系及目标应用支持的转场。不能把一段动画烘焙成视频后声称原动画可编辑。输入中已保留的媒体裁剪、淡入淡出和书签按对应语义读写。[Microsoft 媒体扩展](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-pptx/f8b7e1cb-976e-4f38-8139-f9e5ffa826e8)

错误应定位到时间节点、目标对象、事件或媒体资源，区分坏文件、语义冲突、宿主能力、权限与暂时缓冲。暂停和等待用户输入不是导出成功，也不是失败；宿主向用户展示真实状态。

验收至少覆盖顺序/并行/点击、文字范围和图表分步、路径和重复、跨页媒体、seek/暂停/重播、旧 generation 帧丢弃、音画同步与队列释放。对照目标应用的事件轨迹、采样帧及再编辑后的行为。单张截图与单次自动播放不能替代这些断言。
