const fs=require('fs');
function show(p,label){
  const env=JSON.parse(fs.readFileSync(p,'utf8'));
  console.log('===== '+label+' =====');
  console.log('ok='+env.ok+' protocolVersion='+env.protocolVersion+' symbolicFormatVersion='+JSON.stringify(env.symbolicFormatVersion)+' mathIrVersion='+env.mathIrVersion);
  const s=env.result.structured;
  console.log('type='+s.type);
  const fields={}; for(const f of s.fields) fields[f.name]=f.value;
  console.log('status='+JSON.stringify(fields.status));
  console.log('diagnostics='+JSON.stringify(fields.diagnostics,null,1));
  console.log('display tail: ...'+env.result.display.slice(-160));
}
show('C:/Users/ricar/dev/LovelaceSharp/out/aot-solve.json','published binary: x = symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x)');
show('C:/Users/ricar/dev/LovelaceSharp/out/aot-limit.json','published binary: x = symbol("x"); limit_full(sin(x)/x, x, 0)');
