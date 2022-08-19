const { Decoder } = require('../index');
const { writeFileSync, existsSync, mkdirSync, rmSync } = require('fs');
const { createCanvas } = require('@napi-rs/canvas');
const previewDirectory = './__visual_preview__'

// Create preview
if (existsSync(previewDirectory)) {
    rmSync(previewDirectory, { recursive: true, force: true })
}
mkdirSync(previewDirectory);

const gif = Decoder.decode('./gifs/BBoomer.gif')
let buffers = gif.decodeFrames()

const canvas = createCanvas(gif.lsd.width, gif.lsd.height)
const ctx = canvas.getContext("2d");

for (let i = 0; i < buffers.length; i++) {
    const frame = gif.frames[i]
    const buffer = buffers[i]
    console.log(frame.im.width, frame.im.height)
    const image = ctx.createImageData(frame.im.width, frame.im.height)
    image.data.set(buffer)
    ctx.putImageData(image, frame.im.left, frame.im.top)
    const canvasBuffer = canvas.toBuffer('image/png')
    writeFileSync(`${previewDirectory}/${i + 1}.png`, canvasBuffer)
}