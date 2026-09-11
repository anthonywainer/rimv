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

/// Agreement across consecutive word hypotheses. Committed words never change;
/// a conflicting later hypothesis remains uncommitted until agreement returns.
#[derive(Default)]
pub struct TranscriptStabilizer {
    stable: Vec<String>,
    previous: Vec<String>,
}
impl TranscriptStabilizer {
    pub fn update(&mut self, text: &str, final_result: bool) -> (String, String) {
        let words: Vec<String> = text.split_whitespace().map(str::to_owned).collect();
        if words.starts_with(&self.stable) {
            let agreement = words
                .iter()
                .zip(&self.previous)
                .take_while(|(a, b)| a == b)
                .count();
            let committed = if final_result {
                words.len()
            } else {
                agreement.max(self.stable.len())
            };
            self.stable = words[..committed].to_vec();
            self.previous = words.clone();
            (self.stable.join(" "), words[committed..].join(" "))
        } else {
            // Preserve the committed prefix; revise only the remaining suffix.
            let suffix = words.get(self.stable.len()..).unwrap_or_default();
            if final_result {
                self.stable.extend_from_slice(suffix);
            }
            self.previous = words.clone();
            (
                self.stable.join(" "),
                if final_result {
                    String::new()
                } else {
                    suffix.join(" ")
                },
            )
        }
    }
}

struct Work {
    source: AudioSource,
    audio: Utterance,
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
                        if state.0 != work.audio.start_ms {
                            let next_id =
                                NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            *state = (
                                work.audio.start_ms,
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
        q.push(
            Work {
                source,
                audio: Utterance {
                    start_ms,
                    end_ms: start_ms + samples.len() as u64 * 1000 / ASR_SAMPLE_RATE as u64,
                    samples: samples.to_vec(),
                },
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
    fn conflicting_hypothesis_cannot_rewrite_committed_words() {
        let mut s = TranscriptStabilizer::default();
        s.update("hello world", false);
        s.update("hello world", false);
        assert_eq!(s.update("goodbye", true), ("hello world".into(), "".into()));
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
