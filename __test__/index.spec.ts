import test from 'ava'

import { Decoder } from '../index'

test('sample_1.gif: Frame Count is 1', (t) => {
  const gif = Decoder.decode('./gifs/sample_1.gif')
  t.is(gif.frames.length, 1)
})

test('sample_1.gif: Version is 89a', (t) => {
  const gif = Decoder.decode('./gifs/sample_1.gif')
  t.is(gif.version, '89a')
})

test('sample_2_animation.gif: Frame Count is 3', (t) => {
  const gif = Decoder.decode('./gifs/sample_2_animation.gif')
  t.is(gif.frames.length, 3)
})

test('sample_2_animation.gif: Frame 3: Top is 2', (t) => {
  const gif = Decoder.decode('./gifs/sample_2_animation.gif')
  t.is(gif.frames[2].im.top, 2)
})

test('Dancing.gif: Global Table Length is 256', (t) => {
  const gif = Decoder.decode('./gifs/Dancing.gif')
  t.is(gif.globalTable.length, 256)
})

test('Dancing.gif: Frame 1: Local Color Table Flag is false', (t) => {
  const gif = Decoder.decode('./gifs/Dancing.gif')
  t.is(gif.frames[0].im.localColorTableFlag, false)
})
