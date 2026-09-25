#import <AppKit/AppKit.h>

typedef void (*RimvModelCommandCallback)(uint32_t, uint8_t);

void rimv_model_manager_configure(const char *catalog_json, const char *storage_path,
                                  RimvModelCommandCallback command);
void rimv_model_manager_update(const char *catalog_json);
void rimv_model_manager_show_selector(NSView *anchor);
