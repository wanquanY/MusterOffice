# 实际静态交付文件的离线 XSD 检查

2026-09-30。对实际模型 V7 的两个三页版本，以及同一当前原生 Worker 生成的 10 页、
40 页文字/形状测量文件，新增独立的离线标准 Schema 验证。四份文件共 **156 个 XML
部件全部通过**，覆盖页面、母版、版式、主题、演示属性、核心元数据、内容类型和关系。
两版真实模型文件在已激活 `a86266c` 内核下的重算字节不变，来源见
[真实模型记录](live-agent-acceptance.md)与[完整导出测量](native-export-scaling.md)。

使用既有 ECMA-376 Part 4 Transitional XSD，以及 Part 2 OPC 官方 XSD。核心属性的
Dublin Core 与 W3C XML 依赖从规范指定的日期路径取得，保留原字节与摘要，未把历史
日期依赖替换为新版定义。所有材料下载完成后才运行检查；解析器禁用网络、DTD 加载
和外部实体展开，验证器不写默认属性、不修改 PPTX。字体故意改为非法 `sz=0` 的自有
反例被 XSD `minInclusive` 拒绝，未生成通过报告。

[检查器](../../tools/verification/pptx-static-schema-check.py)由调用者提供三个显式 Schema
目录与输入清单，根 Schema 由验证程序选择；文稿不能自行指定远程 Schema。已覆盖五类
XML 根命名空间，其他命名空间会明确失败，不能被当作已验证内容跳过。输入清单为
`[{"name":"owned-case","source":"/caller/selected/presentation.pptx"}]`，输出路径及输入文件
由调用者管理；这不是内核的新文件访问或持久存储接口。

```sh
python3 tools/verification/pptx-static-schema-check.py \
  "$MO_ECMA_XSD_DIRECTORY" "$MO_OPC_XSD_DIRECTORY" \
  "$MO_DUBLIN_CORE_XSD_DIRECTORY" "$MO_CASES_JSON" "$MO_NEW_REPORT_JSON"
```

Dublin Core 目录需要 `dc.xsd`、`dcterms.xsd`、`dcmitype.xsd` 和保存规范日期版本的
`xml-200103.xsd`。所用来源 URL、全部 Schema 摘要、逐文件/部件摘要、lxml 版本和反例
结果均在[证据文件](../reviews/evidence/2026-09-30-static-pptx-xsd.json)中。

此检查补充原有 OPC 关系图、内容类型、ZIP CRC 和交付摘要验证，不替代它们；也不证明
任意图表/工作簿语义、完整排版、Office/WPS 原生编辑、动画或媒体播放。真实 Artifact 的
历史质量声明没有被重写；正式替换门禁保持分别验收。
