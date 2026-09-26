"""Decimal color composition over independent XML/layer declarations."""
import importlib.util,math
from pathlib import Path
from drawingml_reference import color as declaration
spec=importlib.util.spec_from_file_location('decimal_colors',Path(__file__).with_name('pptx-color-independent.py'))
maths=importlib.util.module_from_spec(spec);spec.loader.exec_module(maths)
D=maths.D;unit=maths.unit;encode=maths.encode;decode=maths.decode;pct=maths.pct
Unresolved=maths.Unresolved

def evaluate(expression,placeholder,env,context,presets,coverage):
 mapping,map_ref,scheme,scheme_ref,ordinals=env;deps=[];notices=[]
 def notice(name):
  if name not in notices:notices.append(name)
 def resolve(name,stack,inside):
  if name=='phClr':
   candidate = (placeholder() if callable(placeholder) else placeholder) if not inside else None
   if candidate is not None:
    deps.append({'kind':'placeholder'});return color(candidate,stack,True)
   if context['placeholder'] is None:raise Unresolved('missingPlaceholder')
   deps.append({'kind':'placeholder'});v=[D(v)/255 for v in context['placeholder']];return v[:3],v[3],None
  if name in ['dk1','lt1','dk2','lt2']:slot=name
  else:
   if mapping is None:raise Unresolved('missingColorMap')
   slot=mapping[name]
  if slot in stack:raise Unresolved('schemeCycle',slot=slot)
  if scheme is None:raise Unresolved('missingColorScheme')
  if slot not in scheme:raise Unresolved('missingThemeSlot',slot=slot)
  node=scheme[slot];deps.append({'kind':'theme','part':scheme_ref['part'],'slot':slot,'sourceOrdinal':ordinals[node]})
  return color(declaration(node,ordinals),stack+[slot],inside)
 def color(term,stack,inside):
  val=term['value'];kind=val['kind'];coverage['models'].add(kind);alpha=D(1);hls=None
  if kind=='srgb':rgb=[D(v)/255 for v in val['rgb']]
  elif kind=='scRgb':rgb=[encode(pct(val[k])) for k in ['red','green','blue']]
  elif kind=='hsl':
   hls=[D(val['hue'])/21600000,unit(pct(val['luminance'])),unit(pct(val['saturation']))];rgb=maths.hls_to_rgb(*hls)
  elif kind=='preset':
   coverage['presets'].add(val['color']);rgb=[D(v)/255 for v in presets[val['color']]]
   if val['color']=='ltGoldenrodYellow':notice('presetAliasDiscrepancy')
  elif kind=='scheme':rgb,alpha,hls=resolve(val['slot'],stack,inside)
  else:
   assert kind=='system';sys=val['color']
   if sys in context['systemColors']:rgb=context['systemColors'][sys];origin='hostContext'
   elif val['lastColor'] is not None:rgb=val['lastColor'];origin='fileLastColor'
   else:raise Unresolved('missingSystemColor',color=sys)
   deps.append({'kind':'system','color':sys,'origin':origin});rgb=[D(v)/255 for v in rgb]
  for transform in term['transforms']:
   t=transform['kind'];coverage['transforms'].add(t)
   v=transform.get('value');parameter=(D(v)/21600000 if t in ['hue','hueOff'] else pct(v)) if v is not None else None
   def apply(v):return unit(v+parameter if t.endswith('Off') else v*parameter if t.endswith('Mod') else parameter)
   if not t.startswith(('alpha','hue','lum','sat')) and t!='comp':hls=None
   if t.startswith('alpha'):alpha=apply(alpha)
   elif t.startswith(('red','green','blue')):
    i=0 if t.startswith('red') else 1 if t.startswith('green') else 2;rgb[i]=apply(rgb[i])
   elif t.startswith(('hue','lum','sat')) or t=='comp':
    if hls is None:hls=maths.rgb_to_hls(*map(unit,rgb))
    i=0 if t.startswith('hue') or t=='comp' else 1 if t.startswith('lum') else 2
    hls[i]=(hls[i]+D('.5'))%1 if t=='comp' else apply(hls[i]);rgb=maths.hls_to_rgb(*hls)
   elif t=='tint':rgb=[encode(unit((1-parameter)+decode(v)*parameter)) for v in rgb]
   elif t=='shade':rgb=[encode(unit(decode(v)*parameter)) for v in rgb]
   elif t=='inv':rgb=[encode(unit(1-decode(v))) for v in rgb]
   elif t=='gray':
    rgb=[unit(sum(v*D(w) for v,w in zip(rgb,['.22','.72','.06'])))]*3;notice('grayWeightsProvisional')
   elif t=='gamma':rgb=[encode(unit(v)) for v in rgb]
   else:
    assert t=='invGamma';rgb=[decode(unit(v)) for v in rgb]
  return rgb,alpha,hls
 try:
  rgb,alpha,_=color(expression,[],False);srgb=rgb+[alpha];linear=[decode(v) for v in rgb]+[alpha]
  if not all(math.isfinite(float(v)) for v in srgb+linear):raise Unresolved('numericRange')
  outcome={'status':'resolved','srgb':[float(v) for v in srgb],'linear':[float(v) for v in linear],'rgba8':[maths.sample(v,255) for v in srgb],'rgba16':[maths.sample(v,65535) for v in srgb],'clippedForSrgb':any(v<0 or v>1 for v in rgb)}
 except Unresolved as e:outcome={'status':'unresolved','reason':e.reason}
 return {'kind':'solid','outcome':outcome,'dependencies':deps,'notices':notices}
