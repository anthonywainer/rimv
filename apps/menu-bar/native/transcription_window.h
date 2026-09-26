#import <AppKit/AppKit.h>
typedef void (*RimvTranscriptionCommandCallback)(uint32_t, uint8_t);
void rimv_transcription_window_configure(RimvTranscriptionCommandCallback command);
void rimv_transcription_window_snapshot(const char *snapshot_json);
void rimv_transcription_window_update(const char *update_json);
void rimv_transcription_window_open(void);
void rimv_transcription_window_open_live(const char *snapshot_json, const char *transcript_state_json);
void rimv_transcription_window_open_session(const char *path);
void rimv_transcription_window_recording_metadata(const char *path, const char *title, uint8_t deleted);
void rimv_transcription_window_close_session(const char *path);
void rimv_transcription_window_shutdown_playback(void);
