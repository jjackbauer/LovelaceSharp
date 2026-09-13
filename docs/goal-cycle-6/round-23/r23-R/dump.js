
const fs=require('fs'),path=require('path');
const D=process.argv[2], raw=path.join(D,'raw');
const env=n=>JSON.parse(fs.readFileSync(path.join(raw,n+'.out'),'utf8').replace(/[\r\n]+$/,''));
for(const n of ['c18','c25','c19']){ const e=env(n); console.log('=== '+n+' rc-env keys: '+Object.keys(e).join(',')); console.log('  code='+e.code+' cat='+e.category+' msg='+JSON.stringify(e.message)); console.log('  diags='+JSON.stringify(e.diagnostics)); }
const e16=env('c16');
console.log('=== c16 result keys: '+Object.keys(e16.result).join(',')+' structuredType='+(e16.result.structured&&e16.result.structured.type));
console.log('=== c16 structured fields: '+JSON.stringify((e16.result.structured.fields||[]).map(f=>f.name)));
const e19=env('c19');
console.log('=== c19 result keys: '+(e19.result?Object.keys(e19.result).join(','):'NONE')+' structured fields: '+JSON.stringify(((e19.result||{}).structured||{}).fields?e19.result.structured.fields.map(f=>f.name):null));
// write replay scripts from the published pretty values
function sol(n){ const e=env(n); const fs_=e.result.structured.fields; const s=fs_.find(f=>f.name==='solutions').value; return {status:fs_.find(f=>f.name==='status').value.value, complete:fs_.find(f=>f.name==='complete').value.value, completeness:fs_.find(f=>f.name==='completeness').value.value, shape:s.shape, vals:s.elements.map(el=>el.fields.find(f=>f.name==='value').value.pretty), canon:s.elements.map(el=>el.fields.find(f=>f.name==='value').value.canonical), rep:fs_.find(f=>f.name==='represented_count').value.value, unrep:fs_.find(f=>f.name==='unrepresented_count').value.value, reason:fs_.find(f=>f.name==='unrepresented_reason').value}; }
const s16=sol('c16'); console.log('=== c16 SOLVE: '+JSON.stringify(s16));
const s19=sol('c19'); console.log('=== c19 SOLVE: '+JSON.stringify(s19));
fs.writeFileSync(path.join(D,'ls','c36.ls'),'print("a"); print()\n');
fs.writeFileSync(path.join(D,'ls','c37.ls'),'dft("x")\n');
s16.vals.forEach((v,i)=>{ fs.writeFileSync(path.join(D,'ls','r'+i+'.ls'),'x = '+v+'; x^2 - 4\n'); });
fs.writeFileSync(path.join(D,'ls','r-canon.ls'),'x = '+s16.canon[0]+'; x\n');
console.log('=== wrote replay scripts: '+s16.vals.map((v,i)=>'r'+i+'.ls(x='+v+')').join(' '));
