# 字体验证语料

`owned-tracking.ttf` 在原创装饰字体之上添加可选与必需 GSUB 连字，用于字符间距测试；由 `tools/verification/tracking-font-fixtures.py` 生成，清单为 `tracking-manifest.json`。仍是原创合成测试轮廓，不属于运行时字体包，许可状态与下述原创资源一致。详见[字符间距](../../docs/implementation/character-spacing.md)。

`owned-decorations.ttf` 由 `tools/verification/decoration-font-fixtures.py` 从原创 MVAR/avar 字体生成，设置单下划线/删除线度量，并将两个 Hebrew 码点映射到原创测试轮廓验证双向坐标。它不是语言字形质量样本；清单见 `decoration-manifest.json`，实现与证据见[文字装饰](../../docs/implementation/text-decorations.md)。许可状态与下述原创字体一致。

`owned.ttf`、`owned.otf`、`owned.ttc` 是本仓库原创的微型合成字体；三角形轮廓、名称与映射由 `tools/verification/owned-font-fixtures.py` 写出，不包含第三方字体轮廓或产品素材。使用 FontTools 4.61.1 可重建，摘要见 `owned.json`。它们只验证元数据、cmap 和异常处理，不用于展示，不代表完整可变字形或视觉质量。公开项目许可证尚未决定，适用仓库当前许可状态。

`upstream.json` 固定 Google Fonts commit 与五份原始字体、各自 OFL 1.1 许可证的 SHA-256/Git blob ID。通过 `tools/verification/fetch-font-corpus.py` 下载到忽略目录，未将这些第三方二进制纳入仓库或运行包。获取时校验许可证和字体，保持原始文件；Noto Sans SC 的许可声明含 Reserved Font Name `Source`。发行字体仍需随包保留相应版权及许可，并单独完成产品字体分发审查。

格式变体与损坏探针由 `tools/verification/font-fixtures.py` 在忽略目录生成，只修改原创字体。完整范围见[字体资源实现](../../docs/implementation/font-resources.md)。

`owned-metrics-hhea.ttf` / `owned-metrics-typo.ttf` 由 `tools/verification/metric-font-fixtures.py` 从原创 `owned.ttf` 生成，添加原创 avar 映射和 28 个 MVAR 度量记录，分别检查 hhea/typo 选择。摘要见 `owned-metrics.json`；许可与原始原创样本相同，不作为产品字体。独立计算见[字体实例度量](../../docs/implementation/font-instance-metrics.md)。

`owned-outlines.ttf/.otf` 由 `tools/verification/outline-font-fixtures.py` 生成原创二次/三次曲线、孔洞、负坐标、复合变换和 gvar 样本，摘要见 `owned-outlines.json`；许可与其他原创样本一致。它们只用于[轮廓验证](../../docs/implementation/font-outlines.md)，不作为产品字体。

`owned-carets*.ttf/.otf` 由 `tools/verification/caret-font-fixtures.py` 从原创
`owned.ttf` / `owned-outlines.ttf/.otf` 生成，验证非等距合字、GDEF 坐标/点/变体、复合与 off-curve 点、
超限和不可计算点。清单为 `owned-carets.json`，许可状态与上述原创字体一致。
FontTools 独立计算和 Native/WASM 证据见[合字光标](../../docs/implementation/font-ligature-carets.md)。
