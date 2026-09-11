'use strict';
const fs = require('fs');
const path = require('path');
const VOLATILE = new Set(['revision','elapsed','elapsedTime','timings']);
function normalise(v){
  if (Array.isArray(v)) return v.map(normalise);
  if (v && typeof v === 'object'){ const o={}; for(const k of Object.keys(v)) o[k]=VOLATILE.has(k)?'<volatile>':normalise(v[k]); return o; }
  return v;
}
const rawDir = path.resolve('out/raw-r5');
const outDir = path.resolve('Lovelace.Run.Tests/fixtures');
const names = fs.readdirSync(outDir).filter(f => f.endsWith('.ls')).map(f => f.slice(0, -3)).sort();
for (const n of names){
  const env = JSON.parse(fs.readFileSync(path.join(rawDir, n + '.json'), 'utf8'));
  const text = JSON.stringify(normalise(env), null, 2) + '\n';
  fs.writeFileSync(path.join(outDir, n + '.json'), text, 'utf8');
  console.log('wrote ' + n + '.json (' + text.length + ' bytes)');
}