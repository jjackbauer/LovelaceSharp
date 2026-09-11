'use strict';
const fs = require('fs');
const path = require('path');
const before = 'docs/goal-cycle-3/round-5/fixtures-before';
const after = 'Lovelace.Run.Tests/fixtures';
function names(dir){ return fs.readdirSync(dir).filter(f => f.endsWith('.json')).sort(); }
const b = names(before), a = names(after);
let changed = 0;
for (const n of b){
  if (!a.includes(n)) { console.log('REMOVED ' + n); changed++; continue; }
  const x = fs.readFileSync(path.join(before, n), 'utf8');
  const y = fs.readFileSync(path.join(after, n), 'utf8');
  if (x === y) { console.log('same    ' + n); continue; }
  changed++;
  console.log('CHANGED ' + n + '  (' + x.length + ' -> ' + y.length + ' bytes)');
  const xl = x.split('\n'), yl = y.split('\n');
  for (let i = 0, j = 0; i < Math.max(xl.length, yl.length); i++){
    if (xl[i] !== yl[i]) console.log('  - ' + JSON.stringify(xl[i]) + '\n  + ' + JSON.stringify(yl[i]));
  }
}
for (const n of a) if (!b.includes(n)) { console.log('ADDED   ' + n); changed++; }
console.log('changed/added files: ' + changed + ' of ' + a.length + ' goldens');