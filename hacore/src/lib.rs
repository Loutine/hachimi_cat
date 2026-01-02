#[cfg(target_vendor = "apple")]
pub mod apple_platform_audio_engine;
pub mod crossplatform_audio_processor;
pub mod default_audio_engine;

// use libhachimi::audio_processing::AudioProcessor;

#[derive(Debug, Clone)]
pub enum DecodeCommand {
    DecodeNormal(Vec<u8>),
    DecodeFEC(Vec<u8>),
    DecodePLC,
}

pub trait EngineBuilder {
    fn build(
        encoder_output: tokio::sync::mpsc::Sender<Vec<u8>>,
        decoder_input: ringbuf::HeapCons<DecodeCommand>,
    ) -> anyhow::Result<Box<Self>>;
}

pub trait AudioEngine {
    fn notify_decoder(&self);

    fn play(&self) -> anyhow::Result<()>;

    fn pause(&self) -> anyhow::Result<()>;

    fn enable_mic(&self) -> anyhow::Result<()>;

    fn disable_mic(&self) -> anyhow::Result<()>;
}
