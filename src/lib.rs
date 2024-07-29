#![deny(clippy::all)]

use napi::bindgen_prelude::{Buffer, FromNapiValue};
use napi::{Error, Result};
use napi_derive::napi;

use byteorder::{ByteOrder, LittleEndian};
use std::ops::IndexMut;

use derivative::Derivative;

const MAX_STACK_SIZE: u16 = 4096;

const DISPOSAL_UNSPECIFIED: u32 = 0;
const DISPOSAL_NONE: u32 = 1;
const DISPOSAL_BACKGROUND: u32 = 2;
const DISPOSAL_PREVIOUS: u32 = 3;

fn shl_or(val: u32, shift: usize, def: u32) -> u32 {
  [val << (shift & 31), def][((shift & !31) != 0) as usize]
}
fn shr_or(val: u32, shift: usize, def: u32) -> u32 {
  [val >> (shift & 31), def][((shift & !31) != 0) as usize]
}

#[derive(Default, Debug, Clone)]
#[napi(js_name = "Gif")]
struct Gif {
  pub version: String,
  pub lsd: LogicalScreenDescriptor,
  pub global_table: Vec<Color>,
  pub frames: Vec<Frame>,
}

#[derive(Debug, Clone)]
#[napi(object)]
struct DecoderOptions {
  /// Whether to implement the disposal method of the previous frame, default is `true`
  pub implement_disposal_previous: bool,
  /// Whether to store the cache of the frame, should be `false` when enabling disableDisposalMethods | rawDecode, default is `true`
  pub store_cache: bool,
  /// Whether to disable the use of any disposal methods, default is `false`
  pub disable_disposal_methods: bool,
  /// Whether to return the raw decoded frame, also disables the use of any disposal methods, default is `false`
  pub raw_decode: bool,
}

impl Default for DecoderOptions {
  fn default() -> DecoderOptions {
    DecoderOptions {
      implement_disposal_previous: true,
      store_cache: true,
      disable_disposal_methods: false,
      raw_decode: false,
    }
  }
}

#[napi]
impl Gif {
  #[napi]
  pub fn decode_frames(&mut self, decoder_options: DecoderOptions) -> Vec<Buffer> {
    let mut buffers: Vec<Buffer> = Vec::new();

    let has_disposal_3 = self
      .frames
      .iter()
      .any(|frame| frame.gcd.disposal_method == DISPOSAL_PREVIOUS);

    let mut previous_pixels: Buffer =
      Buffer::from(vec![0; (self.lsd.width * self.lsd.height) as usize * 4]);

    let mut maybe_previous_frame_index: Option<usize> = None;
    let mut previous_disposal_method = 0;
    for i in 0..self.frames.len() {
      let buffer = match &self.frames[i].cached_frame {
        Some(cached_frame) => cached_frame.to_owned(),
        None => self.decode_frame_internal(
          i,
          &decoder_options,
          maybe_previous_frame_index,
          &previous_disposal_method,
          &has_disposal_3,
          Some(&mut previous_pixels),
        ),
      };
      buffers.push(buffer);
      maybe_previous_frame_index = Some(i);
      previous_disposal_method = self.frames[i].gcd.disposal_method;
    }
    return buffers;
  }

  #[napi]
  pub fn decode_frame(
    &mut self,
    frame_index: u32,
    decoder_options: DecoderOptions,
  ) -> Result<Buffer> {
    if frame_index >= self.frames.len() as u32 {
      return Err(Error::from_reason("Frame index out of bounds".to_string()));
    }

    if let Some(cached_frame) = &self.frames[frame_index as usize].cached_frame {
      return Ok(cached_frame.to_owned());
    }

    let has_disposal_3 = self
      .frames
      .iter()
      .any(|frame| frame.gcd.disposal_method == DISPOSAL_PREVIOUS);

    let maybe_previous_frame_index: Option<usize> = match frame_index.checked_sub(1) {
      Some(previous_frame_index) => Some(previous_frame_index as usize),
      None => None,
    };

    let previous_disposal_method = match maybe_previous_frame_index {
      Some(previous_frame_index) => self.frames[previous_frame_index].gcd.disposal_method,
      None => 0,
    };

    Ok(self.decode_frame_internal(
      frame_index as usize,
      &decoder_options,
      maybe_previous_frame_index,
      &previous_disposal_method,
      &has_disposal_3,
      None,
    ))
  }

  fn decode_frame_internal(
    &mut self,
    frame_index: usize,
    decoder_options: &DecoderOptions,
    maybe_previous_frame_index: Option<usize>,
    previous_disposal_method: &u32,
    has_disposal_3: &bool,
    maybe_previous_pixels: Option<&mut Buffer>,
  ) -> Buffer {
    let mut buffer: Vec<u8> = Vec::new();

    if !decoder_options.raw_decode {
      if !decoder_options.disable_disposal_methods {
        if decoder_options.implement_disposal_previous
          && previous_disposal_method == &DISPOSAL_PREVIOUS
        {
          if maybe_previous_pixels.is_some() {
            buffer = (*(maybe_previous_pixels.as_ref()).unwrap()).to_vec();
          } else {
            match maybe_previous_frame_index {
              None => {
                self.fill_with_empty_color(&mut buffer);
              }
              Some(previous_frame_index) => {
                let previous_frame = &self.frames[previous_frame_index];
                match &previous_frame.previous_pixels {
                  Some(previous_pixels) => buffer = previous_pixels.to_owned().into(),
                  None => {
                    let mut maybe_pp_frame_index: Option<usize> =
                      match previous_frame_index.checked_sub(1) {
                        Some(pp_frame_index) => Some(pp_frame_index),
                        None => None,
                      };
                    loop {
                      if let Some(pp_frame_index) = maybe_pp_frame_index {
                        match &self.frames[pp_frame_index].previous_pixels {
                          Some(previous_pixels) => {
                            buffer = previous_pixels.to_owned().into();
                            break;
                          }
                          None => {
                            maybe_pp_frame_index = match pp_frame_index.checked_sub(1) {
                              Some(pp_frame_index) => Some(pp_frame_index),
                              None => None,
                            };
                          }
                        }
                      } else {
                        self.fill_with_empty_color(&mut buffer);
                        break;
                      }
                    }
                  }
                }
              }
            }
          }
        } else if previous_disposal_method == &DISPOSAL_BACKGROUND {
          self.fill_with_empty_color(&mut buffer);
          let background_color = match self
            .global_table
            .get(self.lsd.background_color_index as usize)
          {
            Some(color) => color,
            None => &Color {
              red: 0,
              green: 0,
              blue: 0,
            },
          };

          let transparent_color_index = self.frames[frame_index].gcd.transparent_color_index;

          let is_overflow_transparent_index =
            transparent_color_index >= self.frames[frame_index].color_table.len() as u32;

          let mut is_bg_transparent = false;
          if self.frames[frame_index].gcd.transparent_color_flag {
            is_bg_transparent = (transparent_color_index == self.lsd.background_color_index)
              || (is_overflow_transparent_index && self.lsd.background_color_index == 0);
          }

          let top = self.frames[frame_index].im.top;
          let left = self.frames[frame_index].im.left;
          let bottom = self.frames[frame_index].im.top + self.frames[frame_index].im.height;
          let right = self.frames[frame_index].im.left + self.frames[frame_index].im.width;
          if is_bg_transparent {
            for y in top..bottom {
              for x in left..right {
                let buffer_index = ((((y) * self.lsd.width) + (x)) * 4) as usize;
                buffer[buffer_index] = background_color.red.try_into().unwrap();
                buffer[buffer_index + 1] = background_color.green.try_into().unwrap();
                buffer[buffer_index + 2] = background_color.blue.try_into().unwrap();
                buffer[buffer_index + 3] = 0;
              }
            }
          } else {
            for y in top..bottom {
              for x in left..right {
                let buffer_index = ((((y) * self.lsd.width) + (x)) * 4) as usize;
                buffer[buffer_index] = background_color.red.try_into().unwrap();
                buffer[buffer_index + 1] = background_color.green.try_into().unwrap();
                buffer[buffer_index + 2] = background_color.blue.try_into().unwrap();
                buffer[buffer_index + 3] = 255;
              }
            }
          }
        } else {
          match maybe_previous_frame_index {
            None => {
              self.fill_with_empty_color(&mut buffer);
            }
            Some(previous_frame_index) => {
              let maybe_pp_frame_index: Option<usize> = match previous_frame_index.checked_sub(1) {
                Some(pp_frame_index) => Some(pp_frame_index),
                None => None,
              };
              let mut previous_disposal_method = match maybe_pp_frame_index {
                Some(pp_frame_index) => self.frames[pp_frame_index].gcd.disposal_method,
                None => 0,
              };

              buffer = match &self.frames[previous_frame_index].cached_frame {
                Some(cached_frame) => cached_frame.to_owned().into(),
                None => {
                  let buffer = self.decode_frame_internal(
                    previous_frame_index,
                    &decoder_options,
                    maybe_pp_frame_index,
                    &mut previous_disposal_method,
                    &has_disposal_3,
                    None,
                  );
                  buffer.into()
                }
              };
            }
          }
        }
      } else {
        self.fill_with_empty_color(&mut buffer);
      }
    }

    let frame = self.frames.get_mut(frame_index).unwrap();

    let top = frame.im.top;
    let left = frame.im.left;
    let bottom = frame.im.top + frame.im.height;
    let right = frame.im.left + frame.im.width;

    let mut index = 0;
    if decoder_options.raw_decode {
      for _y in top..bottom {
        for _x in left..right {
          match frame.index_stream.get(index) {
            Some(color_index) => match frame.color_table.get(*color_index as usize) {
              Some(color) => {
                buffer.push(color.red.try_into().unwrap());
                buffer.push(color.green.try_into().unwrap());
                buffer.push(color.blue.try_into().unwrap());
                let is_transparent_index = frame.gcd.transparent_color_flag
                  && color_index == (&frame.gcd.transparent_color_index.try_into().unwrap());
                buffer.push(if is_transparent_index { 0 } else { 255 })
              }
              None => {
                for _ in 0..4 {
                  buffer.push(0);
                }
              }
            },
            None => {
              for _ in 0..4 {
                buffer.push(0);
              }
            }
          }
          index += 1;
        }
      }
    } else {
      for y in top..bottom {
        for x in left..right {
          let buffer_index = ((((y) * self.lsd.width) + (x)) * 4) as usize;
          match frame.index_stream.get(index) {
            Some(color_index) => match frame.color_table.get(*color_index as usize) {
              Some(color) => {
                if *previous_disposal_method == DISPOSAL_UNSPECIFIED
                  || *previous_disposal_method == DISPOSAL_NONE
                {
                  let is_transparent_index = frame.gcd.transparent_color_flag
                    && color_index == (&frame.gcd.transparent_color_index.try_into().unwrap());
                  if !is_transparent_index {
                    buffer[buffer_index] = color.red.try_into().unwrap();
                    buffer[buffer_index + 1] = color.green.try_into().unwrap();
                    buffer[buffer_index + 2] = color.blue.try_into().unwrap();
                    buffer[buffer_index + 3] = 255;
                  }
                } else {
                  buffer[buffer_index] = color.red.try_into().unwrap();
                  buffer[buffer_index + 1] = color.green.try_into().unwrap();
                  buffer[buffer_index + 2] = color.blue.try_into().unwrap();
                  let is_transparent_index = frame.gcd.transparent_color_flag
                    && color_index == (&frame.gcd.transparent_color_index.try_into().unwrap());
                  buffer[buffer_index + 3] = if is_transparent_index { 0 } else { 255 }
                }
              }
              None => {}
            },
            None => {}
          }
          index += 1;
        }
      }
    }
    let buffer = Buffer::from(buffer);

    if decoder_options.store_cache {
      frame.cached_frame = Some(buffer.clone());
    }

    if !decoder_options.raw_decode
      && !decoder_options.disable_disposal_methods
      && decoder_options.implement_disposal_previous
    {
      let disposal_method = frame.gcd.disposal_method;
      if *has_disposal_3
        && disposal_method != DISPOSAL_UNSPECIFIED
        && disposal_method != DISPOSAL_PREVIOUS
      {
        if let Some(previous_pixels) = maybe_previous_pixels {
          *previous_pixels = buffer.clone();
        } else {
          frame.previous_pixels = Some(buffer.clone());
        }
      }
    }
    buffer
  }

  fn fill_with_empty_color(&self, buffer: &mut Vec<u8>) {
    for _ in 0..(self.lsd.width * self.lsd.height) {
      buffer.push(0);
      buffer.push(0);
      buffer.push(0);
      buffer.push(0);
    }
  }
}

#[derive(Default, Debug, Clone)]
#[napi(object)]
pub struct LogicalScreenDescriptor {
  pub width: u32,
  pub height: u32,
  pub global_color_flag: bool,
  pub color_resolution: u32,
  pub sorted_flag: bool,
  pub global_color_size: u32,
  pub background_color_index: u32,
  pub pixel_aspect_ratio: u32,
}

#[derive(Derivative, Default, Clone)]
#[derivative(Debug)]
#[napi(js_name = "Frame")]
pub struct Frame {
  pub gcd: GraphicsControlExtension,
  pub im: ImageDescriptor,
  pub color_table: Vec<Color>,
  pub index_stream: Vec<u8>,
  /// Generated when decoding the frame, used to properly implement disposal method 0 | 1, can be disabled using DecoderOptions.storeCache
  #[derivative(Debug = "ignore")]
  pub cached_frame: Option<Buffer>,
  /// Generated when decoding the frame if any disposal method is 3 present in the Gif, used when decoding individual frames, can be disabled using DecoderOptions.implementDisposalPrevious
  #[derivative(Debug = "ignore")]
  pub previous_pixels: Option<Buffer>,
}

impl FromNapiValue for Frame {
  fn from_unknown(value: napi::JsUnknown) -> Result<Self> {
    Ok(Self {
      gcd: GraphicsControlExtension::default(),
      im: ImageDescriptor::default(),
      color_table: Vec::default(),
      index_stream: Vec::default(),
      cached_frame: None,
      previous_pixels: None,
    })
  }

  unsafe fn from_napi_value(
    env: napi::sys::napi_env,
    napi_val: napi::sys::napi_value,
  ) -> Result<Self> {
    Ok(Self {
      gcd: GraphicsControlExtension::default(),
      im: ImageDescriptor::default(),
      color_table: Vec::default(),
      index_stream: Vec::default(),
      cached_frame: None,
      previous_pixels: None,
    })
  }
}

#[derive(Default, Debug, Clone)]
#[napi(object)]
pub struct ImageDescriptor {
  pub left: u32,
  pub top: u32,
  pub width: u32,
  pub height: u32,
  pub interlace_flag: bool,
  pub sort_flag: bool,
}

#[derive(Default, Debug, Clone)]
#[napi(object)]
pub struct GraphicsControlExtension {
  pub disposal_method: u32,
  pub user_input_flag: bool,
  pub transparent_color_flag: bool,
  pub delay_time: u32,
  pub transparent_color_index: u32,
}

#[derive(Default, Debug, Clone)]
#[napi(object)]
pub struct Color {
  pub red: u32,
  pub green: u32,
  pub blue: u32,
}

#[napi(js_name = "Decoder")]
struct Decoder {}

#[napi]
impl Decoder {
  #[napi]
  pub fn decode_path(file_path: String) -> Result<Gif> {
    let contents = match std::fs::read(&file_path) {
      Ok(contents) => contents,
      Err(err) => return Err(Error::from_reason(err.to_string())),
    };
    let contents = contents.as_slice();
    return Self::decode_internal(contents);
  }

  #[napi]
  pub fn decode_buffer(buffer: Buffer) -> Result<Gif> {
    let contents: Vec<u8> = buffer.into();
    let contents = contents.as_slice();
    return Self::decode_internal(contents);
  }

  fn decode_internal(contents: &[u8]) -> Result<Gif> {
    let signature = match contents.get(0..3) {
      Some(signature_bytes) => match String::from_utf8(signature_bytes.to_vec()) {
        Ok(parsed_signature) => parsed_signature,
        Err(err) => return Err(Error::from_reason(err.to_string())),
      },
      None => {
        return Err(Error::from_reason(
          "Unable to get file signature, the file is corrupted".to_string(),
        ))
      }
    };
    if signature != "GIF" {
      return Err(Error::from_reason(format!(
        "Invalid file signature, got {}",
        signature
      )));
    }

    let mut gif = Gif::default();
    match contents.get(3..6) {
      Some(version_bytes) => match String::from_utf8(version_bytes.to_vec()) {
        Ok(parsed_version) => {
          gif.version = parsed_version;
        }
        Err(err) => return Err(Error::from_reason(err.to_string())),
      },
      None => {
        return Err(Error::from_reason(
          "Unable to get file's version, the file is corrupted".to_string(),
        ))
      }
    }

    match Self::handle_logical_screen_descriptor(&mut gif, contents) {
      Ok(_) => {}
      Err(error) => return Err(error),
    }

    let mut offset: usize = 13;

    // Global Color Table
    let length: usize = 3 * 2 << gif.lsd.global_color_size;
    let mut i: usize = offset;

    if gif.lsd.global_color_flag {
      let mut global_color_vector: Vec<Color> = Vec::new();

      while i < offset + length {
        let red;
        let green;
        let blue;
        match contents.get(i) {
          Some(red_byte) => {
            red = *red_byte;
          }
          None => {
            return Err(Error::from_reason(
              "Unable to get red in handle_logical_screen_descriptor, the file is corrupted"
                .to_string(),
            ))
          }
        };
        match contents.get(i + 1) {
          Some(green_byte) => {
            green = *green_byte;
          }
          None => {
            return Err(Error::from_reason(
              "Unable to get green in handle_logical_screen_descriptor, the file is corrupted"
                .to_string(),
            ))
          }
        };
        match contents.get(i + 2) {
          Some(blue_byte) => {
            blue = *blue_byte;
          }
          None => {
            return Err(Error::from_reason(
              "Unable to get blue in handle_logical_screen_descriptor, the file is corrupted"
                .to_string(),
            ))
          }
        };
        global_color_vector.push(Color {
          red: (red as u32),
          green: (green as u32),
          blue: (blue as u32),
        });
        i = i + 3;
      }
      Self::increment_offset(&mut offset, length);
      gif.global_table = global_color_vector;
    }
    let mut done = false;
    loop {
      let introducer = match contents.get(offset) {
        Some(introducer) => introducer,
        None => {
          return Err(Error::from_reason(
            "Unable to get introducer, the file is corrupted".to_string(),
          ))
        }
      };
      Self::increment_offset(&mut offset, 1);
      match introducer {
        0x2C => {
          // Image Descriptor
          match Self::handle_image_descriptor(&mut offset, &mut gif, contents) {
            Ok(_) => {}
            Err(error) => return Err(error),
          };
        }
        0x21 => {
          let label = match contents.get(offset) {
            Some(introducer) => introducer,
            None => {
              return Err(Error::from_reason(
                "Unable to get label, the file is corrupted".to_string(),
              ))
            }
          };
          Self::increment_offset(&mut offset, 1);
          match label {
            0xF9 => {
              match Self::handle_graphic_control_extension(&mut offset, &mut gif, contents) {
                Ok(_) => {}
                Err(error) => return Err(error),
              };
            }
            0x01 => {
              match Self::handle_plain_text_extension(&mut offset, &mut gif, contents) {
                Ok(_) => {}
                Err(error) => return Err(error),
              };
            }
            0xFF => {
              match Self::handle_application_extension(&mut offset, &mut gif, contents) {
                Ok(_) => {}
                Err(error) => return Err(error),
              };
            }
            0xFE => {
              match Self::handle_comment_extension(&mut offset, &mut gif, contents) {
                Ok(_) => {}
                Err(error) => return Err(error),
              };
            }
            _ => {}
          }
        }
        0x3B => {
          done = true;
        }
        0x00 => {}
        _ => {}
      }
      if done {
        break;
      }
    }
    // Trailer
    #[cfg(debug_assertions)]
    println!("End of file.");
    return Ok(gif);
  }
  fn skip(offset: &mut usize, contents: &[u8]) -> Result<()> {
    loop {
      let data_sub_blocks_count;
      match contents.get(*offset) {
        Some(data_sub_blocks_count_byte) => {
          data_sub_blocks_count = *data_sub_blocks_count_byte;
        }
        None => {
          return Err(Error::from_reason(
            "Unable to get data_sub_blocks_count in skip, the file is corrupted".to_string(),
          ))
        }
      };
      Self::increment_offset(offset, 1);
      if data_sub_blocks_count > 0 {
        Self::increment_offset(offset, data_sub_blocks_count.into());
      } else {
        break;
      }
      if *offset >= contents.len() - 1 {
        break;
      }
    }
    Ok(())
  }
  fn increment_offset(offset: &mut usize, amount: usize) {
    *offset += amount;
  }
  fn handle_logical_screen_descriptor(gif: &mut Gif, contents: &[u8]) -> Result<()> {
    // Logic Screen Descriptor
    #[cfg(debug_assertions)]
    println!("Logic Screen Descriptor Offset: {}", 6);

    match contents.get(6..8) {
      Some(width_bytes) => {
        let width = LittleEndian::read_u16(width_bytes);
        gif.lsd.width = width as u32;
      }
      None => {
        return Err(Error::from_reason(
          "Unable to get gif width in handle_logical_screen_descriptor, the file is corrupted"
            .to_string(),
        ))
      }
    };
    match contents.get(8..10) {
      Some(height_bytes) => {
        let height = LittleEndian::read_u16(height_bytes);
        gif.lsd.height = height as u32;
      }
      None => {
        return Err(Error::from_reason(
          "Unable to get gif height in handle_logical_screen_descriptor, the file is corrupted"
            .to_string(),
        ))
      }
    };

    let packed_field;
    match contents.get(10) {
      Some(packed_field_bytes) => {
        packed_field = *packed_field_bytes;
      }
      None => {
        return Err(Error::from_reason(
          "Unable to get packed_field in handle_logical_screen_descriptor, the file is corrupted"
            .to_string(),
        ))
      }
    };

    gif.lsd.global_color_flag = (packed_field & 0b1000_0000) != 0; // global_color_flag
    gif.lsd.color_resolution = (packed_field & 0b0111_0000) as u32; // color_resolution
    gif.lsd.sorted_flag = (packed_field & 0b0000_1000) != 0; // sorted_flag
    gif.lsd.global_color_size = (packed_field & 0b0000_0111) as u32; // global_color_size

    match contents.get(11) {
      Some(background_color_index_byte) => {
        gif.lsd.background_color_index = *background_color_index_byte as u32;
      }
      None => {
        return Err(Error::from_reason(
          "Unable to get background_color_index in handle_logical_screen_descriptor, the file is corrupted".to_string(),
        ))
      }
    };
    match contents.get(12) {
      Some(pixel_aspect_ratio_byte) => {
        gif.lsd.pixel_aspect_ratio = *pixel_aspect_ratio_byte as u32;
      }
      None => {
        return Err(Error::from_reason(
          "Unable to get pixel_aspect_ratio in handle_logical_screen_descriptor, the file is corrupted".to_string(),
        ))
      }
    };
    return Ok(());
  }
  fn handle_graphic_control_extension(
    offset: &mut usize,
    gif: &mut Gif,
    contents: &[u8],
  ) -> Result<()> {
    // Graphical Control Extension
    #[cfg(debug_assertions)]
    println!("Graphic Control Extension Offset: {}", *offset);

    let mut parsed_frame: Frame = Frame::default();

    match contents.get(*offset) {
      Some(_) => {}
      None => {
        return Err(Error::from_reason(
          "Unable to get byte_size in handle_graphic_control_extension, the file is corrupted"
            .to_string(),
        ))
      }
    }; // Get byte size (I dont know what this is used for)
    Self::increment_offset(offset, 1);

    let packed_field: u8;
    match contents.get(*offset) {
      Some(packed_field_bytes) => {
        packed_field = *packed_field_bytes;
      }
      None => {
        return Err(Error::from_reason(
          "Unable to get packed_field in handle_graphic_control_extension, the file is corrupted"
            .to_string(),
        ))
      }
    };
    parsed_frame.gcd.disposal_method = shr_or((packed_field & 0b0001_1100) as u32, 2, 0);
    parsed_frame.gcd.user_input_flag = (packed_field & 0b0000_0010) != 0;
    parsed_frame.gcd.transparent_color_flag = (packed_field & 0b0000_0001) != 0;
    Self::increment_offset(offset, 1);

    match contents.get(*offset..*offset + 2) {
      Some(delay_time_bytes) => {
        parsed_frame.gcd.delay_time = LittleEndian::read_u16(delay_time_bytes) as u32;
      }
      None => {
        return Err(Error::from_reason(
          "Unable to get delay_time in handle_graphic_control_extension, the file is corrupted"
            .to_string(),
        ))
      }
    };
    Self::increment_offset(offset, 2);

    match contents.get(*offset) {
      Some(transparent_color_index_bytes) => {
        parsed_frame.gcd.transparent_color_index = *transparent_color_index_bytes as u32;
      }
      None => {
        return Err(Error::from_reason(
          "Unable to get transparent_color_index in handle_graphic_control_extension, the file is corrupted".to_string(),
        ))
      }
    };
    Self::increment_offset(offset, 1);

    match contents.get(*offset) {
      Some(_) => {}
      None => return Err(Error::from_reason(
        "Unable to get block_terminator in handle_graphic_control_extension, the file is corrupted"
          .to_string(),
      )),
    }; // Get block_terminator
    Self::increment_offset(offset, 1);
    // End

    gif.frames.push(parsed_frame);
    Ok(())
  }
  fn handle_image_descriptor(offset: &mut usize, gif: &mut Gif, contents: &[u8]) -> Result<()> {
    // Image Descriptor
    #[cfg(debug_assertions)]
    println!("Image Descriptor Offset: {}", *offset);

    let frame_index = gif.frames.len() - 1;
    let mut parsed_frame = &mut gif.frames[frame_index];

    match contents.get(*offset..*offset + 2) {
      Some(left_bytes) => {
        parsed_frame.im.left = LittleEndian::read_u16(left_bytes) as u32;
      }
      None => {
        return Err(Error::from_reason(
          "Unable to get image_left in handle_image_descriptor, the file is corrupted".to_string(),
        ))
      }
    };
    Self::increment_offset(offset, 2);

    match contents.get(*offset..*offset + 2) {
      Some(top_bytes) => {
        parsed_frame.im.top = LittleEndian::read_u16(top_bytes) as u32;
      }
      None => {
        return Err(Error::from_reason(
          "Unable to get image_top in handle_image_descriptor, the file is corrupted".to_string(),
        ))
      }
    };
    Self::increment_offset(offset, 2);

    match contents.get(*offset..*offset + 2) {
      Some(width_bytes) => {
        parsed_frame.im.width = LittleEndian::read_u16(width_bytes) as u32;
      }
      None => {
        return Err(Error::from_reason(
          "Unable to get image_width in handle_image_descriptor, the file is corrupted".to_string(),
        ))
      }
    };
    Self::increment_offset(offset, 2);

    match contents.get(*offset..*offset + 2) {
      Some(height_bytes) => {
        parsed_frame.im.height = LittleEndian::read_u16(height_bytes) as u32;
      }
      None => {
        return Err(Error::from_reason(
          "Unable to get image_height in handle_image_descriptor, the file is corrupted"
            .to_string(),
        ))
      }
    };
    Self::increment_offset(offset, 2);

    let packed_field;
    match contents.get(*offset) {
      Some(packed_field_byte) => {
        packed_field = *packed_field_byte;
      }
      None => {
        return Err(Error::from_reason(
          "Unable to get packed_field in handle_image_descriptor, the file is corrupted"
            .to_string(),
        ))
      }
    };
    parsed_frame.im.interlace_flag = (packed_field & 0b0100_0000) != 0;
    parsed_frame.im.sort_flag = (packed_field & 0b0010_0000) != 0;
    // let _ = (packed_field & 0b0001_1000) as u8; // Future use
    Self::increment_offset(offset, 1);
    // End

    // Local Color Table (Check local color table flag)
    if (packed_field & 0b1000_0000) != 0 {
      let length: usize = 3 * 2 << (packed_field & 0b0000_0111) as u32;
      let mut i: usize = *offset;
      let mut local_color_vector: Vec<Color> = Vec::new();

      while i < *offset + length {
        let red;
        let green;
        let blue;
        match contents.get(i) {
          Some(red_byte) => {
            red = *red_byte;
          }
          None => {
            return Err(Error::from_reason(
              "Unable to get red in handle_image_descriptor, the file is corrupted".to_string(),
            ))
          }
        };
        match contents.get(i + 1) {
          Some(green_byte) => {
            green = *green_byte;
          }
          None => {
            return Err(Error::from_reason(
              "Unable to get green in handle_image_descriptor, the file is corrupted".to_string(),
            ))
          }
        };
        match contents.get(i + 2) {
          Some(blue_byte) => {
            blue = *blue_byte;
          }
          None => {
            return Err(Error::from_reason(
              "Unable to get blue in handle_image_descriptor, the file is corrupted".to_string(),
            ))
          }
        };
        local_color_vector.push(Color {
          red: (red as u32),
          green: (green as u32),
          blue: (blue as u32),
        });
        i = i + 3;
      }
      Self::increment_offset(offset, length);
      parsed_frame.color_table = local_color_vector;
    } else {
      parsed_frame.color_table = gif.global_table.to_vec();
    }
    let null_code: i32 = -1;
    let npix = parsed_frame.im.width * parsed_frame.im.height;

    // Initialize GIF data stream decoder.
    let lzw_minimum_code_size;
    match contents.get(*offset) {
      Some(lzw_minimum_code_size_byte) => {
        lzw_minimum_code_size = *lzw_minimum_code_size_byte;
      }
      None => {
        return Err(Error::from_reason(
          "Unable to get lzw_minimum_code_size in handle_image_descriptor, the file is corrupted"
            .to_string(),
        ))
      }
    };
    Self::increment_offset(offset, 1);

    let clear_code = shl_or(1, lzw_minimum_code_size as usize, 0);
    let eoi_code = clear_code + 1;
    let mut available = clear_code + 2;
    let mut old_code = null_code;
    let mut code_size: usize = (lzw_minimum_code_size + 1) as usize;
    let mut code_mask = shl_or(1, code_size, 0) - 1;

    let mut prefix: Vec<u16> = vec![0; MAX_STACK_SIZE as usize]; // No need to fill with 0 (already filled)
    let mut suffix: Vec<u8> = vec![0; MAX_STACK_SIZE as usize];
    for code in 0..clear_code {
      *suffix.index_mut(code as usize) = code as u8;
    }

    let mut pixel_stack: Vec<u8> = vec![0; (MAX_STACK_SIZE + 1) as usize];
    let mut top = 0;

    let mut index_stream: Vec<u8> = Vec::new();

    let mut block: &[u8] = &[0];

    let mut in_code = 0;
    let mut first: u8 = 0;
    let mut datum: u32 = 0;
    let mut bits: usize = 0;
    let mut data_sub_blocks_count = 0;
    let mut bi = 0;

    let mut n = 0;
    while n < npix {
      if top == 0 {
        if bits < code_size {
          if data_sub_blocks_count == 0 {
            match contents.get(*offset) {
			  Some(data_sub_blocks_count_byte) => {
				data_sub_blocks_count = *data_sub_blocks_count_byte;
			  }
			  None => {
				return Err(Error::from_reason(
				  "Unable to get data_sub_blocks_count in handle_image_descriptor, the file is corrupted"
					.to_string(),
				))
			  }
			};
            Self::increment_offset(offset, 1);
            if data_sub_blocks_count <= 0 {
              break;
            }
            let offset_add: usize = *offset + data_sub_blocks_count as usize;
            match contents.get(*offset..offset_add) {
              Some(block_bytes) => {
                block = block_bytes;
              }
              None => {
                return Err(Error::from_reason(
                  "Unable to get block in handle_image_descriptor, the file is corrupted"
                    .to_string(),
                ))
              }
            };
            *offset = offset_add;
            bi = 0;
          }
          datum += shl_or(block[bi as usize] as u32 & 0xFF, bits, 0);
          bits += 8;
          bi += 1;
          data_sub_blocks_count -= 1;
          continue;
        }
        let mut code = datum & code_mask;
        datum = shr_or(datum, code_size, 0);
        bits -= code_size;
        if code > available || code == eoi_code {
          break;
        }
        if code == clear_code {
          code_size = (lzw_minimum_code_size + 1) as usize;
          code_mask = shl_or(1, code_size, 0) - 1;
          available = clear_code + 2;
          old_code = null_code;
          continue;
        }
        if old_code == null_code {
          index_stream.push(suffix[code as usize]);
          old_code = code as i32;
          first = code as u8;
          continue;
        }
        in_code = code;
        if code == available {
          *pixel_stack.index_mut(top as usize) = first as u8;
          top += 1;
          code = old_code as u32;
        }
        while code > clear_code {
          *pixel_stack.index_mut(top as usize) = suffix[code as usize];
          top += 1;
          code = prefix[code as usize] as u32;
        }
        first = suffix[code as usize] & 0xFF;

        *pixel_stack.index_mut(top as usize) = first;
        top += 1;

        if available < MAX_STACK_SIZE as u32 {
          *prefix.index_mut(available as usize) = old_code as u16;
          *suffix.index_mut(available as usize) = first;
          available += 1;
          if (available & code_mask) == 0 && available < MAX_STACK_SIZE as u32 {
            code_size += 1;
            code_mask += available;
          }
        }
        old_code = in_code as i32;
      }
      top -= 1;
      index_stream.push(pixel_stack[top]);
      n += 1;
    }
    for _ in index_stream.len()..npix as usize {
      index_stream.push(0);
    }
    if parsed_frame.im.interlace_flag {
      index_stream = Self::deinterlace(&mut index_stream, parsed_frame.im.width as usize);
    }
    parsed_frame.index_stream = index_stream;
    Ok(())
  }
  // deinterlace function from https://github.com/matt-way/gifuct-js/blob/master/src/deinterlace.js
  fn deinterlace(index_stream: &mut Vec<u8>, width: usize) -> Vec<u8> {
    let mut new_index_stream = vec![0; index_stream.len()];
    let rows = index_stream.len() / width;

    // See appendix E.
    let offsets = [0, 4, 2, 1];
    let steps = [8, 8, 4, 2];

    let mut from_row = 0;
    for pass in 0..4 {
      let mut to_row = offsets[pass];
      while to_row < rows {
        let from_pixels = &index_stream[from_row * width..(from_row + 1) * width];
        new_index_stream.splice(
          (to_row * width)..(to_row * width) + width,
          from_pixels.to_vec(),
        );
        from_row += 1;
        to_row += steps[pass];
      }
    }
    return new_index_stream;
  }
  fn handle_plain_text_extension(offset: &mut usize, gif: &mut Gif, contents: &[u8]) -> Result<()> {
    // Plain Text Extension (Optional)
    #[cfg(debug_assertions)]
    println!("Plain Text Extension Offset: {}", *offset);

    let block_size;
    match contents.get(*offset) {
      Some(block_size_byte) => {
        block_size = *block_size_byte as usize;
      }
      None => {
        return Err(Error::from_reason(
          "Unable to get block_size in handle_plain_text_extension, the file is corrupted"
            .to_string(),
        ))
      }
    };
    Self::increment_offset(offset, 1 + block_size);

    match Self::skip(offset, contents) {
      Ok(_) => {}
      Err(error) => return Err(error),
    };
    Ok(())
  }
  fn handle_application_extension(
    offset: &mut usize,
    gif: &mut Gif,
    contents: &[u8],
  ) -> Result<()> {
    // Application Extension (Optional)
    #[cfg(debug_assertions)]
    println!("Application Extension Offset: {}", *offset);

    let block_size;
    match contents.get(*offset) {
      Some(block_size_byte) => {
        block_size = *block_size_byte as usize;
      }
      None => {
        return Err(Error::from_reason(
          "Unable to get block_size in handle_application_extension, the file is corrupted"
            .to_string(),
        ))
      }
    };
    Self::increment_offset(offset, 1);

    let mut application = String::from("");
    let length = *offset + block_size;
    match contents.get(*offset..length) {
      Some(application_bytes) => match String::from_utf8(application_bytes.to_vec()) {
        Ok(parsed_application) => {
          application = parsed_application;
        }
        Err(err) => println!("Attempt to get application failed: {}", err),
      },
      None => {
        return Err(Error::from_reason(
          "Unable to get application in handle_application_extension, the file is corrupted"
            .to_string(),
        ))
      }
    };
    Self::increment_offset(offset, block_size);

    match Self::skip(offset, contents) {
      Ok(_) => {}
      Err(error) => return Err(error),
    };
    Ok(())
  }
  fn handle_comment_extension(offset: &mut usize, gif: &mut Gif, contents: &[u8]) -> Result<()> {
    // Comment Extension (Optional)
    #[cfg(debug_assertions)]
    println!("Comment Extension Offset: {}", *offset);

    match Self::skip(offset, contents) {
      Ok(_) => {}
      Err(error) => return Err(error),
    };
    Ok(())
  }
}
