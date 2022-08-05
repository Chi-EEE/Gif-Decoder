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
})

test('Decoding Individual Frames', (t) => {
  let gif = Decoder.decode('./gifs/sample_2_animation.gif')
  t.notDeepEqual(gif.frames[2].decode(), null)

  gif = Decoder.decode('./gifs/clap.gif')
  t.notDeepEqual(gif.frames[1].decode(), null)

  gif = Decoder.decode('./gifs/NOIDONTTHINKSO.gif')
  t.notDeepEqual(gif.frames[58].decode(), null)

  gif = Decoder.decode('./gifs/pepeMeltdown.gif')
  t.notDeepEqual(gif.frames[9].decode(), null)

  gif = Decoder.decode('./gifs/monkaX.gif')
  t.notDeepEqual(gif.frames[5].decode(), null)

  gif = Decoder.decode('./gifs/TeaTime.gif')
  t.notDeepEqual(gif.frames[60].decode(), null)

  gif = Decoder.decode('./gifs/forsenDisco.gif')
  t.notDeepEqual(gif.frames[59].decode(), null)

  gif = Decoder.decode('./gifs/forsenEnter.gif')
  t.notDeepEqual(gif.frames[33].decode(), null)
})

test('sample_2_animation.gif: Frame 3: Top is 2', (t) => {
  const gif = Decoder.decode('./gifs/sample_2_animation.gif')
  t.is(gif.frames[2].im.top, 2)
})

test('Dancing.gif: Global Table Length is 256', (t) => {
  const gif = Decoder.decode('./gifs/Dancing.gif')
  t.is(gif.globalTable.length, 256)
})
