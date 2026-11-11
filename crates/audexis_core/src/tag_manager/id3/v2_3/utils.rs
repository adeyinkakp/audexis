pub(crate) use super::super::v2_common::{
    build_frame, decode_text_payload, encode_img_payload, encode_text_payload, split_encoded_text,
    to_synchsafe,
};

pub(crate) use super::super::v2_common::create_header as create_header_with_version;

pub(crate) fn create_header(tag_size: usize) -> [u8; 10] {
    super::super::v2_common::create_header(3, tag_size)
}
