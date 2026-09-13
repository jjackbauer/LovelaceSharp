
const fs=require('fs'),path=require('path');
const D=process.argv[2], raw=path.join(D,'raw');
function env(n){ try{ return JSON.parse(fs.readFileSync(path.join(raw,n+'.out'),'utf8').replace(/[\r\n]+$/,'')); }catch(e){ return null; } }
function rc(n){ return fs.readFileSync(path.join(raw,n+'.rc'),'utf8').trim(); }
function fields(rec){ const o={}; for(const f of (rec.structured&&rec.structured.fields)||[]) o[f.name]=f.value; return o; }
function walk(v,fn,p){ fn(v,p); if(v&&typeof v==='object'){ for(const k of Object.keys(v)) walk(v[k],fn,p+'.'+k);} }
const L=[];
// --- I-1 byte accounting: stdout = envelope + one CRLF; stderr 0 bytes
for(const f of fs.readdirSync(raw).filter(x=>x.endsWith('.out'))){
  const n=f.slice(0,-4); const b=fs.readFileSync(path.join(raw,f));
  const errB=fs.statSync(path.join(raw,n+'.err')).size;
  let verdict;
  if(b.length===0){ verdict='EMPTY-STDOUT'; }
  else{
    const s=b.toString('utf8');
    const m=s.match(/[\r\n]+$/); const term=m?m[0]:'';
    const body=s.slice(0,s.length-term.length);
    let okBody=false,extra='';
    try{ JSON.parse(body); okBody=true; }catch(e){ extra=e.message.slice(0,60); }
    const onlyCRLF = term===''||term==='\r\n';
    verdict=(okBody?'JSON':('NOT-JSON:'+extra))+' term='+JSON.stringify(term)+(onlyCRLF?'':' NONSTANDARD')+(b.length>Buffer.byteLength(body)+2?' TAIL-BYTES':'');
  }
  L.push(['I-1',n,'rc='+rc(n),'stdout='+b.length+'B stderr='+errB+'B',verdict].join(' | '));
}
// --- durations agree (human string vs elapsedTime)
for(const n of ['c01','c04','c06','c08','c12','c13','c15','c28missing','c18','c09']){
  const e=env(n); if(!e){L.push(['I-4dur',n,'NO ENV'].join(' | '));continue;}
  const m=(e.elapsed||'').match(/^([0-9.]+)\s*(\S+)$/);
  const et=e.elapsedTime||{};
  const agree = m && Math.abs(parseFloat(m[1])-et.value)<1e-9 && m[2]===et.unit;
  L.push(['I-4dur',n,'elapsed='+JSON.stringify(e.elapsed),'elapsedTime='+JSON.stringify(e.elapsedTime),agree?'AGREE':'DISAGREE'].join(' | '));
}
// --- print stream vs hasOutput; timings positions slice the file
const srcmap={c01:'c01.ls',c03:'c03.ls',c05:'c05.ls',c06:'c06.ls',c07:'c07.ls',c29:'c29.ls',c30:'c30.ls',c31:'c31.ls',c21:'c21.ls',c21crlf:'c21-crlf.ls',c21bom:'c21-bomcrlf.ls',c21nonl:'c21-nonl.ls',c11:'c11.ls',c26stdin:'c11.ls'};
for(const n of Object.keys(srcmap)){
  const e=env(n); if(!e){L.push(['I-5',n,'NO ENV'].join(' | '));continue;}
  const src=fs.readFileSync(path.join(D,'ls',srcmap[n]),'utf8');
  const t=(e.timings||[]).map(x=>x.position+':'+x.resultKind+':'+x.hasOutput).join(' ');
  const slices=(e.timings||[]).map(x=>JSON.stringify((src.slice(x.position)||'').split(/[\r\n]/)[0].slice(0,22))).join(' ');
  const has=(e.timings||[]).filter(x=>x.hasOutput).length;
  L.push(['I-5',n,'rc='+rc(n),'output='+JSON.stringify(e.output),'hasOutputTrue='+has,'timings='+t,'slice='+slices].join(' | '));
}
// --- error diagnostics positions vs timings + messages (doc example byte check)
for(const n of ['c06','c07','c08','c14','c15','c21','c22','c33','c34','c28missing']){
  const e=env(n); if(!e){L.push([n,'NO ENV'].join(' | '));continue;}
  const d=(e.diagnostics||[]).map(x=>'pos='+x.position+' L'+x.line+' C'+x.column).join(';');
  L.push(['I-6',n,'code='+e.code+' cat='+e.category,'rc='+rc(n),'timings=['+(e.timings||[]).map(x=>x.position).join(',')+']','diags='+d,'msg='+JSON.stringify(e.message)].join(' | '));
}
// --- solve record: table, counts, shapes, diagnostic codes
for(const n of ['c16','c35','c19']){
  const e=env(n); if(!e||!e.result){L.push([n,'NO RESULT'].join(' | '));continue;}
  const f=fields(e.result.structured);
  const sol=f.solutions||{}; const rr=f.unrepresented_reason||{};
  const dg=(f.diagnostics&&f.diagnostics.elements||[]).map(d=>{const q=fields(d);return q.code.value+'/'+q.category.value+'/rec='+q.recoverable.value;}).join(',');
  L.push([n,'status='+(f.status&&f.status.value)+' complete='+(f.complete&&f.complete.value)+' completeness='+(f.completeness&&f.completeness.value),
    'sol.shape='+JSON.stringify(sol.shape)+' sol.n='+(sol.elements||[]).length+' represented='+(f.represented_count&&f.represented_count.value)+' unrepresented='+(f.unrepresented_count&&f.unrepresented_count.value),
    'unrep_reason='+JSON.stringify(rr.value),'diag='+dg,
    'consistency: shape==n:'+((sol.elements||[]).length===((sol.shape||[])[0]))+' complete_matches_completeness:'+(((f.complete&&f.complete.value)==='true')===((f.completeness&&f.completeness.value)==='Complete'))].join(' | '));
}
// --- truncation fields anywhere
for(const n of ['c18','c16','c35']){
  const e=env(n); const hits=[]; walk(e,(v,p)=>{ if(p.endsWith('.truncated')||p.endsWith('.budget')||p.endsWith('.truncationReason')) hits.push(p+'='+JSON.stringify(v&&v.value!==undefined?v.value:v)); }, '');
  L.push(['I-7',n,'truncFields='+(hits.join(' ')||'NONE'),'prettyTail='+JSON.stringify((JSON.stringify(e).match(/\"pretty\":\"[^\"]{0,60}/g)||[]).slice(0,3).join(' / '))].join(' | '));
}
// --- plot / cancel extras
for(const n of ['c09','c10','c13','c12','c33']){
  const e=env(n); if(!e){continue;}
  L.push(['extra',n,'keys='+Object.keys(e).join(','),'plot='+(e.plot?'present':'ABSENT'),'cancellation='+JSON.stringify(e.cancellation||null),'partialVariables='+JSON.stringify((e.partialVariables||[]).map(v=>v.name)),'diagN='+((e.diagnostics||[]).length)].join(' | '));
}
console.log(L.join('\n'));
