#![deny(clippy::all)]

use napi::bindgen_prelude::{Buffer, FromNapiValue};
use napi::{Error, Result};
use napi_derive::napi;

use byteorder::{ByteOrder, LittleEndian};
use std::ops::IndexMut;

const MAX_STACK_SIZE: u16 = 4096;

fn shl_or(val: u32, shift: usize, def: u32) -> u32 {
  [val << (shift & 31), def][((shift & !31) != 0) as usize]
}
fn shr_or(val: u32, shift: usize, def: u32) -> u32 {
  [val >> (shift & 31), def][((shift & !31) != 0) as usize]
}

#[derive(Default)]
#[napi(js_name = "Gif")]
struct Gif {
  pub version: String,
  pub lsd: LogicalScreenDescriptor,
  pub global_table: Vec<Color>, // Check globalColorFlag before using this
  pub frames: Vec<Frame>,
}

#[napi]
impl Gif {
  // Goes through every index stream of the frames and makes a vector of buffers from them
  #[napi]
  pub fn decode_frames(&mut self) -> Vec<Buffer> {
    let mut buffers: Vec<Buffer> = Vec::new();
    let frames_iter = self.frames.iter();
    for frame in frames_iter {
      buffers.push(frame.decode());
    }
    return buffers;
  }
}

#[derive(Default, Clone)]
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

#[derive(Default, Clone)]
#[napi(js_name = "Frame")]
pub struct Frame {
  pub gcd: GraphicsControlExtension,
  pub im: ImageDescriptor,
  pub color_table: Vec<Color>,
  pub index_stream: Vec<u8>,
}

#[napi]
impl Frame {
  #[napi]
  pub fn decode(&self) -> Buffer {
    let mut buffer: Vec<u8> = Vec::new();
    for index in (&self.index_stream).into_iter() {
      match self.color_table.get(*index as usize) {
        Some(color) => {
          buffer.push(color.red.try_into().unwrap());
          buffer.push(color.green.try_into().unwrap());
          buffer.push(color.blue.try_into().unwrap());
          if self.gcd.transparent_color_flag
            && index == (&self.gcd.transparent_color_index.try_into().unwrap())
          {
            buffer.push(0);
          } else {
            buffer.push(255);
          }
        }
        None => {
          for _ in 0..3 {
            buffer.push(255);
          }
          buffer.push(0);
        }
      }
    }
    Buffer::from(buffer)
  }
}

impl FromNapiValue for Frame {
  fn from_unknown(value: napi::JsUnknown) -> Result<Self> {
    Ok(Self {
      gcd: GraphicsControlExtension::default(),
      im: ImageDescriptor::default(),
      color_table: Vec::default(),
      index_stream: Vec::default(),
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
    })
  }
}

#[derive(Default, Clone)]
#[napi(object)]
pub struct ImageDescriptor {
  pub left: u32,
  pub top: u32,
  pub width: u32,
  pub height: u32,
  pub interlace_flag: bool,
  pub sort_flag: bool,
}

#[derive(Default, Clone)]
#[napi(object)]
pub struct GraphicsControlExtension {
  pub disposal_method: u32,
  pub user_input_flag: bool,
  pub transparent_color_flag: bool,
  pub delay_time: u32,
  pub transparent_color_index: u32,
}

#[derive(Default, Clone)]
#[napi(object)]
pub struct Color {
  pub red: u32,
  pub green: u32,
  pub blue: u32,
}

///

#[napi(js_name = "Decoder")]
struct Decoder {}

#[napi]
impl Decoder {
  #[napi]
  pub fn decode_path(file_path: String) -> Result<Gif> {
    let contents = std::fs::read(file_path).expect("Something went wrong reading the file");
    let contents = contents.as_slice();
    return Self::decode_internal(contents);
  }

  #[napi]
  pub fn decode_buffer(buffer: Buffer) -> Result<Gif> {
    let contents = buffer.to_vec();
    let contents = contents.as_slice();
    return Self::decode_internal(contents);
  }

  fn decode_internal(contents: &[u8]) -> Result<Gif> {
    {
      let mut signature: String = String::new();
      match String::from_utf8(contents[0..3].to_vec()) {
        Ok(parsed_signature) => {
          signature = parsed_signature;
        }
        Err(err) => return Err(Error::from_reason(err.to_string())),
      }
      if signature != "GIF" {
        return Err(Error::from_reason(format!(
          "Invalid file signature, got {}",
          signature
        )));
      }
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
    if parsed_frame.gcd.disposal_method == 0 {
      parsed_frame.gcd.disposal_method = 1; // elect to keep old image if discretionary
    }
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
