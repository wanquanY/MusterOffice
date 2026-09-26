"""Check typed native enum facts against the pinned official schema."""
import hashlib,json,re
from pathlib import Path
from lxml import etree as E
SCHEMA=Path('.codex-work/ecma376/xsd/dml-main.xsd');SOURCE=Path('crates/mo-pptx/src/source/text/names.rs')
sha=lambda b:hashlib.sha256(b).hexdigest()
assert sha(SCHEMA.read_bytes())=='6978ba7e889070b0c3cb5b546b23e5a6c3516134afc53b87a21f482ca33f3858'
types={'NativeTextAnchor':'ST_TextAnchoringType','NativeTextVerticalOverflow':'ST_TextVertOverflowType','NativeTextHorizontalOverflow':'ST_TextHorzOverflowType','NativeTextVertical':'ST_TextVerticalType','NativeTextWrap':'ST_TextWrappingType','NativeTextUnderline':'ST_TextUnderlineType','NativeTextStrike':'ST_TextStrikeType','NativeTextCaps':'ST_TextCapsType','NativeTextAlign':'ST_TextAlignType','NativeTextFontAlign':'ST_TextFontAlignType','NativeTextTabAlign':'ST_TextTabAlignType','NativeTextAutonumber':'ST_TextAutonumberScheme','NativeFontCollectionIndex':'ST_FontCollectionIndex'}
schema=E.parse(SCHEMA);text=SOURCE.read_text();checked={};X={'x':'http://www.w3.org/2001/XMLSchema'}
for rust,native in types.items():
    expected=[n.get('value') for n in schema.findall(f"x:simpleType[@name='{native}']/x:restriction/x:enumeration",X)]
    body=re.search(r'pub enum '+rust+r' \{([^}]+)\}',text).group(1);actual=re.findall(r'#\[serde\(rename = "([^"]+)"\)\]',body)
    assert actual==expected,(rust,actual,expected);checked[rust]={'nativeType':native,'values':actual}
result={'format':'musteroffice.source-text-enums/1','sourcePath':str(SOURCE),'sourceSha256':sha(SOURCE.read_bytes()),'xsdPath':str(SCHEMA),'xsdSha256':sha(SCHEMA.read_bytes()),'enumTypes':len(checked),'values':sum(len(v['values']) for v in checked.values()),'enums':checked}
Path('.codex-work/source-text/enums.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'enumTypes':result['enumTypes'],'values':result['values']}))
