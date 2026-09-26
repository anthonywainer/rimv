#import "model_manager.h"

static BOOL MMIsDark(NSAppearance *appearance) {
    return [[appearance bestMatchFromAppearancesWithNames:@[NSAppearanceNameAqua, NSAppearanceNameDarkAqua]]
        isEqualToString:NSAppearanceNameDarkAqua];
}

extern void rimv_menu_selector_did_close(NSInteger);
extern bool rimv_menu_release_popover_focus(NSWindow *, NSInteger);
@interface RimvModelManager : NSObject <NSWindowDelegate, NSPopoverDelegate>
@property(nonatomic, copy) NSArray<NSDictionary *> *models;
@property(nonatomic, copy) NSString *storagePath;
@property(nonatomic) RimvModelCommandCallback command;
@property(nonatomic, strong) NSPopover *selector;
@property(nonatomic, strong) NSWindow *window;
@property(nonatomic, strong) NSView *content;
@property(nonatomic, copy) NSString *tab;
@end

@implementation RimvModelManager
- (BOOL)releaseSelectorFocus {
    NSWindow *window = self.selector.contentViewController.view.window;
    if (!self.selector.shown) return YES;
    return rimv_menu_release_popover_focus(window, 2);
}
- (BOOL)closeSelectorSafely {
    if (!self.selector.shown) return YES;
    if (![self releaseSelectorFocus]) return NO;
    [self.selector performClose:nil];
    return YES;
}
- (NSColor *)color:(CGFloat)lr :(CGFloat)lg :(CGFloat)lb dark:(CGFloat)dr :(CGFloat)dg :(CGFloat)db {
    BOOL dark = MMIsDark(self.content.effectiveAppearance ?: NSApp.effectiveAppearance);
    return [NSColor colorWithRed:(dark ? dr : lr) / 255.0 green:(dark ? dg : lg) / 255.0 blue:(dark ? db : lb) / 255.0 alpha:1];
}
- (NSTextField *)label:(NSString *)text frame:(NSRect)frame size:(CGFloat)size weight:(NSFontWeight)weight color:(NSColor *)color in:(NSView *)parent {
    NSTextField *label = [NSTextField labelWithString:text ?: @""];
    label.frame = frame; label.font = [NSFont systemFontOfSize:size weight:weight]; label.textColor = color;
    label.lineBreakMode = NSLineBreakByTruncatingTail; [parent addSubview:label]; return label;
}
- (NSInteger)catalogIndex:(NSString *)identifier {
    return [self.models indexOfObjectPassingTest:^BOOL(NSDictionary *entry, NSUInteger index, BOOL *stop) {
        (void)index; (void)stop; return [entry[@"id"] isEqual:identifier];
    }];
}
- (void)send:(NSInteger)tag {
    NSInteger op = tag / 100, index = tag % 100;
    if (index < 0 || index >= (NSInteger)self.models.count) return;
    NSDictionary *model = self.models[index]; NSString *identifier = model[@"id"];
    NSDictionary *download = @{@"parakeet-tdt-0.6b-v3-int8":@40, @"whisper-tiny":@41, @"whisper-base":@42,
        @"whisper-small":@43, @"whisper-medium":@44, @"whisper-large":@45, @"whisper-turbo":@46};
    NSNumber *code = download[identifier]; if (!code) return;
    if (op == 1) self.command((uint32_t)code.integerValue, 0);
    if (op == 2) self.command((uint32_t)(code.integerValue + 20), 0);
    if (op == 3) self.command((uint32_t)(code.integerValue + 40), 0);
}
- (void)action:(NSButton *)sender { [self send:sender.tag]; }
- (void)confirmRemove:(NSButton *)sender {
    NSInteger index = sender.tag % 100; if (index < 0 || index >= (NSInteger)self.models.count) return;
    NSAlert *alert = [[NSAlert alloc] init]; alert.messageText = @"Remove model?";
    alert.informativeText = [NSString stringWithFormat:@"%@ will be removed from local storage.", self.models[index][@"name"]];
    [alert addButtonWithTitle:@"Remove"]; [alert addButtonWithTitle:@"Cancel"];
    if ([alert runModal] == NSAlertFirstButtonReturn) [self send:sender.tag];
}
- (void)showSelector:(NSView *)anchor {
    // Selector transitions are serialized by RimvMenu. Never replace a
    // visible selector; doing so detaches the old delegate from the close
    // lifecycle and can leave the coordinator waiting on the wrong window.
    if (self.selector.shown) return;
    NSView *view = [[NSView alloc] initWithFrame:NSMakeRect(0, 0, 330, 0)];
    NSArray *installed = [self.models filteredArrayUsingPredicate:[NSPredicate predicateWithBlock:^BOOL(NSDictionary *m, NSDictionary *b) { (void)b; return [m[@"state"] isEqual:@"installed"]; }]];
    CGFloat height = installed.count ? MIN(330, 74 + installed.count * 46 + 42) : 166;
    view.frame = NSMakeRect(0, 0, 330, height);
    NSColor *primary = [self color:17 :20 :24 dark:244 :247 :250];
    [self label:@"Model" frame:NSMakeRect(18, height - 36, 160, 20) size:17 weight:NSFontWeightSemibold color:primary in:view];
    if (!installed.count) {
        [self label:@"No transcription models are installed." frame:NSMakeRect(18, height - 80, 294, 18) size:13 weight:NSFontWeightRegular color:[self color:107 :114 :128 dark:158 :168 :182] in:view];
        NSButton *download = [NSButton buttonWithTitle:@"Download Models…" target:self action:@selector(openManager:)];
        download.frame = NSMakeRect(18, 22, 150, 32); [view addSubview:download];
    } else {
        CGFloat y = height - 80;
        for (NSDictionary *model in installed) {
            NSInteger index = [self catalogIndex:model[@"id"]]; BOOL selected = [model[@"selected"] boolValue];
            NSButton *row = [NSButton buttonWithTitle:model[@"name"] target:self action:@selector(selectAndClose:)];
            row.tag = 200 + index; row.frame = NSMakeRect(10, y, 310, 40); row.bordered = NO; row.alignment = NSTextAlignmentLeft;
            row.image = [NSImage imageWithSystemSymbolName:(selected ? @"checkmark" : @"cpu") accessibilityDescription:nil]; row.imagePosition = NSImageLeading;
            [view addSubview:row]; y -= 46;
        }
        NSBox *line = [[NSBox alloc] initWithFrame:NSMakeRect(18, 35, 294, 1)]; line.boxType = NSBoxSeparator; [view addSubview:line];
        NSButton *manage = [NSButton buttonWithTitle:@"Manage Models…" target:self action:@selector(openManager:)]; manage.frame = NSMakeRect(14, 4, 160, 28); manage.bordered = NO; manage.alignment = NSTextAlignmentLeft; [view addSubview:manage];
    }
    NSViewController *controller = [[NSViewController alloc] init]; controller.view = view;
    self.selector = [[NSPopover alloc] init]; self.selector.delegate=self; self.selector.behavior = NSPopoverBehaviorApplicationDefined; self.selector.contentSize = view.bounds.size; self.selector.contentViewController = controller;
    [self.selector showRelativeToRect:anchor.bounds ofView:anchor preferredEdge:NSRectEdgeMaxX];
}
- (void)selectAndClose:(NSButton *)sender { [self send:sender.tag]; [self closeSelectorSafely]; }
- (void)popoverDidClose:(NSNotification *)notification {
    if (notification.object != self.selector) return;
    rimv_menu_selector_did_close(2);
}
- (void)openManager:(id)sender { (void)sender; if ([self closeSelectorSafely]) [self showWindow]; }
- (void)switchTab:(NSButton *)sender { self.tab = sender.identifier; [self render]; }
- (void)showWindow {
    if (!self.window) {
        self.window = [[NSWindow alloc] initWithContentRect:NSMakeRect(0, 0, 780, 620)
            styleMask:(NSWindowStyleMaskTitled | NSWindowStyleMaskClosable | NSWindowStyleMaskResizable)
            backing:NSBackingStoreBuffered defer:NO];
        self.window.title = @"Model Manager"; self.window.minSize = NSMakeSize(650, 480); self.window.delegate = self;
        self.content = self.window.contentView;
    }
    self.tab = self.tab ?: @"available"; [self render]; [NSApp activateIgnoringOtherApps:YES]; [self.window makeKeyAndOrderFront:nil];
}
- (void)render {
    if (!self.content) return; for (NSView *v in self.content.subviews.copy) [v removeFromSuperview];
    self.content.wantsLayer = YES; self.content.layer.backgroundColor = [self color:248 :250 :252 dark:16 :26 :43].CGColor;
    NSView *sidebar = [[NSView alloc] initWithFrame:NSMakeRect(0, 0, 190, self.content.bounds.size.height)]; sidebar.autoresizingMask = NSViewHeightSizable;
    sidebar.wantsLayer = YES; sidebar.layer.backgroundColor = [self color:243 :244 :246 dark:21 :38 :58].CGColor; [self.content addSubview:sidebar];
    NSArray *tabs = @[@[@"available", @"arrow.down.circle", @"Available Models"], @[@"installed", @"externaldrive", @"Installed Models"], @[@"settings", @"gearshape", @"Settings"]];
    CGFloat y = self.content.bounds.size.height - 75;
    for (NSArray *tab in tabs) { NSButton *button = [NSButton buttonWithTitle:tab[2] target:self action:@selector(switchTab:)]; button.identifier = tab[0]; button.frame = NSMakeRect(14, y, 162, 35); button.bordered = NO; button.alignment = NSTextAlignmentLeft; button.image = [NSImage imageWithSystemSymbolName:tab[1] accessibilityDescription:nil]; button.imagePosition = NSImageLeading; if ([self.tab isEqual:tab[0]]) { button.wantsLayer=YES; button.layer.cornerRadius=8; button.layer.backgroundColor=[self color:220 :245 :242 dark:17 :105 :110].CGColor; } [sidebar addSubview:button]; y -= 43; }
    NSView *area = [[NSView alloc] initWithFrame:NSMakeRect(210, 18, self.content.bounds.size.width-230, self.content.bounds.size.height-36)]; area.autoresizingMask = NSViewWidthSizable|NSViewHeightSizable; [self.content addSubview:area];
    NSColor *primary = [self color:17 :20 :24 dark:244 :247 :250]; NSColor *secondary = [self color:107 :114 :128 dark:158 :168 :182];
    if ([self.tab isEqual:@"settings"]) { [self label:@"Model Settings" frame:NSMakeRect(0, area.bounds.size.height-36, 300, 26) size:20 weight:NSFontWeightSemibold color:primary in:area]; [self label:@"Model storage" frame:NSMakeRect(0, area.bounds.size.height-86, 200, 18) size:14 weight:NSFontWeightMedium color:primary in:area]; [self label:self.storagePath frame:NSMakeRect(0, area.bounds.size.height-110, area.bounds.size.width-130, 18) size:12 weight:NSFontWeightRegular color:secondary in:area]; NSButton *open = [NSButton buttonWithTitle:@"Open Folder" target:self action:@selector(openFolder:)]; open.frame=NSMakeRect(area.bounds.size.width-110, area.bounds.size.height-118, 105, 28); [area addSubview:open]; return; }
    BOOL installedTab = [self.tab isEqual:@"installed"]; [self label:(installedTab ? @"Installed Models" : @"Available Models") frame:NSMakeRect(0, area.bounds.size.height-36, 330, 26) size:20 weight:NSFontWeightSemibold color:primary in:area]; [self label:(installedTab ? @"Choose a locally installed model." : @"Download and manage your transcription models.") frame:NSMakeRect(0, area.bounds.size.height-58, 380, 17) size:12 weight:NSFontWeightRegular color:secondary in:area];
    CGFloat yCard=area.bounds.size.height-155; NSInteger shown=0;
    for (NSInteger i=0;i<(NSInteger)self.models.count;i++) { NSDictionary *m=self.models[i]; BOOL installed=[m[@"state"] isEqual:@"installed"]; BOOL downloading=[m[@"state"] isEqual:@"downloading"]; if (installedTab && !installed) continue; shown++; NSView *card=[[NSView alloc] initWithFrame:NSMakeRect(0,yCard,area.bounds.size.width,98)]; card.autoresizingMask=NSViewWidthSizable; card.wantsLayer=YES; card.layer.cornerRadius=12; card.layer.borderWidth=1; card.layer.borderColor=[self color:229 :231 :235 dark:41 :63 :87].CGColor; card.layer.backgroundColor=[self color:255 :255 :255 dark:24 :40 :59].CGColor; [area addSubview:card]; [self label:m[@"name"] frame:NSMakeRect(15,62,260,19) size:15 weight:NSFontWeightSemibold color:primary in:card]; NSString *detail=(downloading && m[@"progress"] != [NSNull null]) ? m[@"progress"] : m[@"description"]; [self label:detail frame:NSMakeRect(15,38,MIN(360,card.bounds.size.width-150),18) size:12 weight:NSFontWeightRegular color:secondary in:card]; [self label:m[@"size"] frame:NSMakeRect(card.bounds.size.width-120,62,105,17) size:12 weight:NSFontWeightRegular color:secondary in:card].alignment=NSTextAlignmentRight; NSString *title=installed ? ([m[@"selected"] boolValue]?@"Selected":@"Use Model") : (downloading ? @"Downloading…" : @"Download"); NSButton *action=[NSButton buttonWithTitle:title target:self action:@selector(action:)]; action.enabled=!downloading; action.tag=(installed?200:100)+i; action.frame=NSMakeRect(card.bounds.size.width-110,15,95,28); action.autoresizingMask=NSViewMinXMargin; [card addSubview:action]; if(installed){ NSButton *remove=[NSButton buttonWithTitle:@"…" target:self action:@selector(confirmRemove:)]; remove.tag=300+i; remove.frame=NSMakeRect(card.bounds.size.width-145,15,28,28); remove.bordered=NO; remove.autoresizingMask=NSViewMinXMargin; [card addSubview:remove]; } yCard-=110; }
    if (!shown) [self label:@"No installed models yet. Download a model from Available Models." frame:NSMakeRect(0,yCard,430,24) size:14 weight:NSFontWeightRegular color:secondary in:area];
}
- (void)openFolder:(id)sender { (void)sender; if (self.storagePath.length) [[NSWorkspace sharedWorkspace] openURL:[NSURL fileURLWithPath:self.storagePath isDirectory:YES]]; }
@end

static RimvModelManager *manager;
static NSArray *parseCatalog(const char *json) { if (!json) return @[]; NSData *data=[NSData dataWithBytes:json length:strlen(json)]; id value=[NSJSONSerialization JSONObjectWithData:data options:0 error:nil]; return [value isKindOfClass:NSArray.class] ? value : @[]; }
static void configureManager(NSArray *models, NSString *storagePath, RimvModelCommandCallback command) {
    if (!manager) manager = [RimvModelManager new];
    manager.models = models;
    manager.storagePath = storagePath;
    manager.command = command;
}
void rimv_model_manager_configure(const char *catalog_json, const char *storage_path, RimvModelCommandCallback command) {
    NSArray *models = parseCatalog(catalog_json);
    NSString *storagePath = storage_path ? [NSString stringWithUTF8String:storage_path] : @"";
    if (NSThread.isMainThread) configureManager(models, storagePath, command);
    else dispatch_sync(dispatch_get_main_queue(), ^{ configureManager(models, storagePath, command); });
}
void rimv_model_manager_update(const char *catalog_json) {
    NSArray *models = parseCatalog(catalog_json);
    void (^update)(void) = ^{ if (!manager) manager = [RimvModelManager new]; manager.models = models; [manager render]; };
    if (NSThread.isMainThread) update(); else dispatch_async(dispatch_get_main_queue(), update);
}
void rimv_model_manager_show_selector(NSView *anchor) {
    if (!manager || !anchor) return;
    [manager showSelector:anchor];
}
bool rimv_model_manager_selector_contains_window(NSWindow *window) { return manager.selector.shown && manager.selector.contentViewController.view.window == window; }
bool rimv_model_manager_selector_is_shown(void) { return manager.selector.shown; }
bool rimv_model_manager_close_selector(void) { return [manager closeSelectorSafely]; }
