"""Native declarations -> independent high-precision coordinate values."""
import math
import re
from decimal import Decimal as D
from guide_decimal import ARITY, builtins, compute

DECIMAL = re.compile(r'^[+-]?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)$')
class Unresolved(Exception):
    def __init__(self, kind, ordinal=None, **extra):
        self.reason = {'kind':kind, **({'origin':{'kind':'document','sourceOrdinal':ordinal}} if ordinal is not None else {}), **extra}
def evaluate(g, width, height, additional_builtins=None):
    if g['retainedOrdinals']: raise Unresolved('retainedContent',g['retainedOrdinals'][0])
    d = g['definition']
    if d['kind']=='preset': raise Unresolved('presetNotExpanded',preset=d['preset'])
    environment = builtins(D(width),D(height))
    environment.update(additional_builtins or {})
    native = set(environment)
    origins = {}
    adjustments = {}
    def finite(v, n):
        if not math.isfinite(float(v)): raise Unresolved('numericRange',n)
        return v
    def guides(key):
        output=[]
        for guide in (d[key] or {'entries':[]})['entries']:
            n = guide['sourceOrdinal']; name = guide['name'].strip()
            if not name or any(c.isspace() for c in name) or DECIMAL.fullmatch(name): raise Unresolved('invalidGuideName',n)
            if name in native: raise Unresolved('reservedGuide',n)
            f = guide['formula'].split(); op=f[0] if f else ''
            if op not in ARITY: raise Unresolved('formula',n,issue='unknownOperation')
            if len(f)!=ARITY[op]+1: raise Unresolved('formula',n,issue='arity')
            args=[];deps=[]
            for token in f[1:]:
                if token in environment:
                    args.append(environment[token]);deps.append({'kind':'guide','sourceOrdinal':origins[token]} if token in origins else {'kind':'builtin','name':token})
                elif DECIMAL.fullmatch(token): args.append(finite(D(token),n))
                else: raise Unresolved('unknownReference',n,token=token)
            try: value=finite(compute(op,args),n)
            except ArithmeticError as e: raise Unresolved('formula',n,issue=str(e)) from e
            environment[name]=value;origins[name]=n
            if key=='adjustments': adjustments[name]=n
            output.append({'sourceOrdinal':n,'name':name,'value':value,'dependencies':deps})
        return output
    def scalar(raw,n,angle=False):
        token=raw.strip()
        if token in environment: return environment[token]
        if re.fullmatch(r'[+-]?[0-9]+',token):
            v=int(token)
            low,high=(-2147483648,2147483647) if angle else (-27273042329600,27273042316900)
            if low<=v<=high:return D(v)
            raise Unresolved('invalidAngle' if angle else 'invalidCoordinate',n)
        if angle and DECIMAL.fullmatch(token): raise Unresolved('invalidAngle',n)
        if not angle:
            for unit,factor in [('mm',36000),('cm',360000),('in',914400),('pt',12700),('pc',152400),('pi',152400)]:
                if token.endswith(unit):
                    raw=token[:-2]
                    if not DECIMAL.fullmatch(raw):raise Unresolved('invalidCoordinate',n)
                    return finite(D(raw)*factor,n)
        raise Unresolved('unknownReference',n,token=token)
    def point(p):return {'sourceOrdinal':p['sourceOrdinal'],'x':scalar(p['x'],p['sourceOrdinal']),'y':scalar(p['y'],p['sourceOrdinal'])}
    def handle(h):
        n=h['sourceOrdinal'];v={'sourceOrdinal':n,'kind':h['kind'],'position':point(h['position'])}
        for key,raw in h.items():
            if key in v:continue
            if raw is None:v[key]=None
            elif key.startswith('guide'):
                if raw.strip() not in adjustments:raise Unresolved('invalidHandleReference',n)
                v[key]=adjustments[raw.strip()]
            else:v[key]=scalar(raw,n,key.endswith('Angle'))
        return v
    def command(c):
        n=c['sourceOrdinal'];v={'sourceOrdinal':n,'kind':c['kind']}
        for key,value in c.items():
            if key in v:continue
            v[key]=point(value) if isinstance(value,dict) else scalar(value,n,key.endswith('Angle'))
        return v
    result={'sourceOrdinal':g['sourceOrdinal'],'adjustments':guides('adjustments'),'guides':guides('guides')}
    result['handles']=[handle(h) for h in (d['handles'] or {'entries':[]})['entries']]
    result['connections']=[{'sourceOrdinal':c['sourceOrdinal'],'angle':scalar(c['angle'],c['sourceOrdinal'],True),'position':point(c['position'])} for c in (d['connections'] or {'entries':[]})['entries']]
    r=d['textRect'];result['textRect']=None if r is None else {'sourceOrdinal':r['sourceOrdinal'],**{k:scalar(r[k],r['sourceOrdinal']) for k in ['left','top','right','bottom']}}
    result['paths']=[{**p,'commands':[command(c) for c in p['commands']]} for p in d['paths']['entries']]
    def locate(value, root=False):
        if isinstance(value,list):return [locate(v) for v in value]
        if not isinstance(value,dict):return value
        out={}
        for key,v in value.items():
            if key=='sourceOrdinal' and not root:out['origin']={'kind':'document','sourceOrdinal':v}
            elif key in ['guideX','guideY','guideRadius','guideAngle'] and v is not None:out[key]={'kind':'document','sourceOrdinal':v}
            else:out[key]=locate(v)
        return out
    return locate(result,True)
