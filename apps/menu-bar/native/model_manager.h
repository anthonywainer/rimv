#import <AppKit/AppKit.h>

typedef void (*RimvModelCommandCallback)(uint32_t, uint8_t);

void rimv_model_manager_configure(const char *catalog_json, const char *storage_path,
                                  RimvModelCommandCallback command);
void rimv_model_manager_update(const char *catalog_json);
void rimv_model_manager_show_selector(NSView *anchor);
bool rimv_model_manager_selector_contains_window(NSWindow *window);
bool rimv_model_manager_selector_is_shown(void);
void rimv_model_manager_close_selector(void);
