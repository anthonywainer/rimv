export type EngineStatus = "idle" | "starting" | "recording" | "stopping" | "error";
export type Source = "microphone" | "system";

export interface AsrMetrics { inferences: number; coalesced_work: number; dropped_work: number; dropped_events: number; vad_segments: number; average_inference_ms: number; average_rtf_milli: number; }
export interface FeatureState { enabled: boolean; available: boolean; status: "disabled" | "loading" | "ready" | "transcribing" | "error"; dropped_blocks: number; asr?: AsrMetrics; backend?: string; model?: string; supports_partial_results?: boolean; supports_true_streaming?: boolean; }
export interface EngineSnapshot { status: EngineStatus; elapsed_ms: number; microphone: { enabled: boolean; active: boolean }; system_audio: { enabled: boolean; active: boolean }; transcription: FeatureState; }
export interface TranscriptSegment { source: Source; start_ms: number; end_ms: number; text: string; }
export interface TranscriptUpdate { source: Source; utterance_id: string; start_ms: number; end_ms: number; stable_text: string; unstable_text: string; is_final: boolean; language?: string; confidence?: string; }
export type EngineCommand = { type: "start_capture" } | { type: "stop_capture" } | { type: "set_microphone_enabled"; enabled: boolean } | { type: "set_system_audio_enabled"; enabled: boolean } | { type: "set_transcription_enabled"; enabled: boolean } | { type: "get_state" };
export type EngineEvent = { type: "snapshot"; snapshot: EngineSnapshot } | { type: "transcript_update"; update: TranscriptUpdate } | { type: "transcript_partial"; segment: TranscriptSegment } | { type: "transcript_final"; segment: TranscriptSegment } | { type: "transcription_error"; error: { message: string } } | { type: "error"; error: { message: string } };
