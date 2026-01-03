use libhachimi::AudioProcessor;
use nnnoiseless::DenoiseState;
use ringbuf::{
    HeapCons, HeapProd,
    traits::{Consumer, Observer, Producer},
};
// use webrtc_audio_processing::InitializationConfig;

use crate::FRAME10MS;

pub struct ApplePlatformAudioProcessor {
    // Singal Process State Machines
    // post_processor: Processor,
    denoise: Box<DenoiseState<'static>>,
}

impl ApplePlatformAudioProcessor {
    pub fn build() -> anyhow::Result<Self> {
        // let init_config = &InitializationConfig {
        //     num_capture_channels: 1,
        //     num_render_channels: 1,
        //     enable_experimental_agc: false,
        //     enable_intelligibility_enhancer: false,
        // };

        // let post_config = Config {
        //     echo_cancellation: None,
        //     gain_control: Some(GainControl {
        //         mode: webrtc_audio_processing::GainControlMode::AdaptiveDigital,
        //         target_level_dbfs: 3,
        //         compression_gain_db: 20,
        //         enable_limiter: true,
        //     }),
        //     noise_suppression: None,
        //     voice_detection: None,
        //     enable_transient_suppressor: false,
        //     enable_high_pass_filter: false,
        // };

        // let mut post_processor = Processor::new(init_config)?;
        // post_processor.set_config(post_config);

        let denoise = DenoiseState::new();

        Ok(Self {
            // post_processor,
            denoise,
        })
    }
}
impl AudioProcessor for ApplePlatformAudioProcessor {
    fn process(
        &mut self,
        mic_cons: &mut HeapCons<f32>,
        ref_cons: &mut HeapCons<f32>,
        mic_prod: &mut HeapProd<f32>,
        ref_prod: &mut HeapProd<f32>,
    ) {
        let mut mic_frame = [0f32; FRAME10MS];
        let mut ref_frame = [0f32; FRAME10MS];
        let mut output_frame = [0f32; FRAME10MS];

        while mic_cons.occupied_len() >= FRAME10MS
            && ref_cons.occupied_len() >= FRAME10MS
            && mic_prod.vacant_len() >= FRAME10MS
            && ref_prod.vacant_len() >= FRAME10MS
        {
            ref_cons.pop_slice(&mut ref_frame);
            ref_prod.push_slice(&ref_frame);
            mic_cons.pop_slice(&mut mic_frame);

            self.denoise.process_frame(&mut output_frame, &mic_frame);

            // self.post_processor
            //     .process_capture_frame(&mut output_frame)
            //     .unwrap();

            mic_prod.push_slice(&output_frame);
        }
    }
}
