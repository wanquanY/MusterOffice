/** Owned low-level V12 fixtures; not a PPTX source compiler. */
export const floatWord=n=>new Uint32Array(new Float32Array([n]).buffer)[0];
const f=floatWord;
export function ellipticFrame({width=128,height=64,matrix=[128,0,0,0,64,0],scale=[.8,.6],field=[0,0,0,0],tile=[0,0],draws=1,office=false}={}){
 const r=[0x4d4f534b,12,width,height,0xffffffff,1,draws,5,0,1,0,0,0,0,0,5];
 for(const [op,x,y] of [[1,0,0],[2,width,0],[2,width,height],[2,0,height],[5,0,0]])r.push(op,f(x),f(y),0,0,0,0);
 r.push(4,0,office?2:0,0,office?2:3,...matrix.map(f),...tile,...scale.map(f),...field.map(f));
 for(const [p,g] of office?[[0,32],[1,224]]:[[0,32],[.5,128],[1,224]])r.push(f(p),f(g/255),f(g/255),f(g/255),f(1));
 for(let i=0;i<draws;++i)r.push(0,0,0,0,0,1,0,0);
 return new Uint32Array(r);
}
