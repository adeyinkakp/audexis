use symphonia_adapter_libopus::OpusDecoder;
use symphonia_core::codecs::registry::CodecRegistry;
pub fn get_registry() -> CodecRegistry {
    let mut codec_registry = CodecRegistry::new();

    symphonia::default::register_enabled_codecs(&mut codec_registry);
    codec_registry.register_audio_decoder::<OpusDecoder>();
    codec_registry
}
