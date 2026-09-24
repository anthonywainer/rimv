use crate::{
    ASR_SAMPLE_RATE, SpeechEvent, SpeechMetrics, SpeechToTextEngine, TranscriptUpdate, Utterance,
};
use engine_protocol::AudioSource;
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Condvar, Mutex, mpsc::SyncSender},
    thread::{self, JoinHandle},
    time::Instant,
};

const FINAL_CAPACITY: usize = 8;
const MIN_PARTIAL_SAMPLES: usize = ASR_SAMPLE_RATE as usize / 2;
const PARTIAL_STEP_SAMPLES: usize = ASR_SAMPLE_RATE as usize / 2;
/// Partial hypotheses need only recent acoustic context. Keeping this bounded
/// avoids repeatedly decoding an ever-growing VAD utterance; final work still
/// receives the entire utterance below.
const PARTIAL_WINDOW_SAMPLES: usize = ASR_SAMPLE_RATE as usize * 10;

/// Agreement across consecutive word hypotheses. Committed words never change;
/// a conflicting later hypothesis remains uncommitted until agreement returns.
#[derive(Default)]
pub struct TranscriptStabilizer {
    stable: Vec<String>,
    previous: Vec<String>,
    // The tail is deliberately retained if a later rolling-window hypothesis
    // no longer contains the committed prefix.  Indexing that unrelated
    // hypothesis by `stable.len()` used to replace a valid live tail with an
    // arbitrary suffix (and could make most of the display vanish).
    provisional: Vec<String>,
}
impl TranscriptStabilizer {
    pub fn update(&mut self, text: &str, final_result: bool) -> (String, String) {
        let words: Vec<String> = text.split_whitespace().map(str::to_owned).collect();
        // A final decode covers the complete VAD utterance. It may revise an
        // earlier cumulative hypothesis, so combining it with a partial prefix
        // can manufacture repeated or obsolete clauses in persisted output.
        if final_result {
            self.stable = words.clone();
            self.previous = words;
            self.provisional.clear();
            return (self.stable.join(" "), String::new());
        }
        if words.starts_with(&self.stable) {
            // Before any words have been confirmed, a bounded input window may
            // advance past the first word. Merge its overlap with the visible
            // provisional text instead of replacing the whole caption.
            if self.stable.is_empty()
                && !self.previous.is_empty()
                && !words.starts_with(&self.previous)
                && self.extend_provisional_from_overlap(&words)
            {
                self.previous = words;
                return (self.stable.join(" "), self.provisional.join(" "));
            }
            let agreement = words
                .iter()
                .zip(&self.previous)
                .take_while(|(a, b)| a == b)
                .count();
            let committed = agreement.max(self.stable.len());
            self.stable = words[..committed].to_vec();
            self.previous = words.clone();
            self.provisional = words[committed..].to_vec();
            (self.stable.join(" "), self.provisional.join(" "))
        } else {
            // A rolling ASR window may temporarily omit its beginning. It is
            // not position-compatible with the committed prefix. Preserve
            // that prefix, but extend/revise the provisional tail when the new
            // window begins at a later point in the visible hypothesis. This
            // keeps live words moving without ever treating a partial as final.
            self.extend_provisional_from_overlap(&words);
            self.previous = words.clone();
            (self.stable.join(" "), self.provisional.join(" "))
        }
    }

    fn extend_provisional_from_overlap(&mut self, words: &[String]) -> bool {
        let visible = self
            .stable
            .iter()
            .chain(&self.provisional)
            .cloned()
            .collect::<Vec<_>>();
        let Some((start, overlap)) = longest_visible_prefix_overlap(&visible, words) else {
            return false;
        };
        if start + overlap < self.stable.len() {
            return false;
        }
        self.provisional = visible[self.stable.len()..start + overlap].to_vec();
        self.provisional.extend_from_slice(&words[overlap..]);
        true
    }
}

/// Finds a useful prefix of a later rolling-window hypothesis inside the text
/// already displayed. A three-word minimum avoids joining on a coincidental
/// short phrase; comparisons ignore casing and punctuation only for matching.
fn longest_visible_prefix_overlap(visible: &[String], current: &[String]) -> Option<(usize, usize)> {
    let mut best = None;
    for start in 0..visible.len() {
        let length = visible[start..]
            .iter()
            .zip(current)
            .take_while(|(left, right)| comparable_word(left) == comparable_word(right))
            .count();
        if length >= 3 && best.is_none_or(|(_, best_length)| length > best_length) {
            best = Some((start, length));
        }
    }
    best
}

fn comparable_word(word: &str) -> String {
    word.chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

struct Work {
    source: AudioSource,
    audio: Utterance,
    /// The VAD utterance identity is independent from a bounded partial
    /// window's input timestamp.
    utterance_start_ms: u64,
    final_result: bool,
}
#[derive(Default)]
struct Queue {
    finals: VecDeque<Work>,
    partials: VecDeque<Work>, // at most one per source (two audio sources)
    last_partial: HashMap<AudioSource, (u64, usize)>,
    closed: bool,
}
impl Queue {
    fn push(&mut self, work: Work, metrics: &mut SpeechMetrics) {
        let before = self.partials.len();
        self.partials.retain(|old| old.source != work.source);
        metrics.coalesced_work += (before - self.partials.len()) as u64;
        if work.final_result {
            metrics.vad_segments += 1;
            self.last_partial.remove(&work.source);
            if self.finals.len() == FINAL_CAPACITY {
                metrics.dropped_work += 1;
            } else {
                self.finals.push_back(work);
            }
        } else {
            self.partials.push_back(work);
        }
        metrics.queued_work = self.finals.len() + self.partials.len();
    }
}

pub(crate) struct Decoder {
    queue: Arc<(Mutex<Queue>, Condvar)>,
    metrics: Arc<Mutex<SpeechMetrics>>,
    join: JoinHandle<()>,
}
impl Decoder {
    pub fn start(
        mut engine: Box<dyn SpeechToTextEngine>,
        events: SyncSender<SpeechEvent>,
        metrics: Arc<Mutex<SpeechMetrics>>,
        final_overflow: Arc<Mutex<VecDeque<TranscriptUpdate>>>,
    ) -> Self {
        let queue = Arc::new((Mutex::new(Queue::default()), Condvar::new()));
        let shared = queue.clone();
        let measured = metrics.clone();
        let join = thread::spawn(move || {
            let mut stabilizers =
                HashMap::<AudioSource, (u64, String, TranscriptStabilizer)>::new();
            static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
            let mut last_partial: Option<Instant> = None;
            loop {
                let (mutex, wake) = &*shared;
                let mut q = mutex.lock().unwrap();
                while q.finals.is_empty() && q.partials.is_empty() && !q.closed {
                    q = wake.wait(q).unwrap();
                }
                // Wall-clock throttle also applies when processing an audio backlog.
                // Final work wakes this wait and is never delayed by partial cadence.
                if q.finals.is_empty()
                    && !q.partials.is_empty()
                    && let Some(last) = last_partial
                    && let Some(remaining) =
                        std::time::Duration::from_millis(500).checked_sub(last.elapsed())
                {
                    let (guard, _) = wake.wait_timeout(q, remaining).unwrap();
                    drop(guard);
                    continue;
                }
                let work = q.finals.pop_front().or_else(|| q.partials.pop_front());
                measured.lock().unwrap().queued_work = q.finals.len() + q.partials.len();
                let Some(work) = work else {
                    break;
                };
                drop(q);
                let started = Instant::now();
                if !work.final_result {
                    last_partial = Some(started);
                }
                let result = engine.transcribe(&work.audio.samples, work.audio.start_ms);
                let elapsed = started.elapsed();
                let mut m = measured.lock().unwrap();
                let ms = elapsed.as_millis() as u64;
                m.inferences += 1;
                m.inference_total_ms += ms;
                m.inference_average_ms = m.inference_total_ms as f64 / m.inferences as f64;
                m.inference_max_ms = m.inference_max_ms.max(ms);
                m.first_latency_ms.get_or_insert(ms);
                let rtf = elapsed.as_secs_f64()
                    / (work.audio.samples.len() as f64 / ASR_SAMPLE_RATE as f64);
                m.rtf_total += rtf;
                m.rtf_max = m.rtf_max.max(rtf);
                drop(m);
                match result {
                    Ok(segments) => {
                        let state = stabilizers.entry(work.source).or_insert_with(|| {
                            let next_id =
                                NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            (
                                work.audio.start_ms,
                                format!("utterance-{next_id}"),
                                TranscriptStabilizer::default(),
                            )
                        });
                        if state.0 != work.utterance_start_ms {
                            let next_id =
                                NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            *state = (
                                work.utterance_start_ms,
                                format!("utterance-{next_id}"),
                                TranscriptStabilizer::default(),
                            );
                        }
                        let text = segments
                            .iter()
                            .map(|s| s.text.as_str())
                            .collect::<Vec<_>>()
                            .join(" ");
                        let (stable_text, unstable_text) = state.2.update(&text, work.final_result);
                        let update = TranscriptUpdate {
                            source: work.source,
                            utterance_id: state.1.clone(),
                            start_ms: work.audio.start_ms,
                            end_ms: work.audio.end_ms,
                            stable_text,
                            unstable_text,
                            is_final: work.final_result,
                            language: None,
                            confidence: None,
                        };
                        if let Err(
                            std::sync::mpsc::TrySendError::Full(SpeechEvent::Update(update))
                            | std::sync::mpsc::TrySendError::Disconnected(SpeechEvent::Update(
                                update,
                            )),
                        ) = events.try_send(SpeechEvent::Update(update))
                        {
                            let mut overflow = final_overflow.lock().unwrap();
                            if update.is_final && overflow.len() < FINAL_CAPACITY {
                                overflow.push_back(update);
                            } else {
                                measured.lock().unwrap().dropped_events += 1;
                            }
                        }
                        if work.final_result {
                            stabilizers.remove(&work.source);
                        }
                    }
                    Err(error) => {
                        if events.try_send(SpeechEvent::Error(error)).is_err() {
                            measured.lock().unwrap().dropped_events += 1;
                        }
                    }
                }
            }
        });
        Self {
            queue,
            metrics,
            join,
        }
    }
    pub fn submit(&self, source: AudioSource, audio: Utterance, final_result: bool) {
        if audio.samples.is_empty() {
            return;
        }
        let (mutex, wake) = &*self.queue;
        mutex.lock().unwrap().push(
            Work {
                source,
                utterance_start_ms: audio.start_ms,
                audio,
                final_result,
            },
            &mut self.metrics.lock().unwrap(),
        );
        wake.notify_one();
    }
    pub fn partial(&self, source: AudioSource, start_ms: u64, samples: &[f32]) {
        if samples.len() < MIN_PARTIAL_SAMPLES {
            return;
        }
        let mut q = self.queue.0.lock().unwrap();
        if let Some(&(start, length)) = q.last_partial.get(&source)
            && start == start_ms
            && samples.len().saturating_sub(length) < PARTIAL_STEP_SAMPLES
        {
            return;
        }
        q.last_partial.insert(source, (start_ms, samples.len()));
        let audio = bounded_partial_audio(start_ms, samples);
        q.push(
            Work {
                source,
                audio,
                utterance_start_ms: start_ms,
                final_result: false,
            },
            &mut self.metrics.lock().unwrap(),
        );
        self.queue.1.notify_one();
    }
    pub fn finish(self) {
        self.queue.0.lock().unwrap().closed = true;
        self.queue.1.notify_one();
        let _ = self.join.join();
    }
}

fn bounded_partial_audio(utterance_start_ms: u64, samples: &[f32]) -> Utterance {
    let window_start_samples = samples.len().saturating_sub(PARTIAL_WINDOW_SAMPLES);
    let window_start_ms = utterance_start_ms
        + window_start_samples as u64 * 1000 / ASR_SAMPLE_RATE as u64;
    Utterance {
        start_ms: window_start_ms,
        end_ms: utterance_start_ms + samples.len() as u64 * 1000 / ASR_SAMPLE_RATE as u64,
        samples: samples[window_start_samples..].to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn work(source: AudioSource, final_result: bool) -> Work {
        Work {
            source,
            audio: Utterance {
                start_ms: 0,
                end_ms: 500,
                samples: vec![0.; 8000],
            },
            utterance_start_ms: 0,
            final_result,
        }
    }
    #[test]
    fn stable_prefix_only_grows_and_final_has_no_mutable_text() {
        let mut s = TranscriptStabilizer::default();
        assert_eq!(
            s.update("hello world", false),
            ("".into(), "hello world".into())
        );
        assert_eq!(
            s.update("hello there", false),
            ("hello".into(), "there".into())
        );
        assert_eq!(
            s.update("hello there friend", false),
            ("hello there".into(), "friend".into())
        );
        assert_eq!(
            s.update("hello there friends", true),
            ("hello there friends".into(), "".into())
        );
    }

    #[test]
    fn partial_input_stops_growing_after_ten_seconds() {
        for seconds in [5_u64, 10, 15, 20, 25, 30] {
            let input = bounded_partial_audio(
                3_000,
                &vec![0.0; seconds as usize * ASR_SAMPLE_RATE as usize],
            );
            assert_eq!(
                input.samples.len(),
                seconds.min(10) as usize * ASR_SAMPLE_RATE as usize,
                "{seconds}s active utterance",
            );
            assert_eq!(input.end_ms, 3_000 + seconds * 1_000);
            assert_eq!(input.start_ms, 3_000 + seconds.saturating_sub(10) * 1_000);
        }
    }

    #[test]
    fn bounded_partial_windows_append_without_duplicate_overlap() {
        let mut s = TranscriptStabilizer::default();
        assert_eq!(
            s.update("one two three four five six seven eight", false),
            (String::new(), "one two three four five six seven eight".into())
        );
        assert_eq!(
            s.update("four five six seven eight nine ten", false),
            (
                String::new(),
                "one two three four five six seven eight nine ten".into()
            )
        );
        assert_eq!(
            s.update("seven eight nine ten eleven", false),
            (
                String::new(),
                "one two three four five six seven eight nine ten eleven".into()
            )
        );
    }

    #[test]
    fn final_decode_keeps_the_complete_utterance() {
        use std::sync::mpsc;

        struct CaptureLengths(Arc<Mutex<Vec<usize>>>);
        impl SpeechToTextEngine for CaptureLengths {
            fn info(&self) -> crate::AsrBackendInfo {
                crate::MockAsrBackend::default().info()
            }

            fn transcribe(
                &mut self,
                audio: &[f32],
                offset_ms: u64,
            ) -> crate::Result<Vec<crate::SpeechSegment>> {
                self.0.lock().unwrap().push(audio.len());
                Ok(vec![crate::SpeechSegment {
                    source: AudioSource::Microphone,
                    start_ms: offset_ms,
                    end_ms: offset_ms + audio.len() as u64 * 1_000 / ASR_SAMPLE_RATE as u64,
                    text: "final words".into(),
                }])
            }
        }

        let lengths = Arc::new(Mutex::new(Vec::new()));
        let (events, _received) = mpsc::sync_channel(4);
        let decoder = Decoder::start(
            Box::new(CaptureLengths(lengths.clone())),
            events,
            Arc::new(Mutex::new(SpeechMetrics::default())),
            Arc::new(Mutex::new(VecDeque::new())),
        );
        decoder.submit(
            AudioSource::Microphone,
            Utterance {
                start_ms: 0,
                end_ms: 30_000,
                samples: vec![0.0; 30 * ASR_SAMPLE_RATE as usize],
            },
            true,
        );
        decoder.finish();
        assert_eq!(*lengths.lock().unwrap(), vec![30 * ASR_SAMPLE_RATE as usize]);
    }
    #[test]
    fn final_hypothesis_replaces_mismatched_partial_without_duplicate_clause() {
        let mut s = TranscriptStabilizer::default();
        s.update("the warning includes heavy rain and landslides", false);
        s.update("the warning includes heavy rain and landslides", false);
        assert_eq!(
            s.update("the warning includes heavy rain", true),
            ("the warning includes heavy rain".into(), "".into())
        );
    }

    #[test]
    fn final_hypothesis_does_not_append_revised_suffix_to_partial() {
        let mut s = TranscriptStabilizer::default();
        s.update("opening middle repeated clause", false);
        s.update("opening middle repeated clause", false);
        assert_eq!(
            s.update("opening middle corrected ending", true),
            ("opening middle corrected ending".into(), "".into())
        );
    }
    #[test]
    fn later_rolling_partial_extends_the_provisional_tail_without_erasing_it() {
        let mut s = TranscriptStabilizer::default();
        s.update("hello welcome to the news", false);
        assert_eq!(
            s.update("hello welcome to the news tonight", false),
            ("hello welcome to the news".into(), "tonight".into())
        );
        // This is a new rolling-window start. Its overlapping text lets the
        // provisional display advance without changing committed words.
        assert_eq!(
            s.update("welcome to the news tonight with more", false),
            ("hello welcome to the news".into(), "tonight with more".into())
        );
    }

    #[test]
    fn unrelated_partial_cannot_erase_the_provisional_tail() {
        let mut s = TranscriptStabilizer::default();
        s.update("hello welcome to the news", false);
        s.update("hello welcome to the news tonight", false);
        assert_eq!(
            s.update("unrelated words from a bad hypothesis", false),
            ("hello welcome to the news".into(), "tonight".into())
        );
    }
    #[test]
    fn overload_coalesces_partials_and_bounds_finals() {
        let mut q = Queue::default();
        let mut m = SpeechMetrics::default();
        for _ in 0..100 {
            q.push(work(AudioSource::Microphone, false), &mut m);
        }
        assert_eq!(q.partials.len(), 1);
        assert_eq!(m.coalesced_work, 99);
        for _ in 0..10 {
            q.push(work(AudioSource::Microphone, true), &mut m);
        }
        assert!(q.partials.is_empty());
        assert_eq!(q.finals.len(), FINAL_CAPACITY);
        assert_eq!(m.dropped_work, 2);
    }
    #[test]
    fn slow_decode_does_not_block_submission_and_final_replaces_pending_partial() {
        use std::sync::mpsc;
        use std::time::Duration;
        struct Slow {
            entered: mpsc::SyncSender<()>,
            release: mpsc::Receiver<()>,
            calls: usize,
        }
        impl SpeechToTextEngine for Slow {
            fn info(&self) -> crate::AsrBackendInfo {
                crate::MockAsrBackend::default().info()
            }
            fn transcribe(
                &mut self,
                _: &[f32],
                offset: u64,
            ) -> crate::Result<Vec<crate::SpeechSegment>> {
                self.calls += 1;
                if self.calls == 1 {
                    self.entered.send(()).unwrap();
                    self.release.recv_timeout(Duration::from_secs(5)).unwrap();
                }
                Ok(vec![crate::SpeechSegment {
                    source: AudioSource::Microphone,
                    start_ms: offset,
                    end_ms: offset + 500,
                    text: "hello world".into(),
                }])
            }
        }
        let (entered, wait) = mpsc::sync_channel(1);
        let (release, blocked) = mpsc::sync_channel(1);
        let (events, received) = mpsc::sync_channel(1);
        let metrics = Arc::new(Mutex::new(SpeechMetrics::default()));
        let overflow = Arc::new(Mutex::new(VecDeque::new()));
        let decoder = Decoder::start(
            Box::new(Slow {
                entered,
                release: blocked,
                calls: 0,
            }),
            events,
            metrics.clone(),
            overflow.clone(),
        );
        decoder.partial(AudioSource::Microphone, 0, &[0.; 7999]);
        assert_eq!(metrics.lock().unwrap().queued_work, 0);
        decoder.partial(AudioSource::Microphone, 0, &[0.; 8000]);
        wait.recv_timeout(Duration::from_secs(5)).unwrap();
        // Decoder remains blocked until release; submission must still complete.
        decoder.partial(AudioSource::Microphone, 0, &[0.; 16000]);
        decoder.partial(AudioSource::Microphone, 0, &[0.; 24000]);
        decoder.submit(
            AudioSource::Microphone,
            work(AudioSource::Microphone, true).audio,
            true,
        );
        decoder.submit(
            AudioSource::System,
            work(AudioSource::System, true).audio,
            true,
        );
        release.send(()).unwrap();
        decoder.finish();
        let SpeechEvent::Update(partial) = received.recv().unwrap() else {
            panic!("partial expected")
        };
        assert!(!partial.is_final);
        let finals = overflow.lock().unwrap();
        assert_eq!(finals.len(), 2);
        assert!(
            finals
                .iter()
                .all(|u| u.is_final && u.unstable_text.is_empty())
        );
        assert_eq!(finals[0].utterance_id, partial.utterance_id);
        assert_ne!(finals[0].utterance_id, finals[1].utterance_id);
        assert_eq!(finals[1].source, AudioSource::System);
        let m = metrics.lock().unwrap();
        assert_eq!(m.inferences, 3);
        assert_eq!(m.coalesced_work, 2);
        assert_eq!(m.dropped_events, 0);
        assert_eq!(m.queued_work, 0);
    }
}
