
const fs=require('fs'),path=require('path');const D=process.argv[2],raw=path.join(D,'raw');
const env=n=>{try{return JSON.parse(fs.readFileSync(path.join(raw,n+'.out'),'utf8').replace(/[\r\n]+$/,''));}catch(e){return null;}};
const rc=n=>fs.readFileSync(path.join(raw,n+'.rc'),'utf8').trim();
for(const n of ['c11','c36','c37','c38','c39','r0','r1','rcanon']){
  const e=env(n); if(!e){console.log(n+' NO ENV size='+fs.statSync(path.join(raw,n+'.out')).size);continue;}
  const t=(e.timings||[]).map(x=>x.position+':'+x.resultKind+':'+x.hasOutput).join(' ');
  let res=''; if(e.result){const f=(e.result.structured&&e.result.structured.fields)||[];res='result:'+e.result.kind+(e.result.structured?('('+e.result.structured.type+')'):'')+' fields='+f.map(x=>x.name+'='+(x.value&&x.value.value!==undefined?x.value.value:x.value&&x.value.kind)).join(',')+' display='+JSON.stringify((e.result.display||'').slice(0,80));}
  console.log([n,'rc='+rc(n),'ok='+e.ok,'code='+e.code,'elapsed='+e.elapsed,'timings=['+t+']','output='+JSON.stringify(e.output),res,'msg='+JSON.stringify(e.message||'')].join(' | '));
}
// truncation fields deep scan for c38/c39
for(const n of ['c38','c39']){ const e=env(n); const hits=[]; (function w(v,p){ if(v&&typeof v==='object'){for(const k of Object.keys(v)){ if(k==='truncated'||k==='budget'||k==='truncationReason') hits.push(p+'.'+k+'='+JSON.stringify(v[k])); w(v[k],p+'.'+k);} }})(e,''); console.log('TRUNC '+n+': '+(hits.join(' ')||'NONE')+' prettySample='+JSON.stringify((JSON.stringify(e).match(/\"pretty\":\"[^\"]{0,50}/g)||[]).slice(0,4))); }
