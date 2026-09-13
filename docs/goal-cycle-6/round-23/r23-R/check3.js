
const fs=require('fs'),path=require('path');const D=process.argv[2],raw=path.join(D,'raw');
const env=n=>{try{return JSON.parse(fs.readFileSync(path.join(raw,n+'.out'),'utf8').replace(/[\r\n]+$/,''));}catch(e){return null;}};
for(const n of ['x1','x2','x3','x4','c12','c13']){const e=env(n);if(!e){console.log(n+' none');continue;}
 const c=e.cancellation||{};console.log(n+' rc='+fs.readFileSync(path.join(raw,n+'.rc'),'utf8').trim()+' budgetMs='+c.budgetMs+' elapsedMs='+c.elapsedMs+' excessMs='+c.excessMs+' stopped='+c.stopped+' exceeded='+c.exceeded+' elapsedStr='+JSON.stringify(e.elapsed)+' elapsedTime='+JSON.stringify(e.elapsedTime)+' elapsedMs-budgetMs='+(c.elapsedMs-c.budgetMs).toFixed(3)+' DIFF='+((c.excessMs-(c.elapsedMs-c.budgetMs))).toFixed(3));}
const c37=env('c37b');console.log('c37b rc='+fs.readFileSync(path.join(raw,'c37b.rc'),'utf8').trim()+' '+JSON.stringify({code:c37.code,category:c37.category,recoverable:c37.recoverable,message:c37.message}));
const c22=env('c22');console.log('c22 '+JSON.stringify({code:c22.code,category:c22.category,recoverable:c22.recoverable,message:c22.message}));
const c33=env('c33');console.log('c33 '+JSON.stringify({code:c33.code,category:c33.category,recoverable:c33.recoverable,message:c33.message}));
const c15=env('c15');console.log('c15 '+JSON.stringify({code:c15.code,category:c15.category,message:c15.message}));
