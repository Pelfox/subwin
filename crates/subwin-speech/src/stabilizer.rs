use crate::CaptionSegment;

#[derive(Debug, Clone, Default)]
pub struct CaptionUpdate {
    pub history: Vec<CaptionSegment>,
    pub active: Vec<CaptionSegment>,
}

#[derive(Debug, Clone)]
pub struct CaptionsStabilizer {
    tail_ms: i64,           // how much of recent audio can be changed
    dedupe_fuzz_ms: i64, // how close two captions can be together to be counted as the same thing
    last_final_end_ms: i64, // the end timestamp of the most recently finalized caption
}

impl CaptionsStabilizer {
    pub fn new(tail_ms: i64) -> Self {
        Self {
            tail_ms,
            dedupe_fuzz_ms: 80,
            last_final_end_ms: 0,
        }
    }

    pub fn push(
        &mut self,
        now_milliseconds: i64,
        mut segments: Vec<CaptionSegment>,
    ) -> CaptionUpdate {
        let cutoff_ms = now_milliseconds - self.tail_ms;
        segments.sort_by_key(|segment| (segment.start_milliseconds, segment.end_milliseconds));

        let mut update = CaptionUpdate::default();
        for segment in segments {
            // Drop non-speech junk like [BLANK_AUDIO]
            if segment.text.starts_with('[') && segment.text.ends_with(']') {
                continue;
            }

            if segment.end_milliseconds <= cutoff_ms {
                // Candidate for finalization
                if segment.end_milliseconds <= self.last_final_end_ms + self.dedupe_fuzz_ms {
                    continue; // overlap / duplicate
                }

                self.last_final_end_ms = self.last_final_end_ms.max(segment.end_milliseconds);
                update.history.push(segment);
            } else {
                // Still live
                update.active.push(segment);
            }
        }

        update
    }
}
