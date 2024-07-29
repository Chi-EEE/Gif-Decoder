# `@chi_eee/gif-decoder`

![https://github.com/HuuChiHuynh/Gif-Decoder/actions](https://github.com/HuuChiHuynh/Gif-Decoder/workflows/CI/badge.svg)
[![install size](https://packagephobia.com/badge?p=@chi_eee/gif-decoder)](https://packagephobia.com/result?p=@chi_eee/gif-decoder)
[![Downloads](https://img.shields.io/npm/dm/@chi_eee/gif-decoder.svg?sanitize=true)](https://npmcharts.com/compare/@chi_eee/gif-decoder?minimal=true)

# Install

```
npm i @chi_eee/gif-decoder
```

https://www.npmjs.com/package/@chi_eee/gif-decoder

# Usage

## Javascript:
```js
const { Decoder } = require('@chi_eee/gif-decoder')
const { readFileSync } = require('fs')

const gif = Decoder.decodePath('sample.gif')
const gif = Decoder.decodeBuffer(readFileSync('sample.gif'))
```

## Typescript:
```js
import { Decoder } from '@chi_eee/gif-decoder';
import { readFileSync } from 'fs'

const gif = Decoder.decodePath('sample.gif')
const gif = Decoder.decodeBuffer(readFileSync('sample.gif'))
```

# Credits

Spec: https://www.w3.org/Graphics/GIF/spec-gif89a.txt

Interlace Function: https://github.com/matt-way/gifuct-js

Gif Blog: https://www.matthewflickinger.com/lab/whatsinagif/index.html

LZW Decompression: https://gist.github.com/devunwired/4479231

Modern Gif: https://github.com/qq15725/modern-gif