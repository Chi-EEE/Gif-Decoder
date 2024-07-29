const { Decoder } = require('../index');
const { writeFileSync, existsSync, mkdirSync, rmSync, readdirSync } = require('fs');
const { createCanvas } = require('@napi-rs/canvas');
const previewDirectory = './__visual_preview__'

// Create preview
if (existsSync(previewDirectory)) {
    rmSync(previewDirectory, { recursive: true, force: true })
}
mkdirSync(previewDirectory);

let one_file = false;
if (one_file) {
    const gif = Decoder.decodePath('./gifs/forsenDisco.gif')
    let buffers = gif.decodeFrames({
        implementDisposalPrevious: true,
        storeCache: true,
        disableDisposalMethods: false,
    })

    const canvas = createCanvas(gif.lsd.width, gif.lsd.height)
    const ctx = canvas.getContext("2d");

    for (let i = 0; i < buffers.length; i++) {
        const buffer = buffers[i]
        const image = ctx.createImageData(gif.lsd.width, gif.lsd.height)
        image.data.set(buffer)
        ctx.putImageData(image, 0, 0)
        const canvasBuffer = canvas.toBuffer('image/png')
        writeFileSync(`${previewDirectory}/${i}.png`, canvasBuffer)
    }
}
else {
    for (const file of readdirSync('./gifs')) {
        try {
            console.log(`Creating preview for ${file}`)

            const gif = Decoder.decodePath(`./gifs/${file}`)
            let buffers = gif.decodeFrames({
                implementDisposalPrevious: true,
                storeCache: true,
                disableDisposalMethods: false,
            })

            const canvas = createCanvas(gif.lsd.width, gif.lsd.height)
            const ctx = canvas.getContext("2d");

            mkdirSync(`${previewDirectory}/${file}`)
            for (let i = 0; i < buffers.length; i++) {
                const buffer = buffers[i]
                const image = ctx.createImageData(gif.lsd.width, gif.lsd.height)
                image.data.set(buffer)
                ctx.putImageData(image, 0, 0)
                const canvasBuffer = canvas.toBuffer('image/png')
                writeFileSync(`${previewDirectory}/${file}/${i}.png`, canvasBuffer)
            }
        } catch (error) {
            console.error(`Error while creating preview for ${file}`)
            console.error(error)
        }
    }
}