"""Independent DOM-based native line declaration oracle (no streaming state)."""
from lxml import etree as E
from drawingml_reference import local,color
from mce_reference import A
from fill_reference import declarations

def line(node,ordinals):
    attr=lambda n,k:n.get(k).strip() if n.get(k) is not None else None
    retained=[]
    def attributes(n,allowed):
        if any(k not in allowed for k in n.attrib):retained.append(ordinals[n])
    attributes(node,['w','cap','cmpd','algn'])
    result={'sourceOrdinal':ordinals[node],'width':str(int(node.get('w'))) if node.get('w') is not None else None,
            'cap':attr(node,'cap'),'compound':attr(node,'cmpd'),'alignment':attr(node,'algn'),
            'fill':None,'dash':None,'join':None,'head':None,'tail':None,'retainedOrdinals':retained}
    def colored(n):
        if len(n)==0:return None
        assert len(n)==1;v=n[0]
        attributes(v,{'scrgbClr':['r','g','b'],'hslClr':['hue','sat','lum'],'sysClr':['val','lastClr']}.get(local(v),['val']))
        for t in v:attributes(t,[] if local(t) in ['comp','inv','gray','gamma','invGamma'] else ['val'])
        return color(v,ordinals)
    for n in node:
        name=local(n);ordinal=ordinals[n];binding={'sourceOrdinal':ordinal}
        assert E.QName(n).namespace==A
        if name in ['gradFill','pattFill']:
            parsed=declarations(n,ordinals);definition=dict(parsed['definition']);kind=definition.pop('kind')
            result['fill']={'kind':kind,**binding,kind:definition};retained.extend(parsed['retainedOrdinals'])
        elif name=='extLst':retained.append(ordinal)
        elif name in ['noFill','solidFill']:
            attributes(n,[]);result['fill']={'kind':'none' if name=='noFill' else 'solid',**binding}
            if name=='solidFill':result['fill']['color']=colored(n)
        elif name=='prstDash':
            attributes(n,['val']);result['dash']={'kind':'preset',**binding,'value':attr(n,'val')}
        elif name=='custDash':
            attributes(n,[]);stops=[]
            for ds in n:
                attributes(ds,['d','sp']);stops.append({'sourceOrdinal':ordinals[ds],'dash':attr(ds,'d'),'space':attr(ds,'sp')})
            result['dash']={'kind':'custom',**binding,'stops':stops}
        elif name in ['round','bevel','miter']:
            attributes(n,['lim'] if name=='miter' else [])
            result['join']={'kind':name,**binding}
            if name=='miter':result['join']['limit']=attr(n,'lim')
        elif name in ['headEnd','tailEnd']:
            attributes(n,['type','w','len']);result['head' if name=='headEnd' else 'tail']={**binding,'kind':attr(n,'type'),'width':attr(n,'w'),'length':attr(n,'len')}
        else:raise AssertionError(name)
    return result

def reference(node,ordinals):
    return {'sourceOrdinal':ordinals[node],'index':int(node.get('idx')),'color':color(node[0],ordinals) if len(node) else None,
            'retainedOrdinals':[ordinals[node]] if any(k!='idx' for k in node.attrib) else []}
