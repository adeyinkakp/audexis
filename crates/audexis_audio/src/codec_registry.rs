use symphonia::default::codecs::{
    AdpcmDecoder, AlacDecoder, FlacDecoder, Mp3Decoder, MpaDecoder, PcmDecoder, VorbisDecoder,
};

use symphonia_adapter_libopus::OpusDecoder;
use symphonia_core::codecs::registry::CodecRegistry;
pub fn get_registry() -> CodecRegistry {
    let mut codec_registry = CodecRegistry::new();

    codec_registry.register_audio_decoder::<Mp3Decoder>();
    codec_registry.register_audio_decoder::<MpaDecoder>();
    codec_registry.register_audio_decoder::<PcmDecoder>();
    codec_registry.register_audio_decoder::<AlacDecoder>();
    codec_registry.register_audio_decoder::<FlacDecoder>();
    codec_registry.register_audio_decoder::<OpusDecoder>();
    codec_registry.register_audio_decoder::<AdpcmDecoder>();
    codec_registry.register_audio_decoder::<AlacDecoder>();
    codec_registry.register_audio_decoder::<VorbisDecoder>();
    codec_registry.register_audio_decoder::<AdpcmDecoder>();

    codec_registry
}
