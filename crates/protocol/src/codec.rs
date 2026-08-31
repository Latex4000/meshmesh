use tokio_util::codec::LengthDelimitedCodec;

pub fn codec() -> LengthDelimitedCodec {
    LengthDelimitedCodec::builder()
        .length_field_type::<u32>()
        .big_endian()
        .max_frame_length(1 << 20)
        .new_codec()
}
