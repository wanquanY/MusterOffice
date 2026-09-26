# 原生旋转时间图与确定性求值

状态：一期完整播放内核中的第一条动画语义链路。继续原 Rust/TS/精选 C++ 方案。本阶段涉及创建、查询、事务编辑、原生 PPTX 写入/读取及指定时间求值；实际播放合成、完整动画、转场和媒体仍在实施范围内。当前不能据此替换 Musterwork。

## 职责与数据

`mo-timeline` 只拥有时间图验证和属性求值，没有系统时钟、事件监听、文件、网络或宿主状态。`TimelinePlan` 是可复用的不可变计算计划；宿主提供版本、播放会话、generation、采样时间和完整事件前缀。当前 JSON 开发入口逐请求验证/编译，不是已完成的保留式实时播放会话。

作者文档通过 `Document.timelines[slideId]` 存储时间图；空映射不进入序列化，保持旧文稿的语义摘要。每个行为具有稳定 ID、正有理时长、开始条件、千分之一精度重复次数、remove/freeze 和旋转目标。旋转使用绝对局部角度，保留负角和多圈角度。时间计算沿用 ticks 字符串与 timescale，不使用浮点时间或固定帧率采样表。

当前开始条件包括页面零时刻偏移、另一个行为的开始/结束加延迟、指定对象或页面的首次点击加延迟。行为仅激活一次，对应 `restart=never`；重复在行为内部表达。每个行为至多一个开始依赖，依赖图必须无环；当前实现不代表容器时间树、多条件组合、restart、自动反向或所有 Office 效果已实现。

求值覆盖等待、已计划、活动、冻结和结束状态。重复中间的整数边界进入下一轮起点；最终整数边界冻结在上一轮终点；分数重复在对应分数进度结束。替换式旋转按最近激活者优先，同一激活时间由作者顺序确定。此组合规则仍需目标应用观察，不以自读写一致证明 Office/WPS 一致。

计算结果以约分后的分子/分母字符串输出。编译默认最多 10,000 个行为、事件前缀最多 65,536 项、精确运算中间数最多 1,024 位。超出预算或取消时不返回部分帧；不会为避免失败而改用不精确结果。工作复杂度由一次图遍历、事件索引和有界大整数运算控制，不做节点与全部事件的笛卡尔积扫描。

## 宿主、版本与 seek

`PlaybackBinding` 包含 session、revision 和独立 uint64 generation 类型。generation 使用规范十进制字符串且不回绕；它不是资源字节数或权限令牌。宿主仍负责版本来源、CAS、会话权限和提交。

交互图必须提供覆盖到采样时间的完整事件前缀，包括明确声明“没有点击”的时间区间。事件从序号 1 连续排列，时间不得倒退；相邻完全相同的重复传输可去重，冲突或过期 generation 拒绝。历史覆盖范围不足时返回 `EVENT_HISTORY_REQUIRED`。回退 seek 不读取采样时间之后的点击，不凭空生成事件；同一输入得到同一状态和摘要。

`mo-kernel-api` 的 `evaluate-timeline` / WASM `evaluate_timeline` 接受已有快照，验证语义摘要及播放版本后调用同一实现。当前目标为属性状态，尚不输出动画像素、音频或视频。它不修改作者文档，也不把临时播放状态写回文件。

## 事务与原生映射

`SetTimeline` 按幻灯片原子替换或删除时间图，事务回执报告 `changedTimelines`。目标与点击触发对象必须属于该页，可在组合内；不能引用其他页、母版或布局对象。删除目标时，拒绝策略报告引用冲突；级联策略删除直接引用该对象的行为及其开始依赖闭包，不修改剩余行为的触发含义。整笔失败保留旧快照。

PPTX 使用标准 `p:timing`、`p:cTn`、`p:animRot`、`p:cBhvr` 和原生目标 ID；时间条件、重复、fill 与旋转属性均为可编辑的原生声明，没有栅格动画或私有 HTML。当前 OOXML 映射要求所有源时间可精确表示为 unsigned 毫秒，非整毫秒或超出范围明确诊断，不舍入导出。

`pptx-timing` / WASM `inspect_pptx_timing` 绑定完整包摘要、幻灯片部件与部件摘要，返回原生时间 ID 和形状 ID 映射。它不依赖本项目作者标记。当前严格读取已实现的结构；遇到额外属性、嵌套容器、其他行为、多个条件、未知目标或兼容分支中的时间树时拒绝投影。原始 OPC 内容保持不变，不将复杂结构扁平化成已实现子集。

来源文本编辑已有的 timing 重定向保护继续保留。读取投影不等于已经实现对任意导入动画的保留式编辑；这一点与作者文档事务明确分开。

## 验证与下一步

验证工具使用 `timeline-` 前缀，产物位于 `.codex-work/timeline`。Rust/WASM 共用实现；Python `Fraction` 从实际公共接口输入重新计算各状态、精确进度和属性值，独立 DOM/XSD 核对实际导出包的原生声明。既有静态接口和像素必须与上一阶段冻结结果一致。

本阶段通过 655 项 Rust 测试、87 份同源 Schema/TypeScript 检查及严格 Clippy。246 次新链路 Native/WASM 调用一致；独立参考核对 193 个采样状态、2309 个节点状态和 399 个旋转属性。2 份原生导出包的 10 个行为经独立 XML 核对，12 个 PML 部件通过 XSD。355 次已有静态绘制调用的元数据与像素保持不变，旧作者快照和静态 PPTX 字节保持不变。原生 CLI 拒绝覆盖和精度失败不发布文件均已核对。

Rust WASM 原始模块为 7,095,019 字节，较上一阶段增加 398,069 字节；C++ 绘制 WASM 保持 2,370,691 字节。本阶段没有完成实时吞吐、启动、内存或完整安装包测量，不能据此宣称播放性能或产品体积已验收。完整绑定见[阶段证据](../reviews/evidence/2026-09-26-timeline-verification.json)。

下一步将时间属性状态接入既有对象放置和场景合成，保持源文档不可变；特别处理分数旋转、组合继承、缓存失效、误差界和回退 seek。随后扩展效果/时间容器、转场、媒体及完整宿主会话，完成 Office/WPS 编辑与播放观察和 Musterwork 产品链路。禁止以当前有限行为子集、XSD 或自读写通过替代这些验收。

规范依据：Microsoft 的[动画结构说明](https://learn.microsoft.com/en-us/office/open-xml/presentation/working-with-animation)、[AnimateRotation](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.presentation.animaterotation?view=openxml-3.0.1)、[CommonTimeNode](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.presentation.commontimenode?view=openxml-3.0.1)，及本地固定 ECMA-376 PML XSD。规范说明不是实测应用兼容结论。
