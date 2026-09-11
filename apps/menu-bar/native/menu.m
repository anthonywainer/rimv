#import <AppKit/AppKit.h>
#import <signal.h>

typedef void (*CommandCallback)(uint32_t, uint8_t);

@interface RimvMenu : NSObject <NSApplicationDelegate>
@property(nonatomic) CommandCallback command;
@property(nonatomic, strong) NSStatusItem *statusItem;
@property(nonatomic, strong) NSMenuItem *statusLine;
@property(nonatomic, strong) NSMenuItem *capture;
@property(nonatomic, strong) NSMenuItem *microphone;
@property(nonatomic, strong) NSMenuItem *system;
@property(nonatomic, strong) NSMenuItem *transcription;
@property(nonatomic, strong) NSMenuItem *drops;
@property(nonatomic, copy) NSString *modelSummary;
@property(nonatomic, strong) NSPanel *modelsPanel;
@property(nonatomic, strong) NSTextField *modelsText;
@property(nonatomic, strong) NSMenuItem *errorItem;
@property(nonatomic, strong) NSDictionary *snapshot;
@property(nonatomic, copy) NSString *root;
@property(nonatomic, copy) NSString *displayedError;
@property(nonatomic) BOOL quitting;
@property(nonatomic) BOOL systemTermination;
@property(nonatomic, strong) dispatch_source_t interruptSignal;
@property(nonatomic, strong) dispatch_source_t terminateSignal;
- (void)applySnapshot:(NSDictionary *)snapshot;
- (void)showError:(NSString *)message;
@end

static RimvMenu *menu;
static NSLock *mailboxLock;
static NSDictionary *pendingSnapshot;
static NSString *pendingError;
static BOOL updateScheduled;

// Coalesce updates into one main-thread task. A blocked/open menu cannot cause
// an unbounded backlog of snapshots or errors. All audio stays inside Rust.
static void scheduleUpdate(void) {
    if (updateScheduled) return; // Caller holds mailboxLock.
    updateScheduled = YES;
    dispatch_async(dispatch_get_main_queue(), ^{
        [mailboxLock lock];
        NSDictionary *snapshot = pendingSnapshot;
        NSString *error = pendingError;
        pendingSnapshot = nil;
        pendingError = nil;
        updateScheduled = NO;
        [mailboxLock unlock];
        if (snapshot) [menu applySnapshot:snapshot];
        if (error) [menu showError:error];
    });
}

static NSString *elapsed(uint64_t milliseconds) {
    uint64_t seconds = milliseconds / 1000;
    return [NSString stringWithFormat:@"%02llu:%02llu:%02llu",
            (unsigned long long)(seconds / 3600),
            (unsigned long long)((seconds / 60) % 60),
            (unsigned long long)(seconds % 60)];
}

@implementation RimvMenu
- (NSMenuItem *)item:(NSString *)title action:(SEL)action key:(NSString *)key {
    NSMenuItem *item = [[NSMenuItem alloc] initWithTitle:title action:action keyEquivalent:key];
    item.target = self;
    return item;
}
- (void)applicationDidFinishLaunching:(NSNotification *)notification {
    (void)notification;
    self.statusItem = [NSStatusBar.systemStatusBar statusItemWithLength:NSVariableStatusItemLength];
    self.statusItem.button.image = [NSImage imageWithSystemSymbolName:@"waveform" accessibilityDescription:@"rimv"];
    self.statusItem.button.image.template = YES;
    self.statusItem.button.imagePosition = NSImageLeft;
    self.statusItem.button.font = [NSFont monospacedDigitSystemFontOfSize:12 weight:NSFontWeightRegular];
    self.statusItem.button.title = @" rimv";
    NSMenu *items = [[NSMenu alloc] initWithTitle:@"rimv"];
    items.autoenablesItems = NO;
    NSMenuItem *heading = [self item:@"rimv" action:NULL key:@""];
    heading.enabled = NO;
    [items addItem:heading];
    self.statusLine = [self item:@"Idle" action:NULL key:@""];
    self.statusLine.enabled = NO;
    [items addItem:self.statusLine];
    [items addItem:NSMenuItem.separatorItem];
    self.capture = [self item:@"Start capture" action:@selector(toggleCapture:) key:@"r"];
    [items addItem:self.capture];
    self.microphone = [self item:@"Microphone" action:@selector(toggleMicrophone:) key:@"m"];
    [items addItem:self.microphone];
    self.system = [self item:@"System audio" action:@selector(toggleSystem:) key:@""];
    [items addItem:self.system];
    self.transcription = [self item:@"Transcription" action:@selector(toggleTranscription:) key:@""];
    [items addItem:self.transcription];
    [items addItem:[self item:@"Models…" action:@selector(showModels:) key:@""]];
    [items addItem:NSMenuItem.separatorItem];
    self.drops = [self item:@"Dropped blocks: 0" action:NULL key:@""];
    self.drops.enabled = NO;
    [items addItem:self.drops];
    self.errorItem = [self item:@"Show capture error…" action:@selector(errorDetails:) key:@""];
    self.errorItem.hidden = YES;
    [items addItem:self.errorItem];
    [items addItem:[self item:@"Open recordings folder" action:@selector(openRecordings:) key:@"o"]];
    [items addItem:[self item:@"Recording permissions…" action:@selector(openPermissions:) key:@""]];
    [items addItem:NSMenuItem.separatorItem];
    [items addItem:[self item:@"Quit rimv" action:@selector(quit:) key:@"q"]];
    self.statusItem.menu = items;
    [self applySnapshot:self.snapshot];
}
- (void)applySnapshot:(NSDictionary *)snapshot {
    self.snapshot = snapshot;
    if (!self.statusItem) return;
    NSString *status = snapshot[@"status"];
    BOOL recording = [status isEqualToString:@"recording"];
    BOOL transitioning = [status isEqualToString:@"starting"] || [status isEqualToString:@"stopping"];
    NSString *duration = elapsed([snapshot[@"elapsed_ms"] unsignedLongLongValue]);
    NSString *title = recording ? [NSString stringWithFormat:@" ● %@", duration] : @" rimv";
    if (![self.statusItem.button.title isEqualToString:title]) self.statusItem.button.title = title;
    self.statusItem.button.toolTip = [NSString stringWithFormat:@"rimv · %@ · %@", status.capitalizedString, duration];
    self.statusLine.title = [NSString stringWithFormat:@"%@ · %@", status.capitalizedString, duration];
    self.capture.title = recording ? @"Stop capture" : @"Start capture";
    self.capture.enabled = !transitioning && !self.quitting;
    NSDictionary *capabilities = snapshot[@"capabilities"];
    self.microphone.enabled = !transitioning && !self.quitting && [capabilities[@"microphone_capture"] boolValue];
    self.system.enabled = !transitioning && !self.quitting && [capabilities[@"system_audio_capture"] boolValue];
    NSDictionary *mic = snapshot[@"microphone"];
    NSDictionary *system = snapshot[@"system_audio"];
    self.microphone.state = [mic[@"enabled"] boolValue] ? NSControlStateValueOn : NSControlStateValueOff;
    self.system.state = [system[@"enabled"] boolValue] ? NSControlStateValueOn : NSControlStateValueOff;
    NSDictionary *transcription = snapshot[@"transcription"];
    self.transcription.enabled = !transitioning && [transcription[@"available"] boolValue];
    self.transcription.state = [transcription[@"enabled"] boolValue] ? NSControlStateValueOn : NSControlStateValueOff;
    self.transcription.title = self.transcription.enabled ? @"Transcription" : @"Transcription — model required";
    self.microphone.title = [self sourceTitle:@"Microphone" source:mic recording:recording];
    self.system.title = [self sourceTitle:@"System audio" source:system recording:recording];
    self.drops.title = [NSString stringWithFormat:@"Dropped blocks: mic %@ · system %@",
                       snapshot[@"dropped_microphone_blocks"], snapshot[@"dropped_system_blocks"]];
    id error = snapshot[@"last_error"];
    if ([error isKindOfClass:NSDictionary.class]) {
        [self showError:error[@"message"]];
    } else if ([status isEqualToString:@"starting"]) {
        self.displayedError = nil;
        self.errorItem.hidden = YES;
    }
}
- (NSString *)sourceTitle:(NSString *)name source:(NSDictionary *)source recording:(BOOL)recording {
    if (![source[@"enabled"] boolValue]) return [name stringByAppendingString:@" — off"];
    if (recording && ![source[@"active"] boolValue]) return [name stringByAppendingString:@" — unavailable (click to disable)"];
    return [name stringByAppendingString:recording ? @" — recording" : @" — on"];
}
- (void)toggleCapture:(id)sender {
    (void)sender;
    self.capture.enabled = NO;
    self.command([self.snapshot[@"status"] isEqualToString:@"recording"] ? 2 : 1, 0);
}
- (void)toggleMicrophone:(id)sender {
    (void)sender;
    self.microphone.enabled = NO;
    self.command(3, ![self.snapshot[@"microphone"][@"enabled"] boolValue]);
}
- (void)toggleSystem:(id)sender {
    (void)sender;
    self.system.enabled = NO;
    self.command(4, ![self.snapshot[@"system_audio"][@"enabled"] boolValue]);
}
- (void)toggleTranscription:(id)sender { (void)sender; self.command(9, ![self.snapshot[@"transcription"][@"enabled"] boolValue]); }
- (void)showError:(NSString *)message {
    self.displayedError = message;
    self.errorItem.hidden = NO;
    self.errorItem.toolTip = message;
}
- (void)errorDetails:(id)sender {
    (void)sender;
    NSAlert *alert = [[NSAlert alloc] init];
    alert.messageText = @"rimv capture error";
    alert.informativeText = self.displayedError ?: @"No error details available.";
    [alert addButtonWithTitle:@"OK"];
    [NSApp activateIgnoringOtherApps:YES];
    [alert runModal];
}
- (void)openRecordings:(id)sender {
    (void)sender;
    [NSWorkspace.sharedWorkspace openURL:[NSURL fileURLWithPath:self.root isDirectory:YES]];
}
- (void)showModels:(id)sender {
    (void)sender;
    if (!self.modelsPanel) {
        self.modelsPanel = [[NSPanel alloc] initWithContentRect:NSMakeRect(0, 0, 420, 280)
            styleMask:(NSWindowStyleMaskTitled | NSWindowStyleMaskClosable) backing:NSBackingStoreBuffered defer:NO];
        self.modelsPanel.title = @"Models";
        NSView *view = self.modelsPanel.contentView;
        self.modelsText = [NSTextField labelWithString:@""];
        self.modelsText.frame = NSMakeRect(24, 100, 372, 145);
        self.modelsText.maximumNumberOfLines = 0;
        [view addSubview:self.modelsText];
        NSArray *titles = @[@"Download Tiny", @"Download Base", @"Download Small"];
        for (NSInteger i = 0; i < 3; i++) {
            NSButton *button = [NSButton buttonWithTitle:titles[i] target:self action:@selector(downloadModel:)];
            button.tag = 6 + i;
            button.frame = NSMakeRect(24 + i * 122, 42, 112, 32);
            [view addSubview:button];
        }
    }
    self.modelsText.stringValue = self.modelSummary ?: @"Loading model recommendations…";
    [NSApp activateIgnoringOtherApps:YES];
    [self.modelsPanel makeKeyAndOrderFront:nil];
}
- (void)downloadModel:(NSButton *)sender { self.command((uint32_t)sender.tag, 0); }
- (void)openPermissions:(id)sender {
    (void)sender;
    [NSWorkspace.sharedWorkspace openURL:[NSURL URLWithString:@"x-apple.systempreferences:com.apple.preference.security?Privacy"]];
}
- (void)quit:(id)sender {
    (void)sender;
    if (self.quitting) return;
    self.quitting = YES;
    self.statusLine.title = @"Finalizing recordings…";
    self.capture.enabled = NO;
    self.microphone.enabled = NO;
    self.system.enabled = NO;
    self.command(5, 0);
}
- (NSApplicationTerminateReply)applicationShouldTerminate:(NSApplication *)sender {
    (void)sender;
    self.systemTermination = YES;
    [self quit:nil];
    return NSTerminateLater;
}
@end

void rimv_menu_create(const char *root, const char *snapshot, CommandCallback command) {
    @autoreleasepool {
        [NSApplication sharedApplication];
        [NSApp setActivationPolicy:NSApplicationActivationPolicyAccessory];
        mailboxLock = [[NSLock alloc] init];
        menu = [[RimvMenu alloc] init];
        menu.command = command;
        menu.root = [NSString stringWithUTF8String:root];
        NSData *data = [[NSString stringWithUTF8String:snapshot] dataUsingEncoding:NSUTF8StringEncoding];
        menu.snapshot = [NSJSONSerialization JSONObjectWithData:data options:0 error:NULL];
        NSApp.delegate = menu;
        // SIGINT/SIGTERM use the same graceful shutdown path, on the main queue.
        signal(SIGINT, SIG_IGN);
        signal(SIGTERM, SIG_IGN);
        menu.interruptSignal = dispatch_source_create(DISPATCH_SOURCE_TYPE_SIGNAL, SIGINT, 0, dispatch_get_main_queue());
        menu.terminateSignal = dispatch_source_create(DISPATCH_SOURCE_TYPE_SIGNAL, SIGTERM, 0, dispatch_get_main_queue());
        dispatch_source_set_event_handler(menu.interruptSignal, ^{ [menu quit:nil]; });
        dispatch_source_set_event_handler(menu.terminateSignal, ^{ [menu quit:nil]; });
        dispatch_resume(menu.interruptSignal);
        dispatch_resume(menu.terminateSignal);
    }
}

void rimv_menu_run(void) {
    @autoreleasepool { [NSApp run]; }
}

void rimv_menu_update(const char *snapshot) {
    // Rust-owned threads do not have an Objective-C autorelease pool.
    @autoreleasepool {
        NSData *data = [[NSString stringWithUTF8String:snapshot] dataUsingEncoding:NSUTF8StringEncoding];
        NSDictionary *state = [NSJSONSerialization JSONObjectWithData:data options:0 error:NULL];
        if (!state) return;
        [mailboxLock lock];
        pendingSnapshot = state;
        scheduleUpdate();
        [mailboxLock unlock];
    }
}

void rimv_menu_error(const char *message) {
    @autoreleasepool {
        [mailboxLock lock];
        pendingError = [NSString stringWithUTF8String:message];
        scheduleUpdate();
        [mailboxLock unlock];
    }
}

void rimv_menu_set_model_summary(const char *summary) {
    NSString *copy = [NSString stringWithUTF8String:summary];
    dispatch_async(dispatch_get_main_queue(), ^{
        menu.modelSummary = copy;
        menu.modelsText.stringValue = copy;
    });
}

void rimv_menu_exit(void) {
    dispatch_async(dispatch_get_main_queue(), ^{
        if (menu.systemTermination) {
            [NSApp replyToApplicationShouldTerminate:YES];
        } else {
            [NSApp stop:nil];
            // Wake the AppKit run loop so Rust can join threads and return.
            [NSApp postEvent:[NSEvent otherEventWithType:NSEventTypeApplicationDefined
                location:NSZeroPoint modifierFlags:0 timestamp:0 windowNumber:0
                context:nil subtype:0 data1:0 data2:0] atStart:NO];
        }
    });
}

static uint32_t testedCommand;
static uint8_t testedEnabled;
static void testCommand(uint32_t command, uint8_t enabled) {
    testedCommand = command;
    testedEnabled = enabled;
}

// Runs on the real AppKit main thread, without opening hardware or simulating
// mouse input. Checks native rendering and action-to-command wiring together.
bool rimv_menu_self_test(void) {
    @autoreleasepool {
        [menu applicationDidFinishLaunching:[NSNotification notificationWithName:NSApplicationDidFinishLaunchingNotification object:NSApp]];
        CommandCallback original = menu.command;
        menu.command = testCommand;
        NSMutableDictionary *state = [menu.snapshot mutableCopy];
        BOOL passed = menu.statusItem != nil && menu.capture.enabled;
        passed &= [menu.capture.title isEqualToString:@"Start capture"];
        passed &= menu.microphone.state == NSControlStateValueOn;
        [menu toggleCapture:nil];
        passed &= testedCommand == 1;
        state[@"status"] = @"starting";
        [menu applySnapshot:state];
        passed &= !menu.capture.enabled && !menu.microphone.enabled;
        state[@"status"] = @"recording";
        state[@"elapsed_ms"] = @754000;
        state[@"microphone"] = @{@"enabled": @YES, @"active": @YES};
        [menu applySnapshot:state];
        passed &= [menu.statusLine.title isEqualToString:@"Recording · 00:12:34"];
        passed &= [menu.capture.title isEqualToString:@"Stop capture"];
        [menu toggleCapture:nil];
        passed &= testedCommand == 2;
        [menu toggleMicrophone:nil];
        passed &= testedCommand == 3 && testedEnabled == 0;
        state[@"microphone"] = @{@"enabled": @NO, @"active": @NO};
        [menu applySnapshot:state];
        passed &= menu.microphone.state == NSControlStateValueOff;
        passed &= [menu.statusLine.title hasPrefix:@"Recording"];
        [menu toggleMicrophone:nil];
        passed &= testedCommand == 3 && testedEnabled == 1;
        [menu toggleSystem:nil];
        passed &= testedCommand == 4 && testedEnabled == 1;
        state[@"system_audio"] = @{@"enabled": @YES, @"active": @NO};
        state[@"last_error"] = @{@"message": @"Permission denied"};
        [menu applySnapshot:state];
        passed &= [menu.system.title containsString:@"unavailable"];
        passed &= !menu.errorItem.hidden && [menu.displayedError isEqualToString:@"Permission denied"];
        state[@"status"] = @"idle";
        [menu applySnapshot:state];
        passed &= [menu.capture.title isEqualToString:@"Start capture"];
        passed &= [menu.statusLine.title isEqualToString:@"Idle · 00:12:34"];
        [menu quit:nil];
        passed &= testedCommand == 5 && !menu.capture.enabled;
        menu.command = original;
        fprintf(stdout, "Native menu self-test: %s\n", passed ? "PASS" : "FAIL");
        return passed;
    }
}
