use biquad::*;
use fdaf_aec::FdafAec;
use nnnoiseless::DenoiseState;
use ringbuf::{
    HeapCons, HeapProd, LocalRb,
    storage::Heap,
    traits::{Consumer, Observer, Producer, Split},
};

use crate::constant::*;

fn sanitize(frame: &mut [f32]) {
    for x in frame.iter_mut() {
        let val = if x.is_finite() { *x } else { 0f32 };
        *x = val.clamp(-1.0, 1.0);
    }
}

// fusionable
fn sanitize_and_normalize_frame(frame: &mut [f32]) {
    for x in frame.iter_mut() {
        if !x.is_finite() {
            *x = 10e-6;
        }
        *x = x.clamp(-1.0, 1.0);
    }

    let _peak: f32 = frame.iter().map(|x| x.abs()).fold(0.0f32, |a, b| a.max(b));

    // TODO

    for x in frame.iter_mut() {
        *x = x.clamp(-1.0, 1.0);
    }
}

pub fn audio_processing(
    mut mic_cons: HeapCons<f32>,
    mut far_end_cons: HeapCons<f32>,
    mut processed_prod: HeapProd<f32>,
) {
    let coeffs =
        Coefficients::<f32>::from_params(Type::HighPass, 48000.hz(), 100.hz(), Q_BUTTERWORTH_F32)
            .expect("Failed to create coefficients");

    // state machine init
    let mut aec = FdafAec::<AEC_FFT_SIZE>::new(STEP_SIZE, 0.9, 10e-4);
    let mut denoise = DenoiseState::new();
    let mut mic_hpfilter = DirectForm2Transposed::<f32>::new(coeffs);
    let mut far_end_hpfilter = DirectForm2Transposed::<f32>::new(coeffs);
    let mut nlp_hpfilter = DirectForm2Transposed::<f32>::new(coeffs);

    // local ringbuffer
    let hpf_mic_to_aec = LocalRb::<Heap<f32>>::new(FRAME_SIZE.max(AEC_FRAME_SIZE) * 4);
    let (mut hpf_mic_prod, mut aec_mic_cons) = hpf_mic_to_aec.split();
    let hpf_far_end_to_aec = LocalRb::<Heap<f32>>::new(FRAME_SIZE.max(AEC_FRAME_SIZE) * 4);
    let (mut hpf_far_end_prod, mut aec_far_end_cons) = hpf_far_end_to_aec.split();

    let aec_to_nlp = LocalRb::<Heap<f32>>::new(FRAME_SIZE.max(AEC_FRAME_SIZE) * 4);
    let (mut aec_prod, mut nlp_cons) = aec_to_nlp.split();

    let nlp_to_ns = LocalRb::<Heap<f32>>::new(FRAME_SIZE.max(AEC_FRAME_SIZE) * 4);
    let (mut nlp_prod, mut ns_cons) = nlp_to_ns.split();

    // local buffers
    // hpf buffers
    let mut hpf_mic_frame = [0f32; FRAME_SIZE];
    let mut hpf_far_end_frame = [0f32; FRAME_SIZE];
    let mut nlp_frame = [0f32; FRAME_SIZE];
    // aec buffers
    let mut aec_mic_frame = [0f32; AEC_FRAME_SIZE];
    let mut aec_far_end_frame = [0f32; AEC_FRAME_SIZE];
    let mut aec_output_frame = [0f32; AEC_FRAME_SIZE];
    // ns buffers
    let mut ns_input_frame = [0.0; DenoiseState::FRAME_SIZE];
    let mut ns_output_frame = [0.0; DenoiseState::FRAME_SIZE];

    // signal process main loop
    loop {
        // pre mic input HighPassFilter
        while mic_cons.occupied_len() >= FRAME_SIZE && hpf_mic_prod.vacant_len() >= FRAME_SIZE {
            mic_cons.pop_slice(&mut hpf_mic_frame);
            // sanitize(&mut hpf_mic_frame);
            for i in hpf_mic_frame.iter_mut() {
                *i = mic_hpfilter.run(*i);
            }
            hpf_mic_prod.push_slice(&hpf_mic_frame);
        }
        // pre far end HighPassFilter
        // FIXME: move to output thread
        while far_end_cons.occupied_len() >= FRAME_SIZE
            && hpf_far_end_prod.vacant_len() >= FRAME_SIZE
        {
            far_end_cons.pop_slice(&mut hpf_far_end_frame);
            // sanitize(&mut hpf_far_end_frame);
            for i in hpf_far_end_frame.iter_mut() {
                *i = far_end_hpfilter.run(*i);
            }
            hpf_far_end_prod.push_slice(&hpf_far_end_frame);
        }

        // aec (echo cancell)
        while aec_mic_cons.occupied_len() >= AEC_FRAME_SIZE
            && aec_prod.vacant_len() >= AEC_FRAME_SIZE
        {
            aec_mic_cons.pop_slice(&mut aec_mic_frame);
            if aec_far_end_cons.occupied_len() >= AEC_FRAME_SIZE {
                aec_far_end_cons.pop_slice(&mut aec_far_end_frame);
            } else {
                aec_far_end_frame = [0.0; AEC_FRAME_SIZE];
            }

            // sanitize(&mut aec_mic_frame);

            aec.process(
                aec_output_frame
                    .first_chunk_mut::<AEC_FRAME_SIZE>()
                    .unwrap(),
                aec_far_end_frame.first_chunk().unwrap(),
                aec_mic_frame.first_chunk().unwrap(),
            );

            // sanitize(&mut aec_output_frame);
            println!("aec result: {:?}", aec_output_frame);

            // processed_prod.push_slice(&aec_output_frame);
            aec_prod.push_slice(&aec_output_frame);
        }

        // nlp filter
        while nlp_cons.occupied_len() >= FRAME_SIZE && nlp_prod.vacant_len() >= FRAME_SIZE {
            nlp_cons.pop_slice(&mut nlp_frame);
            for i in hpf_far_end_frame.iter_mut() {
                *i = nlp_hpfilter.run(*i);
            }
            nlp_prod.push_slice(&nlp_frame);
        }

        // noiseless
        while ns_cons.occupied_len() >= DenoiseState::FRAME_SIZE
            && processed_prod.vacant_len() >= DenoiseState::FRAME_SIZE
        {
            ns_cons.pop_slice(&mut ns_input_frame);
            for i in ns_input_frame.iter_mut() {
                *i *= 32767.0f32;
            }
            denoise.process_frame(&mut ns_output_frame, &ns_input_frame);
            for i in ns_output_frame.iter_mut() {
                *i /= 32767.0f32;
            }
            processed_prod.push_slice(&ns_output_frame);
        }
    }
}
