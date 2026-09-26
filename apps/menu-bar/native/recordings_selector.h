#import <AppKit/AppKit.h>
void rimv_recordings_selector_update(const char *json);
void rimv_recordings_selector_show(NSView *anchor);
bool rimv_recordings_selector_contains_window(NSWindow *window);
bool rimv_recordings_selector_is_shown(void);
void rimv_recordings_selector_close(void);
