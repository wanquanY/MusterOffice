"""Exact rational power-basis/Sturm reference, independent of the C++ kernel.

Factored from the first scalar verification; no sampled root-search assumption.
"""
from fractions import Fraction as F

def trim(p):
    while len(p)>1 and p[-1]==0: p.pop()
    return p

def add(a,b): return trim([(a[i] if i<len(a) else F(0))+(b[i] if i<len(b) else F(0)) for i in range(max(len(a),len(b)))])

def neg(a): return [-v for v in a]

def mul(a,b):
    out=[F(0)]*(len(a)+len(b)-1)
    for i,x in enumerate(a):
        for j,y in enumerate(b): out[i+j]+=x*y
    return trim(out)

def square(a): return mul(a,a)

def poly(values):
    x,y,cx,cy,sx,sy=map(F,values)
    ax=square([x-cx,cx]); ay=square([y-cy,cy])
    bx=square([sx,1-sx]); by=square([sy,1-sy])
    p=add(add(mul(ax,by),mul(ay,bx)),neg(mul(bx,by)))
    while len(p)>1 and p[0]==0: p=p[1:]
    return p

def inside(values):
    x,y,cx,cy,sx,sy=map(F,values)
    if (sx==0 and x!=cx) or (sy==0 and y!=cy): return False
    return (F(0) if sx==0 else ((x-cx)/sx)**2)+(F(0) if sy==0 else ((y-cy)/sy)**2)<=1

def remainder(a,b):
    a=a.copy()
    while len(a)>=len(b) and a!=[0]:
        n=len(a)-len(b); k=a[-1]/b[-1]
        for i,v in enumerate(b): a[n+i]-=k*v
        trim(a)
    return a

def sturm(p):
    seq=[p,trim([i*p[i] for i in range(1,len(p))]) or [F(0)]]
    if seq[-1]==[0]: return seq[:-1]
    while True:
        r=neg(remainder(seq[-2],seq[-1]))
        if r==[0]: return seq
        scale=abs(r[-1]); seq.append([v/scale for v in r])

def at(p,x):
    out=F(0)
    for v in reversed(p): out=out*x+v
    return out

def variations(seq,x):
    s=[1 if v>0 else -1 for p in seq if (v:=at(p,x))!=0]
    return sum(a!=b for a,b in zip(s,s[1:]))
