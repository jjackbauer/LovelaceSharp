
const fs=require('fs'),path=require('path');
const dir=process.argv[2];
const raw=path.join(dir,'raw');
const rows=[];
for(const f of fs.readdirSync(raw).filter(x=>x.endsWith('.out')).sort()){
  const name=f.replace(/\.out$/,'');
  const b=fs.readFileSync(path.join(raw,f));
  const errB=fs.existsSync(path.join(raw,name+'.err'))?fs.readFileSync(path.join(raw,name+'.err')):Buffer.alloc(0);
  const rc=fs.readFileSync(path.join(raw,name+'.rc'),'utf8').trim();
  const s=b.toString('utf8');
  let parsed=null,parseErr=null,extraTail=null,trailingTerm=null;
  // try: whole buffer as JSON
  try{ JSON.parse(s); parsed='exact'; }catch(e){ parseErr=e.message; }
  let consumed=null;
  if(parsed!=='exact'){
    // strip a single trailing CRLF / LF / CR
    let t=s, term='';
    if(t.endsWith('\r\n')){term='CRLF';t=t.slice(0,-2);} else if(t.endsWith('\n')){term='LF';t=t.slice(0,-1);} else if(t.endsWith('\r')){term='CR';t=t.slice(0,-1);}
    try{ JSON.parse(t); parsed='single-terminator'; trailingTerm=term; consumed=b.length-Buffer.byteLength(term); }catch(e){ parseErr+=' | '+e.message; }
    if(parsed!=='single-terminator'){ parsed='FAIL'; }
  } else { trailingTerm='(none)'; }
  let env=null;
  try{ env=JSON.parse(s.replace(/[\r\n]+$/,'')); }catch(e){}
  const rec={name,bytes:b.length,errBytes:errB.length,rc,parse:parsed,trailingTerm,
    lines:s.split(/\r\n|\n|\r/).length-1,
    firstByte:b.length?b[0]:null,lastByte:b.length?b[b.length-1]:null,
    hexTail:b.length?b.slice(-8).toString('hex'):'',
    firstBytes:b.slice(0,24).toString('utf8')};
  if(env){
    rec.ok=env.ok; rec.code=env.code; rec.category=env.category; rec.revision=env.revision;
    rec.outputLen=Array.isArray(env.output)?env.output.length:(env.output===undefined?'ABSENT':typeof env.output);
    rec.partialLen=Array.isArray(env.partialOutput)?env.partialOutput.length:(env.partialOutput===undefined?'ABSENT':typeof env.partialOutput);
    rec.output=env.output&&env.output.length<=6?env.output:undefined;
    rec.partial=env.partialOutput&&env.partialOutput.length<=6?env.partialOutput:undefined;
    rec.timings=Array.isArray(env.timings)?env.timings.length:(env.timings===undefined?'ABSENT':typeof env.timings);
    rec.elapsed=env.elapsed; rec.elapsedTime=env.elapsedTime?JSON.stringify(env.elapsedTime):'ABSENT';
    rec.keys=Object.keys(env).join(',');
    rec.hasResult=env.result!==undefined;
    rec.diagN=Array.isArray(env.diagnostics)?env.diagnostics.length:(env.diagnostics===undefined?'ABSENT':typeof env.diagnostics);
  }
  rows.push(rec);
}
for(const r of rows) console.log(JSON.stringify(r));
