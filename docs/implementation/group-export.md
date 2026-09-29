# 组合无描边与导出后播放一致性

2026-09-27。继续[原生缩放动画](scale-animation.md)暴露的实际失败输入；完整一期及 Musterwork 替换验收保持开放。

## 组合声明的映射

作者模型的 `Appearance.stroke` 描述该对象，不是对所有后代执行样式修改。PPTX `CT_GroupShapeProperties` 没有 `ln` 或自身几何，因此组合显式 `Stroke::None` 对应没有组合线条声明。原先把所有非 Inherit 值一并拒绝，导致已经能预览的作者组合不能导出。

规则现在归属作者到原生声明的投影层：组的 Inherit/None 不生成线条，普通形状的 None 仍生成 `a:ln/a:noFill`。子对象的线宽、颜色、端点、连接和继承声明保持原值；作者声明、语义摘要和版本状态不被改写。写出层继续消费该唯一投影，不增加导出专用补救逻辑。相同组合的 Inherit/None 具有不同作者身份，但得到相同原生文件字节。

依据：[微软 GroupShapeProperties 的子元素定义](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.presentation.groupshapeproperties?view=openxml-3.0.1)及固定 ECMA-376 XSD。可见组合描边仍需要明确的原生语义映射，继续返回诊断；没有将它偷偷改成后代批量样式或额外矩形。

## 实际画面检查发现的几何差异

原始失败文稿修复后可导出。扩大到带非均匀缩放、旋转和翻转的嵌套组合时，分数时间帧出现作者预览与原生回读的画面差异。定位为两条路径使用不同的曲线细分预算：该帧作者椭圆为 16 段，原生为 32 段，并非动画数值或组合矩阵丢失。

局部 Q32 EMU 预算和曲线准入现在由 `coordinate_budget` 共享。保留原生路径已有的向下取整、矩阵不确定性和保守页面范数：路径预算占设备误差容限的四分之一，其中曲线插值再占四分之一；数值转换与下游绘制保留独立误差检查。作者路径采用相同规则，不放宽容限。渐变需要更细边界时的原生预算保持原值。

闭合椭圆的原生预设与作者路径从不同象限开始。回归测试只循环移动完整闭合轮廓的起点，再比较全部精确控制点、变换、画笔和拓扑；没有对坐标舍入或删除线段。实际 Worker 另行检查完整 RGBA 字节，不以几何测试代替像素结果。

## 验证与边界

- 母版、版式、页面、嵌套组合的声明图回读一致；可见子对象描边、显式无描边和继承均保持，原输入不变。
- 四个拥有明确来源的作者输入：原始失败文稿、子对象描边、嵌套组合、两者结合。每份采样 8 次，覆盖起点、分数、中间、结束、结束后、回退和重复采样；32 对导出前后画面完全一致。
- 独立 Rust SDK 使用上一阶段固定 consumer 与本阶段 worker，验证稳定的播放接口；本阶段没有重发 Rust SDK 源码包。
- 最终 18 步门禁通过，含 961 项 Rust 测试、119 份合同及实际原生组件、播放、导出和文档 Native/WASM 一致性；常规测试另有 25 项默认忽略，显式组件测试按证据单列。TS/MCP 专项组未重跑，相关源码和合同未变。
- 新语料 64 帧分别通过独立 Rust SDK、打包 Node WASM SDK 和隔离 Chrome 153；对应 8 个播放实例。旧来源另有原生 37 帧、Node 55 帧保持原像素和元数据，后者额外含上一阶段 18 个缩放帧。仅统计实际 rendered 的回放案例。
- 四份 PPTX 保留原生组、形状、旋转及缩放节点；16 个 PML 部件通过固定 XSD。没有把页面或动画转成图片。
- 四份 PPTX 的 WASM 与原生输出字节一致。开发用 WASM 播放包仍为 60 个文件，未压缩 14,499,491 字节（不含 manifest），压缩归档 4,167,815 字节；较上一阶段分别增加 4,285 和 1,635 字节。没有新依赖、组件或 TS 客户端改动；不是 Musterwork 桌面安装包体积。

最终门禁、Native/WASM/浏览器结果、包体和局部性能记录见[本阶段证据](../reviews/evidence/2026-09-27-group-export-verification.json)。历史失败证据保留：先暴露画面差异；提取公共预算时遗漏渐变子模块的显式导入；一次诊断错误复用了失败构建前的二进制；新测试最初把闭合轮廓起点不同当成几何不同，随后按完整轮廓语义校验。

某些作者曲线现在需要更多细分以匹配原生精度，因此不能宣称性能无代价。上述范围不证明任意作者文稿导入后像素相同，也不证明全部几何、可见组合描边、完整动画、高级内容或 Office/WPS 实际行为已完成。后续继续原生动画语法、完整对象语义与真实应用校准；未修改 Musterwork 或启动替换。

## 局部性能缺口

同一 320×240 嵌套作者帧，固定新旧播放包各准备一个真实 Node Worker，各预热 10 帧后交替测量 100 次同步采样。每次按该版本对应的原生结果检查完整像素和元数据，检查在计时区间之外；时间包括 IPC 和结果传输，不包括初始化。系统缓存、其他系统负载和 GC 未控制，设备与全部输入摘要记录在证据中。

旧路径 16 段，中位 1.664 ms、P95 1.880 ms；新路径 32 段，中位 2.212 ms、P95 2.433 ms，中位增加约 0.548 ms（约 33%）。这是一个明确的局部回退，不能外推为整体性能结论，也不能用几何误差变小替代性能验收。下一步检查播放计划复用局部几何、三角计算与已有精度结果的机会，保持此次精度和导出一致性；内核只持有有界的计算状态，不增加宿主业务存储。

## 复现语料

构建匹配的原生 CLI/Worker 后，可从公开仓库中的自有语料生成导出请求。指定新输出目录，避免覆盖以前的诊断：

```sh
python3 - <<'PY'
from pathlib import Path
import copy, json
s = Path('.codex-work/group-export-example')
s.mkdir()
page = json.loads(Path('fixtures/presentations/playback/page.json').read_text())
doc = page['page']['document']
nodes = doc['timelines']['slide:1']['nodes']
n = copy.deepcopy(nodes[0])
n['id'] = 'scale:group'
n['effect'] = {'kind': 'scale', 'target': 'group:1',
               'from': {'x': 100000, 'y': 100000}, 'to': {'x': 200000, 'y': 50000}}
nodes.append(n)
defaults = json.loads(Path('fixtures/presentations/native-export/request.json').read_text())['defaults']
(s / 'request.json').write_text(json.dumps({'document': doc, 'defaults': defaults, 'resourceBindings': []}))
PY
python3 tools/verification/group-export-playback.py \
  --output .codex-work/group-export-example/frames \
  --worker target/debug/mo-raster-worker --cli target/debug/mo-cli \
  --request .codex-work/group-export-example/request.json \
  --page fixtures/presentations/playback/page.json
```

输出的 author/source 清单可继续交给已有 `sdk-playback-reference.py`、`wasm-playback-reference.mjs` 和 `browser-playback.py`，分别验证接入通道。所有资源均由调用者提供，不引入内核 UI、权限或持久存储。
