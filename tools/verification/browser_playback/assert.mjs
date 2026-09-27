export function check(value, message) {if (!value) throw Error(message);}
export const equal = (a,b) => JSON.stringify(canonical(a)) === JSON.stringify(canonical(b));
function canonical(value) {
  if (Array.isArray(value)) return value.map(canonical);
  if (value && typeof value === 'object') return Object.fromEntries(Object.keys(value).sort().map(k=>[k,canonical(value[k])]));
  return value;
}
export async function rejects(call, checkError) {
  try {await call();} catch (error) {check(checkError(error), 'Unexpected error: '+error); return;}
  throw Error('Expected rejection');
}
export const sha = async bytes => [...new Uint8Array(await crypto.subtle.digest('SHA-256',bytes))]
  .map(n=>n.toString(16).padStart(2,'0')).join('');
