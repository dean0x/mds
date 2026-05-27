#!/usr/bin/env node
// Writes dist-cjs/package.json to mark the CJS output directory as CommonJS.
// Called as the final step of the dual-build (ESM + CJS) for packages that
// ship both formats. Usage: node scripts/write-cjs-package.cjs <output-dir>
'use strict';

const fs = require('fs');
const path = require('path');

const outDir = process.argv[2] || 'dist-cjs';
fs.mkdirSync(outDir, { recursive: true });
const dest = path.join(outDir, 'package.json');
fs.writeFileSync(dest, '{"type":"commonjs"}\n');
