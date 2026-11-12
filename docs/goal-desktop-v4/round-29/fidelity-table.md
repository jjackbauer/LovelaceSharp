| Document | Provenance | SVG bytes | Elements (rect/line/text/polyline/path/g/clipPath) | Product px | Edge px | Diff > 16 | Diff > 64 | Mean abs | Ink uncovered | Verdict |
|---|---|---:|---|---:|---:|---:|---:|---:|---:|---|
| ref-plot | live C# Studio reference (REF-plot.json), 15,000-byte response | 15000 | 2/13/11/1/0/0/0 | 800x600 | 800x600 | 0.2342% (1124) | 0.1254% | 0.1405 | 0.0000% | PASS |
| engine-square | Rust engine plot(): titled, strictly positive series | 15098 | 2/13/12/1/0/0/0 | 800x600 | 800x600 | 0.3256% (1563) | 0.1798% | 0.1918 | 0.0000% | PASS |
| engine-cubic | Rust engine plot(): untitled, negative/positive axes crossing at zero | 14711 | 2/12/10/1/0/0/0 | 800x600 | 800x600 | 0.2448% (1175) | 0.1435% | 0.1428 | 0.0000% | PASS |
| feature-coverage | harness-derived full-feature document; embeds the engine-square polyline | 13484 | 3/0/2/1/1/2/1 | 800x600 | 800x600 | 0.4090% (1963) | 0.1283% | 0.1518 | 0.0000% | PASS |
