#![deny(clippy::all)]

use napi::bindgen_prelude::Buffer;
use napi::{Env, Error, JsNumber, Result, Task};
use napi_derive::napi;

use byteorder::{ByteOrder, LittleEndian};
use std::collections::HashMap;
use std::process::exit;
mod DataHelper;
use DataHelper::BitReader;

//
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
  #[napi]
  pub fn process_frames(&mut self) -> Vec<Buffer> {
    let mut buffers: Vec<Buffer> = Vec::new();
    let frames_iter = self.frames.iter();
    for frame in frames_iter {
      let mut buffer: Vec<u8> = Vec::new();
      if frame.im.local_color_table_flag {
        for index in (&frame.index_stream).into_iter() {
          let color = frame.local_table.get(*index as usize).unwrap();
          buffer.push(color.red.try_into().unwrap());
          buffer.push(color.green.try_into().unwrap());
          buffer.push(color.blue.try_into().unwrap());
          if frame.gcd.transparent_color_flag
            && index == (&frame.gcd.transparent_color_index.try_into().unwrap())
          {
            buffer.push(0);
          } else {
            buffer.push(255);
          }
        }
      } else {
        for index in (&frame.index_stream).into_iter() {
          let color = self.global_table.get(*index as usize).unwrap();
          buffer.push(color.red.try_into().unwrap());
          buffer.push(color.green.try_into().unwrap());
          buffer.push(color.blue.try_into().unwrap());
          if frame.gcd.transparent_color_flag
            && index == (&frame.gcd.transparent_color_index.try_into().unwrap())
          {
            buffer.push(0);
          } else {
            buffer.push(255);
          }
        }
      }
      buffers.push(Buffer::from(buffer));
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
#[napi(object)]
pub struct Frame {
  pub gcd: GraphicsControlExtension,
  pub im: ImageDescriptor,
  pub local_table: Vec<Color>, // Check localColorTableFlag before using this
  pub index_stream: Vec<u8>,
}

#[derive(Default, Clone)]
#[napi(object)]
pub struct ImageDescriptor {
  pub left: u32,
  pub top: u32,
  pub width: u32,
  pub height: u32,
  pub local_color_table_flag: bool,
  pub interface_flag: bool,
  pub sort_flag: bool,
  pub local_color_table_size: u32,
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
  pub fn decode(file_path: String) -> Result<Gif> {
    let contents = match std::fs::read(&file_path) {
      Ok(contents) => contents,
      Err(err) => return Err(Error::from_reason(err.to_string())),
    };

    let contents = contents.as_slice();
    {
      let mut signature: String = String::new();
      match String::from_utf8(contents[0..3].to_vec()) {
        Ok(parsed_signature) => {
          signature = parsed_signature;
        }
        Err(err) => return Err(Error::from_reason(err.to_string())),
      }
      if signature != "GIF" {
        return Err(Error::from_reason(
          "Error: The file's signature is not GIF got: ".to_string() + &signature,
        ));
      }
    }

    let mut gif = Gif::default();
    let mut version: String = String::new();
    match String::from_utf8(contents[3..6].to_vec()) {
      Ok(parsed_version) => {
        version = parsed_version;
      }
      Err(err) => return Err(Error::from_reason(err.to_string())),
    }
    gif.version = version;

    Self::handle_logical_screen_descriptor(&mut gif, contents);

    let mut offset: usize = 13;

    // Global Color Table
    let length: usize = 3 * 2 << gif.lsd.global_color_size;
    let mut i: usize = offset;

    if gif.lsd.global_color_flag {
      let mut global_color_vector: Vec<Color> = Vec::new();

      while i < offset + length {
        global_color_vector.push(Color {
          red: (contents[i] as u32),
          green: (contents[i + 1] as u32),
          blue: (contents[i + 2] as u32),
        });
        i = i + 3;
      }
      Self::increment_offset(&mut offset, length);
      gif.global_table = global_color_vector;
    }
    // End
    loop {
      let introducer = contents[offset];
      if introducer == 0x3B {
        break;
      }
      Self::increment_offset(&mut offset, 1);
      if introducer == 0x2C {
        Self::handle_image_descriptor(&mut offset, &mut gif, contents);
        continue;
      }
      let label = contents[offset];
      Self::increment_offset(&mut offset, 1);
      match label {
        0xF9 => {
          Self::handle_graphic_control_extension(&mut offset, &mut gif, contents);
        }
        0x01 => {
          Self::handle_plain_text_extension(&mut offset, &mut gif, contents);
        }
        0xFF => {
          Self::handle_application_extension(&mut offset, &mut gif, contents);
        }
        0xFE => {
          Self::handle_comment_extension(&mut offset, &mut gif, contents);
        }
        _ => {}
      }
    }
    // Trailer
    #[cfg(debug_assertions)]
    println!("End of file.");
    return Ok(gif);
  }
  fn skip(offset: &mut usize, contents: &[u8]) {
    let mut data_sub_blocks_count = contents[*offset];
    Self::increment_offset(offset, 1);
    loop {
      Self::increment_offset(offset, data_sub_blocks_count.into());
      data_sub_blocks_count = contents[*offset];
      Self::increment_offset(offset, 1);
      if data_sub_blocks_count == 0x00 {
        break;
      }
    }
  }
  fn increment_offset(offset: &mut usize, amount: usize) {
    *offset += amount;
  }
  fn shl_or(offset: &mut usize, val: u16, shift: usize, def: u16) -> u16 {
    [val << (shift & 15), def][((shift & !7) != 0) as usize]
  }
  fn handle_logical_screen_descriptor(gif: &mut Gif, contents: &[u8]) {
    // Logic Screen Descriptor
    #[cfg(debug_assertions)]
    println!("Logic Screen Descriptor Offset: {}", 6);

    gif.lsd.width = LittleEndian::read_u16(&contents[6..8]) as u32; // width
    gif.lsd.height = LittleEndian::read_u16(&contents[8..10]) as u32; // height

    let packed_field = contents[10];

    gif.lsd.global_color_flag = (packed_field & 0b1000_0000) != 0; // global_color_flag
    gif.lsd.color_resolution = (packed_field & 0b0111_0000) as u32; // color_resolution
    gif.lsd.sorted_flag = (packed_field & 0b0000_1000) != 0; // sorted_flag
    gif.lsd.global_color_size = (packed_field & 0b0000_0111) as u32; // global_color_size

    gif.lsd.background_color_index = contents[11] as u32; // background_color_index
    gif.lsd.pixel_aspect_ratio = contents[12] as u32; // pixel_aspect_ratio
  }
  fn handle_graphic_control_extension(offset: &mut usize, gif: &mut Gif, contents: &[u8]) {
    // Graphical Control Extension
    #[cfg(debug_assertions)]
    println!("Graphic Control Extension Offset: {}", *offset);

    let mut parsed_frame: Frame = Frame::default();

    let byte_size = contents[*offset];
    Self::increment_offset(offset, 1);

    let packed_field = contents[*offset];
    parsed_frame.gcd.disposal_method = (packed_field & 0b0001_1100) as u32;
    parsed_frame.gcd.user_input_flag = (packed_field & 0b0000_0010) != 0;
    parsed_frame.gcd.transparent_color_flag = (packed_field & 0b0000_0001) != 0;
    Self::increment_offset(offset, 1);

    parsed_frame.gcd.delay_time = LittleEndian::read_u16(&contents[*offset..*offset + 2]) as u32;
    Self::increment_offset(offset, 2);

    parsed_frame.gcd.transparent_color_index = contents[*offset] as u32;
    Self::increment_offset(offset, 1);

    let block_terminator = contents[*offset]; // This must be 00 ///////////////////////////////////////////////////////////////////
    Self::increment_offset(offset, 1);
    // End

    gif.frames.push(parsed_frame);
  }
  fn handle_image_descriptor(offset: &mut usize, gif: &mut Gif, contents: &[u8]) {
    // Image Descriptor
    #[cfg(debug_assertions)]
    println!("Image Descriptor Offset: {}", *offset);

    let frame_index = gif.frames.len() - 1;
    let mut parsed_frame = &mut gif.frames[frame_index];

    parsed_frame.im.left = LittleEndian::read_u16(&contents[*offset..*offset + 2]) as u32; // image_left
    Self::increment_offset(offset, 2);

    parsed_frame.im.top = LittleEndian::read_u16(&contents[*offset..*offset + 2]) as u32; // image_top
    Self::increment_offset(offset, 2);

    parsed_frame.im.width = LittleEndian::read_u16(&contents[*offset..*offset + 2]) as u32; // image_width
    Self::increment_offset(offset, 2);

    parsed_frame.im.height = LittleEndian::read_u16(&contents[*offset..*offset + 2]) as u32; // image_height
    Self::increment_offset(offset, 2);

    let packed_field = contents[*offset];
    parsed_frame.im.local_color_table_flag = (packed_field & 0b1000_0000) != 0;
    parsed_frame.im.interface_flag = (packed_field & 0b0100_0000) != 0;
    parsed_frame.im.sort_flag = (packed_field & 0b0010_0000) != 0;
    // let _ = (packed_field & 0b0001_1000) as u8; // Future use
    parsed_frame.im.local_color_table_size = (packed_field & 0b0000_0111) as u32;
    Self::increment_offset(offset, 1);
    // End

    // Local Color Table
    if parsed_frame.im.local_color_table_flag {
      let length: usize = 3 * 2 << parsed_frame.im.local_color_table_size;
      let mut i: usize = *offset;
      let mut local_color_vector: Vec<Color> = Vec::new();

      while i < *offset + length {
        local_color_vector.push(Color {
          red: (contents[i] as u32),
          green: (contents[i + 1] as u32),
          blue: (contents[i + 2] as u32),
        });
        i = i + 3;
      }
      Self::increment_offset(offset, length);
      parsed_frame.local_table = local_color_vector;
    }
    // End

    // Image Data
    #[cfg(debug_assertions)]
    println!("Image Data Offset: {}", *offset);

    let lzw_minimum_code_size = contents[*offset];
    Self::increment_offset(offset, 1);

    // Data sub block section
    let mut data_sub_blocks_count = contents[*offset];
    Self::increment_offset(offset, 1);

    let clear_code = Self::shl_or(offset, 2, (lzw_minimum_code_size - 1).into(), 0);
    let eoi_code = clear_code + 1;

    let mut index_stream: Vec<u8> = Vec::new();

    let mut code_table: HashMap<usize, Vec<u8>> = HashMap::new();
    let mut code_stream: Vec<u16> = Vec::new();
    for n in 0..=eoi_code {
      if n < clear_code {
        code_table.insert(n as usize, vec![n as u8]);
      } else {
        code_table.insert(n as usize, vec![]);
      }
    }

    let mut last_code = eoi_code;
    let mut size: usize = (lzw_minimum_code_size + 1).into();
    let mut grow_code = clear_code - 1;

    let mut is_initalized = false;

    let mut br = BitReader::new();
    loop {
      let offset_add: usize = *offset + data_sub_blocks_count as usize;
      let sliced_bytes = &contents[*offset..offset_add];

      br.push_bytes(&sliced_bytes);
      loop {
        let code = br.read_bits(size).unwrap();
        if code == eoi_code {
          code_stream.push(code);
          break;
        } else if code > last_code {
          break;
        } else if code == clear_code {
          code_stream = Vec::new();
          code_table = HashMap::new();
          for n in 0..=eoi_code {
            if n < clear_code {
              code_table.insert(n as usize, vec![n as u8]);
            } else {
              code_table.insert(n as usize, vec![]);
            }
          }
          last_code = eoi_code;
          size = (lzw_minimum_code_size + 1).into();
          grow_code = (2 << size - 1) - 1;
          is_initalized = false;
        } else if !is_initalized {
          match code_table.get(&(code as usize)) {
            Some(codes) => {
              index_stream.extend(codes);
            }
            None => {
              println!("invalid code: {}", code);
              exit(1);
            }
          }
          is_initalized = true;
        } else {
          let mut k: u8 = 0;
          let prev_code = code_stream[code_stream.len() - 1];
          if code <= last_code {
            match code_table.get(&(code as usize)) {
              Some(codes) => {
                index_stream.extend(codes);
                k = codes[0];
              }
              None => {
                println!("invalid code: {}", code);
                exit(2);
              }
            }
          } else {
            match code_table.get(&(prev_code as usize)) {
              Some(codes) => {
                k = codes[0];
                index_stream.extend(codes);
                index_stream.push(k);
              }
              None => {
                println!("invalid code: {}", prev_code);
                exit(3);
              }
            }
          }
          if last_code < 0xFFF {
            match code_table.get(&(prev_code as usize)) {
              Some(codes) => {
                last_code += 1;
                let mut last_code_table = codes.to_vec();
                last_code_table.push(k);
                code_table.insert(last_code as usize, last_code_table);
                if last_code == grow_code && last_code < 0xFFF {
                  size += 1;
                  grow_code = (2 << size - 1) - 1;
                }
              }
              None => {
                println!("invalid code: {}", prev_code);
                exit(4);
              }
            }
          }
        }
        code_stream.push(code);
        let has_bits = match br.has_bits(size) {
          Some(has_bits) => has_bits,
          None => {
            exit(0x0); ///////////todo
          }
        };
        if !has_bits {
          break;
        }
      }

      *offset = offset_add;
      data_sub_blocks_count = contents[*offset];
      Self::increment_offset(offset, 1);
      if data_sub_blocks_count == 0 {
        break;
      }
    }
    parsed_frame.index_stream = index_stream;
  }
  fn handle_plain_text_extension(offset: &mut usize, gif: &mut Gif, contents: &[u8]) {
    // Plain Text Extension (Optional)
    #[cfg(debug_assertions)]
    println!("Plain Text Extension Offset: {}", *offset);

    let block_size: usize = contents[*offset].into();
    Self::increment_offset(offset, 1 + block_size);

    Self::skip(offset, contents);
  }
  fn handle_application_extension(offset: &mut usize, gif: &mut Gif, contents: &[u8]) {
    // Application Extension (Optional)
    #[cfg(debug_assertions)]
    println!("Application Extension Offset: {}", *offset);

    let block_size: usize = contents[*offset].into();
    Self::increment_offset(offset, 1);

    let mut application = String::from("");
    let length = *offset + block_size;
    match String::from_utf8(contents[*offset..length].to_vec()) {
      Ok(parsed_application) => {
        application = parsed_application;
      }
      Err(err) => println!("Attempt to get application failed: {}", err),
    }
    Self::increment_offset(offset, block_size);

    Self::skip(offset, contents);
  }
  fn handle_comment_extension(offset: &mut usize, gif: &mut Gif, contents: &[u8]) {
    // Comment Extension (Optional)
    #[cfg(debug_assertions)]
    println!("Comment Extension Offset: {}", *offset);

    Self::skip(offset, contents);
  }
}
