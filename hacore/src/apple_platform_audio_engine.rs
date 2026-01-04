use std::sync::Arc;

use coreaudio::audio_unit::{
    AudioUnit, IOType, Scope, StreamFormat,
    audio_format::LinearPcmFlags,
    render_callback::{Args, data::NonInterleaved},
};
use ringbuf::{HeapRb, traits::*};

// use libhachimi::audio_processing::AudioProcessor;
use libhachimi::AudioProcessor;

use crate::{
    AudioEngine,
    EngineBuilder,
    FRAME20MS,
    apple_platform_audio_processor::ApplePlatformAudioProcessor,
    // empty_audio_processor::EmptyAudioProcessor,
};

// use coreaudio::audio_unit::

pub struct ApplePlatformAudioEngine {
    vpio_unit: AudioUnit,
    // pub decode_process: Arc<JoinHandle<()>>,
}

impl Drop for ApplePlatformAudioEngine {
    fn drop(&mut self) {
        let _ = self.vpio_unit.stop();
        let _ = self.vpio_unit.uninitialize();
        let _ = self.vpio_unit.free_input_callback();
        let _ = self.vpio_unit.free_render_callback();
    }
}

// FIXME
unsafe impl Send for ApplePlatformAudioEngine {}
unsafe impl Sync for ApplePlatformAudioEngine {}

impl EngineBuilder for ApplePlatformAudioEngine {
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
        let mut vpio_unit = AudioUnit::new(IOType::VoiceProcessingIO)?;
        vpio_unit.uninitialize()?;

        vpio_unit.set_stream_format(
            StreamFormat {
                sample_rate: 48000.0,
                sample_format: coreaudio::audio_unit::SampleFormat::F32,
                flags: LinearPcmFlags::IS_FLOAT
                    | LinearPcmFlags::IS_PACKED
                    | LinearPcmFlags::IS_NON_INTERLEAVED,
                channels: 1,
            },
            Scope::Output,
            coreaudio::audio_unit::Element::Input,
        )?;

        vpio_unit.set_stream_format(
            StreamFormat {
                sample_rate: 48000.0,
                sample_format: coreaudio::audio_unit::SampleFormat::F32,
                flags: LinearPcmFlags::IS_FLOAT
                    | LinearPcmFlags::IS_PACKED
                    | LinearPcmFlags::IS_NON_INTERLEAVED,
                channels: 1,
            },
            Scope::Input,
            coreaudio::audio_unit::Element::Output,
        )?;

        // buffer init

        let mic_buf = HeapRb::<f32>::new(FRAME20MS * 2);
        let (mut mic_prod, mic_cons) = mic_buf.split();

        let speaker_buf = HeapRb::<f32>::new(FRAME20MS * 2);
        let (speaker_prod, mut speaker_cons) = speaker_buf.split();

        // let ap_to_encoder = HeapRb::<f32>::new(FRAME20MS * 2);
        // let (ap_mic_output, mut encoder_input) = ap_to_encoder.split();
        // let decoder_to_ap = HeapRb::<f32>::new(FRAME20MS * 2);
        // let (mut decoder_output, ap_ref_input) = decoder_to_ap.split();

        // start threads

        let audio_process = std::thread::Builder::new()
            .name("Audio Pipeline Thread".to_owned())
            .spawn(move || {
                let mut ap = ApplePlatformAudioProcessor::build().unwrap();
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
            })
            .unwrap();
        let audio_process = Arc::new(audio_process);
        let audio_process_0 = audio_process.clone();
        let audio_process_1 = audio_process.clone();

        vpio_unit.set_input_callback(move |args: Args<NonInterleaved<f32>>| {
            let Args { data, .. } = args;
            for channel in data.channels() {
                mic_prod.push_slice(channel);
            }
            audio_process_0.thread().unpark();
            Ok(())
        })?;

        vpio_unit.set_render_callback(move |args: Args<NonInterleaved<f32>>| {
            let Args { mut data, .. } = args;
            // FIXME
            for channel in data.channels_mut() {
                for channel_sample in channel.iter_mut() {
                    if let Some(sample) = speaker_cons.try_pop() {
                        *channel_sample = sample;
                    } else {
                        *channel_sample = 0.0;
                    }
                }
            }
            audio_process_1.thread().unpark();
            Ok(())
        })?;

        // vpio_unit.set_input_callback(move |args: Args<Interleaved<f32>>| {
        //     let Args { data, .. } = args;
        //     mic_prod.push_slice(data.buffer);
        //     audio_process_1.thread().unpark();
        //     Ok(())
        // })?;

        // vpio_unit.set_render_callback(move |args: Args<Interleaved<f32>>| {
        //     let Args { data, .. } = args;
        //     // 只能象征性催一下
        //     // audio_process_2.thread().unpark();
        //     for frame in data.buffer.chunks_exact_mut(data.channels) {
        //         if let Some(sample) = speaker_cons.try_pop() {
        //             for channel_sample in frame.iter_mut() {
        //                 *channel_sample = sample;
        //             }
        //         } else {
        //             for channel_sample in frame.iter_mut() {
        //                 *channel_sample = 0.0;
        //             }
        //         }
        //     }
        //     Ok(())
        // })?;

        vpio_unit.initialize()?;

        vpio_unit.start()?;

        println!("Audio system running.");

        Ok(Arc::new(ApplePlatformAudioEngine { vpio_unit }))
    }
}

impl AudioEngine for ApplePlatformAudioEngine {
    // fn notify_decoder(&self) {
    //     self.decode_process.thread().unpark();
    // }
    // fn get_decoder_thread(&self) -> Arc<JoinHandle<()>> {
    // self.decode_process.clone()
    // }

    fn play(&mut self) -> anyhow::Result<()> {
        // reset pipelie ringbuffer
        self.vpio_unit.start()?;
        Ok(())
    }

    fn pause(&mut self) -> anyhow::Result<()> {
        self.vpio_unit.stop()?;
        Ok(())
    }
}
