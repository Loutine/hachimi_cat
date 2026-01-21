use crate::{AudioProcessor, FRAME10MS};

pub struct EmptyAudioProcessor {}

impl EmptyAudioProcessor {
    pub fn build() -> anyhow::Result<Self> {
        Ok(EmptyAudioProcessor {})
    }
}
impl AudioProcessor for EmptyAudioProcessor {
    fn process(
        &mut self,
        mic_cons: &mut rtrb::Consumer<f32>,
        ref_cons: &mut rtrb::Consumer<f32>,
        mic_prod: &mut rtrb::Producer<f32>,
        ref_prod: &mut rtrb::Producer<f32>,
    ) {
        while let (Ok(mic_cons), Ok(ref_cons), Ok(mut mic_prod), Ok(mut ref_prod)) = (
            mic_cons.read_chunk(FRAME10MS),
            ref_cons.read_chunk(FRAME10MS),
            mic_prod.write_chunk(FRAME10MS),
            ref_prod.write_chunk(FRAME10MS),
        ) {
            ref_prod
                .as_mut_slices()
                .0
                .copy_from_slice(ref_cons.as_slices().0);
            ref_cons.commit_all();
            ref_prod.commit_all();
            mic_prod
                .as_mut_slices()
                .0
                .copy_from_slice(mic_cons.as_slices().0);
            mic_cons.commit_all();
            mic_prod.commit_all();
        }
    }
}
