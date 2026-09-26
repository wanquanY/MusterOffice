"""Reconstruct the previous declaration contract, without hiding query changes."""
import copy
import hashlib
import json
from pathlib import Path
from lxml import etree as E
from mce_reference import project, A, P

sha=lambda b:hashlib.sha256(b).hexdigest()
read=lambda p:json.loads(Path(p).read_text())

def previous_index(response):
    result=copy.deepcopy(response); index=result.get('index',result)
    def add(v,ids):
        if ids:v['retainedOrdinals']=sorted(set(v['retainedOrdinals']+ids))
    def fill(value):
        if not value:return []
        d=value['definition'];ids=[]
        if d['kind']=='image' and d['blip']:
            ids=d['blip'].pop('effectNodes',[]);add(d['blip'],ids);add(value,ids)
        return ids
    for surface in index.get('surfaces',{}).values():
        surface.pop('effectNodes',None);surface.pop('rootGroupEffects',None)
        fill(surface.get('rootGroupFill'))
        for obj in surface['objects']:
            obj.pop('effects',None);obj.pop('effectReference',None)
            fill(obj.get('fill'));fill(obj.get('pictureFill'))
        bg=surface.get('background')
        if bg and bg['definition']['kind']=='properties':
            props=bg['definition'];ids=fill(props['fill']);effects=props.pop('effects',None)
            if effects:ids.append(effects['sourceOrdinal'])
            add(props,ids);add(bg,ids)
    for theme in index.get('themes',{}).values():
        theme.pop('effectNodes',None)
        if theme['formatScheme']:
            for entry in theme['formatScheme']['effects']:entry.pop('effectStyle',None)
            for family in ['fills','backgroundFills']:
                for entry in theme['formatScheme'][family]:fill(entry.get('fill'))
    return result

def compact(value):return json.dumps(value,ensure_ascii=False,separators=(',',':')).encode()

def compare_records(current,old):
    assert len(current)==len(old);changed=0
    for a,b in zip(current,old):
        if a==b:continue
        assert a.keys()==b.keys(),a.get('name');normalized=copy.deepcopy(a)
        if 'response' in normalized:normalized['response']=previous_index(normalized['response'])
        if 'responseSha256' in normalized and normalized['responseSha256']!=b['responseSha256']:
            value=previous_index(read(a['responsePath']));raw=compact(value)
            assert sha(raw)==b['responseSha256'],(a['name'],sha(raw),b['responseSha256'])
            normalized['responseSha256']=b['responseSha256']
        assert normalized==b,a.get('name');changed+=1
    return changed

def empty_background_expected(origin, physical, ordinals):
    """Independent expectation for the two complete background fills in the old corpus."""
    from drawingml_reference import color
    effect=physical[origin['sourceOrdinal']]
    assert effect.tag=='{'+A+'}effectLst' and not len(effect) and not effect.attrib
    props=effect.getparent();assert props.tag=='{'+P+'}bgPr'
    assert props.get('shadeToTitle','0') in ['0','false'] and len(props)==2
    fill=props[0];owner=origin['owner']
    def at(n):return {'kind':'declaration','owner':owner,'sourceOrdinal':ordinals[n]}
    def value(n,attr,default=None,schema=False):
        raw=n.get(attr)
        return {'value':raw if raw is not None else default,'declaredBy':at(n) if raw is not None else {'kind':'schemaDefault','part':owner['part'],'sourceOrdinal':ordinals[n]} if schema else {'kind':'profileDefault'}}
    def paint(n):
        c=color(n,ordinals);c.pop('sourceOrdinal');c['declaredBy']=at(n)
        return {'color':c,'contextOwner':owner}
    result={'status':'resolved','fill':{'kind':'solid' if fill.tag=='{'+A+'}solidFill' else 'gradient','declaredBy':at(fill)},'redirects':[]}
    if fill.tag=='{'+A+'}solidFill':
        assert len(fill)==1 and not fill.attrib;result['fill']['color']=paint(fill[0]);return result
    assert fill.tag=='{'+A+'}gradFill'
    gs,path,tile=list(fill);assert [E.QName(n).localname for n in [gs,path,tile]]==['gsLst','path','tileRect']
    rect=path[0];assert rect.tag=='{'+A+'}fillToRect'
    def rectangle(n,default,schema=False):
        return {'declaredBy':at(n),**{out:value(n,key,default,schema) for out,key in [('left','l'),('top','t'),('right','r'),('bottom','b')]}}
    result['fill']['gradient']={
        'stops':{'value':[{'position':value(n,'pos'),'color':paint(n[0])} for n in gs],'declaredBy':at(gs)},
        'shade':{'kind':'path','declaredBy':at(path),'path':value(path,'path'),'fillToRect':rectangle(rect,'50000')},
        'tileRect':rectangle(tile,'0',True),
        'flip':value(fill,'flip'),'rotateWithShape':{'value':fill.get('rotWithShape') in ['true','1'],'declaredBy':at(fill)},
    }
    return result

def compare_fill_queries(current,old,snapshots):
    import zipfile
    from effect_reference import BLIP_EFFECTS
    changes=[];assert len(current)==len(old)
    for a,b in zip(current,old):
        assert a.keys()==b.keys(); normalized=copy.deepcopy(a)
        if 'inspectionSha256' in a and a['inspectionSha256']!=b['inspectionSha256']:
            assert sha(compact(previous_index(read(a['inspectionPath']))))==b['inspectionSha256'],a['name']
            normalized['inspectionSha256']=b['inspectionSha256']
        raw=Path(snapshots/(b['name']+'.json')).read_bytes();assert sha(raw)==b['responseSha256']
        before=json.loads(raw);after=read(a['responsePath']);reconstructed=copy.deepcopy(after)
        kinds=[]
        if before!=after:
            assert before['status']==after['status']=='evaluated'
            with zipfile.ZipFile(a['sourcePath']) as z:parts={n:z.read(n) for n in z.namelist()}
            assert len(before['styles']['targets'])==len(after['styles']['targets'])
            for pos,(x,y) in enumerate(zip(before['styles']['targets'],after['styles']['targets'])):
                if x==y:continue
                assert x['target']==y['target']
                reason=x['outcome']['reason'];assert reason['kind']=='retainedContent'
                origin=reason['origin'];assert origin['kind']=='declaration'
                part=origin['owner']['part'];dom,_,ordinals=project(parts[part.lstrip('/')],with_ordinals=True);physical={v:n for n,v in ordinals.items()}
                if x['target']['kind']=='background':
                    assert y['outcome']==empty_background_expected(origin,physical,ordinals),a['name']
                    kinds.append('known-empty-background-list')
                else:
                    old_node=physical[origin['sourceOrdinal']]
                    assert E.QName(old_node).namespace==A and E.QName(old_node).localname in BLIP_EFFECTS
                    assert old_node.getparent().tag=='{'+A+'}blip'
                    new_reason=y['outcome']['reason'];new_origin=new_reason['origin']
                    if new_reason['kind']=='effectEvaluationRequired':
                        assert new_origin==origin
                        kinds.append('typed-image-effect-diagnostic')
                    else:
                        assert new_reason['kind']=='retainedContent'
                        candidate=physical[new_origin['sourceOrdinal']]
                        assert candidate.tag=='{'+A+'}extLst' and candidate.getparent() is old_node.getparent()
                        assert {**new_origin,'sourceOrdinal':origin['sourceOrdinal']}==origin
                        kinds.append('remaining-image-extension-diagnostic')
                    assert y['outcome']=={'status':'unresolved','reason':new_reason}
                reconstructed['styles']['targets'][pos]=x
            assert reconstructed==before,a['name']
            normalized['responseSha256']=b['responseSha256']
            changes.append({'name':a['name'],'previousResponseSha256':b['responseSha256'],'responseSha256':a['responseSha256'],'verifiedChanges':kinds})
        assert normalized==b,a['name']
    return changes
