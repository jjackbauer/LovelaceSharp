#!/usr/bin/env node
// Decode a DSH session log (concatenated zstd frames of JSONL) to plain JSONL on stdout.
import { readFileSync } from 'node:fs'
import { zstdDecompressSync } from 'node:zlib'

const file = process.argv[2]
const buf = readFileSync(file)
const magic = Buffer.from([0x28, 0xb5, 0x2f, 0xfd])
const offsets = []
let idx = 0
while ((idx = buf.indexOf(magic, idx)) !== -1) { offsets.push(idx); idx += 4 }
let text = ''
for (let i = 0; i < offsets.length; i++) {
  const slice = buf.subarray(offsets[i], i + 1 < offsets.length ? offsets[i + 1] : buf.length)
  try { text += zstdDecompressSync(slice).toString('utf8') } catch { /* skip a torn frame */ }
}
process.stdout.write(text)
