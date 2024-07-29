import test from 'ava'

import { readFileSync } from 'fs'
import { Decoder } from '../index'

test('sample_1.gif: Version is 89a', (t) => {
  const gif = Decoder.decodePath('./gifs/sample_1.gif')
  t.is(gif.version, '89a')
})

const gif_test_cases = [
  { file: './gifs/sample_1.gif', expected: 1 },
  { file: './gifs/sample_2_animation.gif', expected: 3 },
  { file: './gifs/clap.gif', expected: 2 },
  { file: './gifs/NOIDONTTHINKSO.gif', expected: 59 },
  { file: './gifs/pepeMeltdown.gif', expected: 10 },
  { file: './gifs/monkaX.gif', expected: 6 },
  { file: './gifs/TeaTime.gif', expected: 61 },
  { file: './gifs/forsenDisco.gif', expected: 24 },
  { file: './gifs/forsenParty.gif', expected: 60 },
  { file: './gifs/forsenEnter.gif', expected: 34 },
  { file: './gifs/shadowchanHeart.0', expected: 23 },
  { file: './gifs/BBoomer.gif', expected: 13 }, // Interlace Gif
  { file: './gifs/YESITHINKSO.gif', expected: 162 }, // 64 bit datum gif
]

test('Correct Frame Count', (t) => {
  for (let gif_test_case of gif_test_cases) {
    let gif = Decoder.decodePath(gif_test_case.file)
    t.is(gif.frames.length, gif_test_case.expected)
  }
})


function decodeGifUsingBuffer(gif_file: string) {
  let gif_buffer = readFileSync(gif_file)
  let gif = Decoder.decodeBuffer(gif_buffer)
  return gif
}

test('Correct Frame Count using Buffer', (t) => {
  for (let gif_test_case of gif_test_cases) {
    let gif = decodeGifUsingBuffer(gif_test_case.file)
    t.is(gif.frames.length, gif_test_case.expected)
  }
})

const gif_test_cases_2 = [
  { file: './gifs/sample_2_animation.gif', expected: 1276 },
  { file: './gifs/clap.gif', expected: 3136 },
  { file: './gifs/NOIDONTTHINKSO.gif', expected: 3136 },
  { file: './gifs/pepeMeltdown.gif', expected: 50176 },
  { file: './gifs/monkaX.gif', expected: 3136 },
  { file: './gifs/TeaTime.gif', expected: 3136 },
  { file: './gifs/forsenDisco.gif', expected: 50176 },
  { file: './gifs/forsenParty.gif', expected: 3136 },
  { file: './gifs/forsenEnter.gif', expected: 3136 },
  { file: './gifs/shadowchanHeart.0', expected: 3136 },
  { file: './gifs/BBoomer.gif', expected: 3136 }, // Interlace Gif
  { file: './gifs/YESITHINKSO.gif', expected: 3136 }, // 64 bit datum gif
]

test('Decoding Individual Frames', (t) => {
  for (let gif_test_case of gif_test_cases_2) {
    let gif = Decoder.decodePath(gif_test_case.file)
    for (let i = 0; i < gif.frames.length; i++) {
      t.is(gif.decodeFrame(i).length, gif_test_case.expected)
    }
  }
})

test('sample_2_animation.gif: Frame 3: Top is 2', (t) => {
  const gif = Decoder.decodePath('./gifs/sample_2_animation.gif')
  t.is(gif.frames[2].im.top, 2)
})

test('Dancing.gif: Global Table Length is 256', (t) => {
  const gif = Decoder.decodePath('./gifs/Dancing.gif')
  t.is(gif.globalTable.length, 256)
})
