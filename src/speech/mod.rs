pub mod whisper;

pub const CONTEXT_LENGTH_MILLISECONDS: u32 = 3000; // the size of context in milliseconds 
pub const REPEAT_RUN_MILLISECONDS: u32 = 500; // how often to run the transcoding

pub(crate) fn milliseconds_to_samples(milliseconds: u32, sample_rate: u32) -> usize {
    ((sample_rate * milliseconds) / 1000) as usize
}

pub trait Transcoder<P> {
    fn min_transcode_samples(sample_rate: u32) -> usize;

    fn accept_samples(&mut self, samples: &[f32]);
    fn try_transcode(&mut self, params: P) -> Option<String>;
}
