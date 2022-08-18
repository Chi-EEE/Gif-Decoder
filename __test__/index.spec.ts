import test from 'ava'

import { Decoder } from '../index'

test('sample_1.gif: Version is 89a', (t) => {
  const gif = Decoder.decode('./gifs/sample_1.gif')
  t.is(gif.version, '89a')
})

test('Correct Frame Count', (t) => {
  let gif = Decoder.decode('./gifs/sample_1.gif')
  t.is(gif.frames.length, 1)

  gif = Decoder.decode('./gifs/sample_2_animation.gif')
  t.is(gif.frames.length, 3)

  gif = Decoder.decode('./gifs/clap.gif')
  t.is(gif.frames.length, 2)

  gif = Decoder.decode('./gifs/NOIDONTTHINKSO.gif')
  t.is(gif.frames.length, 59)

  gif = Decoder.decode('./gifs/pepeMeltdown.gif')
  t.is(gif.frames.length, 10)

  gif = Decoder.decode('./gifs/monkaX.gif')
  t.is(gif.frames.length, 6)

  gif = Decoder.decode('./gifs/TeaTime.gif')
  t.is(gif.frames.length, 61)

  gif = Decoder.decode('./gifs/forsenDisco.gif')
  t.is(gif.frames.length, 60)

  gif = Decoder.decode('./gifs/forsenEnter.gif')
  t.is(gif.frames.length, 34)

  gif = Decoder.decode('./gifs/shadowchanHeart.0')
  t.is(gif.frames.length, 23)

  gif = Decoder.decode('./gifs/BBoomer.gif') // Interlace Gif
  t.is(gif.frames.length, 13)

  gif = Decoder.decode('./gifs/YESITHINKSO.gif') // 64 bit datum gif
  t.is(gif.frames.length, 162)
})

test('Decoding Individual Frames', (t) => {
  let gif = Decoder.decode('./gifs/sample_2_animation.gif')
  t.is(gif.decodeFrame(gif.frames[0]).length, 1276)

  gif = Decoder.decode('./gifs/clap.gif')
  t.is(gif.decodeFrame(gif.frames[1]).length, 3136)

  gif = Decoder.decode('./gifs/NOIDONTTHINKSO.gif')
  t.is(gif.decodeFrame(gif.frames[58]).length, 3136)

  gif = Decoder.decode('./gifs/pepeMeltdown.gif')
  t.is(gif.decodeFrame(gif.frames[9]).length, 50176)

  gif = Decoder.decode('./gifs/monkaX.gif')
  t.is(gif.decodeFrame(gif.frames[5]).length, 3136)

  gif = Decoder.decode('./gifs/TeaTime.gif')
  t.is(gif.decodeFrame(gif.frames[60]).length, 3136)

  gif = Decoder.decode('./gifs/forsenDisco.gif')
  t.is(gif.decodeFrame(gif.frames[59]).length, 3136)

  gif = Decoder.decode('./gifs/forsenEnter.gif')
  t.is(gif.decodeFrame(gif.frames[33]).length, 3136)

  gif = Decoder.decode('./gifs/shadowchanHeart.0') // 0 file gif
  t.is(gif.decodeFrame(gif.frames[0]).length, 3136)

  gif = Decoder.decode('./gifs/BBoomer.gif') // Interlace Gif
  t.is(gif.decodeFrame(gif.frames[0]).length, 3136)

  gif = Decoder.decode('./gifs/YESITHINKSO.gif') // 64 bit datum gif
  t.is(gif.decodeFrame(gif.frames[0]).length, 3136)
})

test('sample_2_animation.gif: Frame 3: Top is 2', (t) => {
  const gif = Decoder.decode('./gifs/sample_2_animation.gif')
  t.is(gif.frames[2].im.top, 2)
})

test('Dancing.gif: Global Table Length is 256', (t) => {
  const gif = Decoder.decode('./gifs/Dancing.gif')
  t.is(gif.globalTable.length, 256)
})
