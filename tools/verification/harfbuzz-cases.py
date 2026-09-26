"""Original multilingual inputs for the pinned shaping component, not paragraph-layout fixtures."""
import hashlib,json,struct
from pathlib import Path
from fontTools.ttLib import TTFont
root=Path('.codex-work/harfbuzz/cases');root.mkdir(exist_ok=True)
fonts={}
for row in json.loads(Path('fixtures/fonts/upstream.json').read_text()):
    if row['name'].endswith('.ttf'):
        path=Path('.codex-work/font-corpus')/row['family']/row['name']
        assert hashlib.sha256(path.read_bytes()).hexdigest()==row['sha256']
        fonts[row['family']]=str(path)
fonts.update({'owned':'fixtures/fonts/owned.ttf','collection':'fixtures/fonts/owned.ttc'})
cases=[]
def tag(s):return int.from_bytes(s.encode('ascii'),'big')
def add(name,family,text,script='Latn',language='en',direction=4,flags=67,level=0,features=(),variations=(),before='',after='',face=0,budget=262144):
    font=fonts[family];chars=[ord(c) for c in before+text+after]
    f=[]
    for name_,value,start,end in features:f.extend([tag(name_),value,start,end])
    v=[]
    for name_,value in variations:v.extend([tag(name_),struct.unpack('<I',struct.pack('<f',value))[0]])
    words=[0x4d4f4842,face,direction,tag(script),flags,level,len(chars),len(before),len(text),len(features),len(variations),budget,1]+chars+f+v
    path=root/(name+'.bin');path.write_bytes(struct.pack('<'+'I'*len(words),*words))
    cases.append({'name':name,'font':font,'fontSha256':hashlib.sha256(Path(font).read_bytes()).hexdigest(),'face':face,'upem':TTFont(font,fontNumber=face)['head'].unitsPerEm,
        'dottedCircleGlyph':TTFont(font,fontNumber=face).getGlyphID(TTFont(font,fontNumber=face).getBestCmap()[0x25cc]) if 0x25cc in TTFont(font,fontNumber=face).getBestCmap() else None,
        'text':text,'script':script,'language':language,'direction':direction,'flags':flags,'clusterLevel':level,'features':features,'variations':variations,
        'before':before,'after':after,'request':str(path),'requestSha256':hashlib.sha256(path.read_bytes()).hexdigest(),'expectedStatus':4 if budget==1 else 0})
add('latin-ligatures','notosans','office affine AVATAR')
add('latin-no-ligatures','notosans','office affine',features=[('liga',0,0,0xffffffff)])
add('latin-kern-off','notosans','AVATAR WA',features=[('kern',0,0,0xffffffff)])
add('latin-feature-range','notosans','office office',features=[('liga',0,0,6)])
add('latin-context-feature','notosans','office office',before='xx ',after=' yy',flags=64,features=[('liga',0,3,9)])
add('latin-overlapping-features','notosans','office office',features=[('liga',0,0,0xffffffff),('liga',1,0,6)])
add('latin-combining','notosans','a\u0301 e\u0323\u0302 x\u0307\u0308')
for level in [1,2,3]:add('cluster-'+str(level),'notosans','a\u0301 office',level=level)
add('latin-variation-weight','notosans','office AVATAR',variations=[('wght',750)])
add('latin-variation-width','notosans','office AVATAR',variations=[('wdth',80),('wght',500)])
add('arabic-joining','notosansarabic','السلام عليكم ورحمة الله','Arab','ar',5)
add('arabic-diacritics','notosansarabic','السَّلَامُ عَلَيْكُمْ','Arab','ar',5)
add('arabic-tatweel','notosansarabic','مرحبا بالعالم','Arab','ar',5,flags=67|128)
add('arabic-context','notosansarabic','لم','Arab','ar',5,flags=64,before='س',after='ان')
add('arabic-variation','notosansarabic','السلام عليكم','Arab','ar',5,variations=[('wght',700)])
add('deva-reorder','notosansdevanagari','क्षेत्र विज्ञान हिन्दी','Deva','hi')
add('deva-join-control','notosansdevanagari','क्\u200dष क्\u200cष','Deva','hi')
add('deva-broken','notosansdevanagari','\u093f\u093c','Deva','hi')
add('deva-no-circle','notosansdevanagari','\u093f\u093c','Deva','hi',flags=67|16)
add('cjk-horizontal','notosanssc','中文排版，标点（测试）。','Hani','zh-Hans')
add('cjk-vertical','notosanssc','中文排版，标点（测试）。','Hani','zh-Hans',6)
add('cjk-bottom-up','notosanssc','中文排版，标点（测试）。','Hani','zh-Hans',7)
add('cjk-variation','notosanssc','中文排版测试','Hani','zh-Hans',variations=[('wght',900)])
add('emoji-sequences','notoemoji','👨‍👩‍👧‍👦 👩🏽‍💻 🇨🇳','Zyyy','und')
add('emoji-selectors','notoemoji','❤︎ ❤️ ☀️','Zyyy','und')
add('ignorables-default','notosans','A\u200bB\u00adC\u200dD')
add('ignorables-preserve','notosans','A\u200bB\u00adC\u200dD',flags=67|4)
add('ignorables-remove','notosans','A\u200bB\u00adC\u200dD',flags=67|8)
add('owned-uvs','owned','A\ufe00 A\ufe01 α\U000e0100')
add('collection-cff','collection','Aα😀',face=1)
add('empty-run','notosans','')
add('output-budget','notosans','office affine',budget=1)
add('unknown-axis','notosans','office',variations=[('zzzz',100)]);cases[-1]['expectedStatus']=1
base=cases[0]
mutations=[('bad-magic',0,0),('bad-version',12,2),('bad-direction',2,0),('verify-flag',4,32),('conflicting-ignorables',4,12),('bad-level',5,4),
 ('text-budget',6,65537),('item-range',7,100),('bad-face',1,99),('bad-scalar',13,0xd800)]
for name,index,value in mutations:
    raw=Path(base['request']).read_bytes();words=list(struct.unpack('<'+'I'*(len(raw)//4),raw));words[index]=value
    path=root/(name+'.bin');path.write_bytes(struct.pack('<'+'I'*len(words),*words))
    c={**base,'name':name,'request':str(path),'requestSha256':hashlib.sha256(path.read_bytes()).hexdigest(),'expectedStatus':5 if name=='bad-face' else 1};cases.append(c)
(root/'manifest.json').write_text(json.dumps({'format':'musteroffice.harfbuzz-cases/1','cases':cases},ensure_ascii=False,indent=2)+'\n')
print('Generated',len(cases),'real font shaping/ABI cases')
