/* tslint:disable */
/* eslint-disable */

/* auto-generated by NAPI-RS */

export interface LogicalScreenDescriptor {
  width: number
  height: number
  globalColorFlag: boolean
  colorResolution: number
  sortedFlag: boolean
  globalColorSize: number
  backgroundColorIndex: number
  pixelAspectRatio: number
}
export interface ImageDescriptor {
  left: number
  top: number
  width: number
  height: number
  interlaceFlag: boolean
  sortFlag: boolean
}
export interface GraphicsControlExtension {
  disposalMethod: number
  userInputFlag: boolean
  transparentColorFlag: boolean
  delayTime: number
  transparentColorIndex: number
}
export interface Color {
  red: number
  green: number
  blue: number
}
export class Gif {
  version: string
  lsd: LogicalScreenDescriptor
  globalTable: Array<Color>
  frames: Array<Frame>
  decodeFrames(): Array<Buffer>
}
export class Frame {
  gcd: GraphicsControlExtension
  im: ImageDescriptor
  colorTable: Array<Color>
  indexStream: Array<number>
  decode(): Buffer
}
/** */
export class Decoder {
  static decode(filePath: string): Gif
}
