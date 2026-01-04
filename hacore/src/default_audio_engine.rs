use std::sync::Arc;

use ringbuf::{HeapRb, traits::*};

// use libhachimi::audio_processing::AudioProcessor;
use libhachimi::{AudioProcessor, constant::*, error};

use crate::{
    AudioEngine, EngineBuilder, FRAME20MS,
    cross_platform_audio_processor::CrossPlatformAudioProcessor,
};

use cpal::{
    self, SampleFormat, Stream, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

pub struct DefaultAudioEngine {
    input_stream: Stream,
    output_stream: Stream,
}

impl EngineBuilder for DefaultAudioEngine {
    /// # Safety
    /// This function is **non-reentrant**. The caller must ensure that
    /// no two threads enter this function simultaneously.
    /// TODO: Rewrite this function.
    fn build(
        encoder_input: ringbuf::HeapProd<f32>,
        decoder_output: ringbuf::HeapCons<f32>,
        encode_thread: std::thread::JoinHandle<()>,
        mixer_thread: Arc<std::thread::JoinHandle<()>>,
    ) -> anyhow::Result<Arc<Self>> {
        // config

        let host = cpal::default_host();

        let input_device = host
            .default_input_device()
            .ok_or(error::Error::InputDeviceInitError)?;

        let mut supported_input_configs = input_device.supported_input_configs()?;
        let input_config = supported_input_configs
            .find(|config| {
                config.sample_format() == SampleFormat::F32
                    && config.min_sample_rate() <= SAMPLE_RATE
                    && config.max_sample_rate() >= SAMPLE_RATE
                    && config.channels() == 1
            })
            .map(|config| config.with_sample_rate(SAMPLE_RATE))
            .ok_or(error::Error::UnsupportedInputSampleFormat)?;

        let input_config: StreamConfig = input_config.into();

        let output_device = host
            .default_output_device()
            .ok_or(error::Error::OutputDeviceInitError)?;

        let mut supported_output_configs = output_device.supported_output_configs()?;
        let output_config = supported_output_configs
            .find(|config| {
                config.sample_format() == SampleFormat::F32
                    && config.min_sample_rate() <= SAMPLE_RATE
                    && config.max_sample_rate() >= SAMPLE_RATE
                    && config.channels() <= 2
            })
            .map(|config| config.with_sample_rate(SAMPLE_RATE))
            .ok_or(error::Error::UnsupportedOutputSampleFormat)?;

        let output_config: StreamConfig = output_config.into();

        let output_channels = output_config.channels as usize;

        // buffer init

        let mic_buf = HeapRb::<f32>::new(FRAME20MS * 2);
        let (mut mic_prod, mic_cons) = mic_buf.split();

        let speaker_buf = HeapRb::<f32>::new(FRAME20MS * 2);
        let (speaker_prod, mut speaker_cons) = speaker_buf.split();

        // start threads

        let audio_process = std::thread::Builder::new()
            .name("Audio Pipeline Thread".to_owned())
            .spawn(move || {
                let mut ap = CrossPlatformAudioProcessor::build().unwrap();
                let mut mic_cons = mic_cons;
                let mut ap_ref_input = decoder_output;
                let mut ap_mic_output = encoder_input;
                let mut speaker_prod = speaker_prod;
                loop {
                    ap.process(
                        &mut mic_cons,
                        &mut ap_ref_input,
                        &mut ap_mic_output,
                        &mut speaker_prod,
                    );
                    encode_thread.thread().unpark();
                    mixer_thread.thread().unpark();
                    std::thread::park();
                }
            })?;
        let audio_process = Arc::new(audio_process);
        let audio_process_0 = audio_process.clone();
        let audio_process_1 = audio_process.clone();

        let input_stream = input_device.build_input_stream(
            &input_config,
            move |data: &[f32], _| {
                mic_prod.push_slice(data);
                audio_process_0.thread().unpark();
            },
            |err| panic!("input error: {:?}", err),
            None,
        )?;

        let output_stream = output_device.build_output_stream(
            &output_config,
            move |output: &mut [f32], _| {
                audio_process_1.thread().unpark();
                for frame in output.chunks_exact_mut(output_channels) {
                    if let Some(sample) = speaker_cons.try_pop() {
                        for channel_sample in frame.iter_mut() {
                            *channel_sample = sample;
                        }
                    } else {
                        for channel_sample in frame.iter_mut() {
                            *channel_sample = 0.0;
                        }
                    }
                }
            },
            |err| panic!("output error: {:?}", err),
            None,
        )?;

        input_stream.play()?;
        output_stream.play()?;
        println!("Audio system running.");

        Ok(Arc::new(DefaultAudioEngine {
            input_stream,
            output_stream,
        }))
    }
}

impl AudioEngine for DefaultAudioEngine {
    fn play(&mut self) -> anyhow::Result<()> {
        self.input_stream.play()?;
        self.output_stream.play()?;
        Ok(())
    }

    fn pause(&mut self) -> anyhow::Result<()> {
        self.input_stream.pause()?;
        self.output_stream.pause()?;
        Ok(())
    }
}
