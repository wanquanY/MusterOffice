"""Independent Decimal geometry arithmetic, with series rather than libm.

80-digit working precision; atan via repeated half-angle reduction, and pi via
Machin's identity. Geometry verification is not target-application acceptance.
"""
from decimal import Decimal as D, getcontext
getcontext().prec = 80
EPS = D('1e-85')
TURN = D(21600000)
def atan_series(x):
    term = x
    result = x
    n = 1
    while True:
        term *= -x*x
        item = term / (2*n+1)
        result += item
        if abs(item) < EPS: return result
        n += 1
        assert n < 10000
PI = 16*atan_series(D(1)/5) - 4*atan_series(D(1)/239)
def atan(x):
    if x < 0: return -atan(-x)
    if x > 1: return PI/2-atan(1/x)
    for _ in range(4): x = x / (1+(1+x*x).sqrt())
    return 16*atan_series(x)
def angle(x, y):
    if x == 0:
        if y == 0: raise ArithmeticError('undefinedDirection')
        return TURN/4 if y > 0 else -TURN/4
    a = atan(y/x)
    if x < 0: a += PI if y >= 0 else -PI
    return a*TURN/(2*PI)
def trig(a):
    a %= TURN
    if a < 0: a += TURN
    cardinals = {D(0):(D(1),D(0)),TURN/4:(D(0),D(1)),TURN/2:(D(-1),D(0)),3*TURN/4:(D(0),D(-1))}
    if a in cardinals: return cardinals[a]
    if a > TURN/2: a -= TURN
    x = a*(2*PI)/TURN
    c = ct = D(1)
    s = st = x
    for k in range(1, 200):
        ct *= -x*x / ((2*k-1)*(2*k))
        st *= -x*x / ((2*k)*(2*k+1))
        c += ct
        s += st
        if max(abs(ct),abs(st)) < EPS: return c,s
    raise AssertionError('trig convergence')
ARITY = {'val':1,'abs':1,'sqrt':1,'at2':2,'cos':2,'sin':2,'tan':2,'min':2,'max':2,'*/':3,'+-':3,'+/':3,'?:':3,'cat2':3,'sat2':3,'mod':3,'pin':3}
def compute(op, a):
    x = a[0]
    y = a[1] if len(a)>1 else D(0)
    z = a[2] if len(a)>2 else D(0)
    if op == 'val': return x
    if op == 'abs': return abs(x)
    if op == 'sqrt': return abs(x).sqrt()
    if op in ['*/','+/']:
        if z == 0: raise ArithmeticError('divisionByZero')
        return (x*y if op=='*/' else x+y)/z
    if op == '+-': return x+y-z
    if op == '?:': return y if x>0 else z
    if op == 'min': return min(x,y)
    if op == 'max': return max(x,y)
    if op == 'pin': return x if y<x else z if y>z else y
    if op == 'mod': return (x*x+y*y+z*z).sqrt()
    if op == 'at2': return angle(x,y)
    if op in ['cat2','sat2']:
        if y == z == 0: raise ArithmeticError('undefinedDirection')
        return x*(y if op=='cat2' else z)/(y*y+z*z).sqrt()
    if op in ['sin','cos']: return x*trig(y)[op=='sin']
    if op == 'tan':
        c,s = trig(y)
        if c == 0: raise ArithmeticError('tangentPole')
        return x*s/c
    raise ArithmeticError('unknownOperation')
def builtins(w,h):
    values = {'w':w,'h':h,'l':D(0),'t':D(0),'r':w,'b':h,'hc':w/2,'vc':h/2,'ss':min(w,h),'ls':max(w,h)}
    for prefix, size, divisors in [('wd',w,[2,3,4,5,6,8,10]),('hd',h,[2,3,4,5,6,8]),('ssd',min(w,h),[2,4,6,8,16,32])]:
        values.update({prefix+str(n):size/n for n in divisors})
    for name,n,d in [('cd2',1,2),('cd4',1,4),('cd8',1,8),('3cd4',3,4),('3cd8',3,8),('5cd8',5,8),('7cd8',7,8)]: values[name]=TURN*n/d
    return values
