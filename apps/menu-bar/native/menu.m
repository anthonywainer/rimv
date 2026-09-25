#import <AppKit/AppKit.h>
#import <signal.h>
#import "model_manager.h"

typedef void (*CommandCallback)(uint32_t, uint8_t);

static BOOL RimvIsDark(NSAppearance *appearance) {
    return [[appearance bestMatchFromAppearancesWithNames:@[
        NSAppearanceNameAqua, NSAppearanceNameDarkAqua
    ]] isEqualToString:NSAppearanceNameDarkAqua];
}

@interface RimvPopoverView : NSView
@property(nonatomic, copy) dispatch_block_t appearanceChanged;
@end

@interface RimvLanguageSelectorView : NSView
@property(nonatomic, copy) dispatch_block_t appearanceChanged;
@end

@implementation RimvLanguageSelectorView
- (BOOL)isFlipped { return YES; }
- (void)viewDidChangeEffectiveAppearance {
    [super viewDidChangeEffectiveAppearance];
    [self setNeedsDisplay:YES];
    if (self.appearanceChanged) self.appearanceChanged();
}
- (void)drawRect:(NSRect)dirtyRect {
    (void)dirtyRect;
    BOOL dark = RimvIsDark(self.effectiveAppearance);
    NSColor *surface = dark
        ? [NSColor colorWithRed:20.0/255.0 green:36.0/255.0 blue:56.0/255.0 alpha:1]
        : NSColor.whiteColor;
    [surface setFill];
    [[NSBezierPath bezierPathWithRoundedRect:self.bounds xRadius:17 yRadius:17] fill];
}
@end

@implementation RimvPopoverView
- (BOOL)isFlipped { return YES; }
- (void)viewDidChangeEffectiveAppearance {
    [super viewDidChangeEffectiveAppearance];
    [self setNeedsDisplay:YES];
    if (self.appearanceChanged) self.appearanceChanged();
}
- (void)drawRect:(NSRect)dirtyRect {
    (void)dirtyRect;
    BOOL dark = RimvIsDark(self.effectiveAppearance);
    NSColor *background = dark
        ? [NSColor colorWithRed:9.0/255.0 green:16.0/255.0 blue:29.0/255.0 alpha:1]
        : [NSColor colorWithRed:247.0/255.0 green:247.0/255.0 blue:248.0/255.0 alpha:1];
    [background setFill];
    NSRectFill(self.bounds);
    NSColor *panel = dark
        ? [NSColor colorWithRed:16.0/255.0 green:26.0/255.0 blue:43.0/255.0 alpha:1]
        : NSColor.whiteColor;
    [panel setFill];
    [[NSBezierPath bezierPathWithRoundedRect:self.bounds xRadius:18 yRadius:18] fill];
}
@end

@interface RimvMenu : NSObject <NSApplicationDelegate, NSPopoverDelegate, NSSearchFieldDelegate>
@property(nonatomic) CommandCallback command;
@property(nonatomic, strong) NSStatusItem *statusItem;
@property(nonatomic, strong) NSTextField *statusLine;
@property(nonatomic, strong) NSButton *capture;
@property(nonatomic, strong) NSButton *source;
@property(nonatomic, strong) NSButton *microphone;
@property(nonatomic, strong) NSButton *system;
@property(nonatomic, strong) NSButton *transcription;
@property(nonatomic, strong) NSTextField *drops;
@property(nonatomic, copy) NSString *modelSummary;
@property(nonatomic, strong) NSButton *errorItem;
@property(nonatomic, strong) NSPopover *popover;
@property(nonatomic, strong) RimvPopoverView *popoverView;
@property(nonatomic, strong) NSTextField *brandDetail;
@property(nonatomic, strong) NSTextField *brandState;
@property(nonatomic, strong) NSTextField *sourceDetail;
@property(nonatomic, strong) id keyMonitor;
@property(nonatomic, strong) id globalClickMonitor;
@property(nonatomic, strong) id localClickMonitor;
@property(nonatomic, strong) NSTextField *brandTitle;
@property(nonatomic, strong) NSButton *systemCard;
@property(nonatomic, strong) NSButton *microphoneCard;
@property(nonatomic, strong) NSButton *bothCard;
@property(nonatomic, strong) NSButton *language;
@property(nonatomic, strong) NSButton *model;
@property(nonatomic, strong) NSTextField *languageValue;
@property(nonatomic, strong) NSTextField *modelValue;
@property(nonatomic, strong) NSImageView *languageIcon;
@property(nonatomic, strong) NSImageView *modelIcon;
@property(nonatomic, strong) NSImageView *languageChevron;
@property(nonatomic, strong) NSImageView *modelChevron;
@property(nonatomic, copy) NSString *selectedLanguage;
@property(nonatomic, copy) NSArray<NSString *> *supportedLanguages;
@property(nonatomic) BOOL supportsLanguageDetection;
@property(nonatomic) BOOL languageMenuOpen;
@property(nonatomic, strong) NSPopover *languagePopover;
@property(nonatomic, strong) RimvLanguageSelectorView *languagePopoverView;
@property(nonatomic, strong) NSSearchField *languageSearch;
@property(nonatomic, strong) NSStackView *languageList;
@property(nonatomic, strong) NSView *languageListDocument;
@property(nonatomic, strong) NSTextField *languageSummary;
@property(nonatomic, strong) NSButton *allLanguagesButton;
@property(nonatomic) BOOL showAllLanguages;
@property(nonatomic, strong) NSView *captureArea;
@property(nonatomic, strong) NSImageView *captureIcon;
@property(nonatomic, strong) NSTextField *captureLabel;
@property(nonatomic, strong) NSStackView *captureGroup;
@property(nonatomic, strong) NSButton *recordings;
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
- (void)setSymbol:(NSString *)name onItem:(id)item description:(NSString *)description {
    [item setImage:[NSImage imageWithSystemSymbolName:name accessibilityDescription:description]];
}
- (NSColor *)color:(CGFloat)r green:(CGFloat)g blue:(CGFloat)b darkRed:(CGFloat)dr green:(CGFloat)dg blue:(CGFloat)db {
    BOOL dark = RimvIsDark(self.popoverView.effectiveAppearance ?: NSApp.effectiveAppearance);
    return [NSColor colorWithRed:(dark ? dr : r) / 255.0 green:(dark ? dg : g) / 255.0 blue:(dark ? db : b) / 255.0 alpha:1];
}
- (NSTextField *)label:(NSString *)text frame:(NSRect)frame size:(CGFloat)size weight:(NSFontWeight)weight {
    NSTextField *label = [NSTextField labelWithString:text];
    label.frame = frame;
    label.font = [NSFont systemFontOfSize:size weight:weight];
    label.lineBreakMode = NSLineBreakByTruncatingTail;
    [self.popoverView addSubview:label];
    return label;
}
- (NSButton *)row:(NSString *)title symbol:(NSString *)symbol action:(SEL)action frame:(NSRect)frame {
    NSButton *button = [NSButton buttonWithTitle:title target:self action:action];
    button.frame = frame;
    button.bordered = NO;
    button.alignment = NSTextAlignmentLeft;
    button.imagePosition = NSImageLeading;
    NSImage *image = [NSImage imageWithSystemSymbolName:symbol accessibilityDescription:title];
    button.image = [image imageWithSymbolConfiguration:[NSImageSymbolConfiguration configurationWithPointSize:16 weight:NSFontWeightRegular]];
    button.image.size = NSMakeSize(16, 16);
    button.imageScaling = NSImageScaleProportionallyDown;
    button.imageHugsTitle = YES;
    button.font = [NSFont systemFontOfSize:14 weight:NSFontWeightRegular];
    button.contentTintColor = [self color:23 green:24 blue:39 darkRed:248 green:249 blue:252];
    button.wantsLayer = YES;
    button.layer.cornerRadius = 10;
    [self.popoverView addSubview:button];
    return button;
}
- (void)separatorAt:(CGFloat)y {
    NSBox *separator = [[NSBox alloc] initWithFrame:NSMakeRect(16, y, 348, 1)];
    separator.boxType = NSBoxSeparator;
    [self.popoverView addSubview:separator];
}
- (NSButton *)selectorRow:(NSString *)labelText symbol:(NSString *)symbol action:(SEL)action frame:(NSRect)frame value:(NSTextField **)valueOut icon:(NSImageView **)iconOut chevron:(NSImageView **)chevronOut {
    NSButton *button = [self row:@"" symbol:@"" action:action frame:frame];
    button.accessibilityLabel = labelText;
    NSImageView *icon = [[NSImageView alloc] initWithFrame:NSZeroRect];
    icon.image = [[NSImage imageWithSystemSymbolName:symbol accessibilityDescription:labelText]
        imageWithSymbolConfiguration:[NSImageSymbolConfiguration configurationWithPointSize:16 weight:NSFontWeightRegular]];
    icon.image.template = YES;
    icon.translatesAutoresizingMaskIntoConstraints = NO;
    NSTextField *label = [NSTextField labelWithString:labelText];
    label.font = [NSFont systemFontOfSize:14 weight:NSFontWeightRegular];
    label.translatesAutoresizingMaskIntoConstraints = NO;
    NSTextField *value = [NSTextField labelWithString:@""];
    value.font = [NSFont systemFontOfSize:14 weight:NSFontWeightRegular];
    value.alignment = NSTextAlignmentRight;
    value.lineBreakMode = NSLineBreakByTruncatingHead;
    value.translatesAutoresizingMaskIntoConstraints = NO;
    NSImageView *chevron = [[NSImageView alloc] initWithFrame:NSZeroRect];
    chevron.image = [[NSImage imageWithSystemSymbolName:@"chevron.right" accessibilityDescription:@"Choose"]
        imageWithSymbolConfiguration:[NSImageSymbolConfiguration configurationWithPointSize:12 weight:NSFontWeightSemibold]];
    chevron.image.template = YES;
    chevron.translatesAutoresizingMaskIntoConstraints = NO;
    [button addSubview:icon];
    [button addSubview:label];
    [button addSubview:value];
    [button addSubview:chevron];
    [NSLayoutConstraint activateConstraints:@[
        [icon.leadingAnchor constraintEqualToAnchor:button.leadingAnchor constant:12],
        [icon.centerYAnchor constraintEqualToAnchor:button.centerYAnchor],
        [icon.widthAnchor constraintEqualToConstant:16], [icon.heightAnchor constraintEqualToConstant:16],
        [label.leadingAnchor constraintEqualToAnchor:icon.trailingAnchor constant:10],
        [label.centerYAnchor constraintEqualToAnchor:button.centerYAnchor],
        [chevron.trailingAnchor constraintEqualToAnchor:button.trailingAnchor constant:-12],
        [chevron.centerYAnchor constraintEqualToAnchor:button.centerYAnchor],
        [chevron.widthAnchor constraintEqualToConstant:12], [chevron.heightAnchor constraintEqualToConstant:12],
        [value.trailingAnchor constraintEqualToAnchor:chevron.leadingAnchor constant:-8],
        [value.leadingAnchor constraintGreaterThanOrEqualToAnchor:label.trailingAnchor constant:12],
        [value.centerYAnchor constraintEqualToAnchor:button.centerYAnchor],
    ]];
    if (valueOut) *valueOut = value;
    if (iconOut) *iconOut = icon;
    if (chevronOut) *chevronOut = chevron;
    return button;
}
- (NSButton *)sourceCard:(NSString *)title symbol:(NSString *)symbol action:(SEL)action frame:(NSRect)frame {
    NSButton *card = [self row:@"" symbol:@"" action:action frame:frame];
    card.accessibilityLabel = title;
    NSImageView *icon = [[NSImageView alloc] initWithFrame:NSMakeRect(0, 0, 20, 20)];
    icon.image = [[NSImage imageWithSystemSymbolName:symbol accessibilityDescription:title]
        imageWithSymbolConfiguration:[NSImageSymbolConfiguration configurationWithPointSize:20 weight:NSFontWeightRegular]];
    icon.image.template = YES;
    icon.imageScaling = NSImageScaleProportionallyDown;
    NSTextField *label = [NSTextField labelWithString:title];
    label.font = [NSFont systemFontOfSize:14 weight:NSFontWeightMedium];
    label.alignment = NSTextAlignmentCenter;
    NSStackView *content = [NSStackView stackViewWithViews:@[icon, label]];
    content.orientation = NSUserInterfaceLayoutOrientationVertical;
    content.alignment = NSLayoutAttributeCenterX;
    content.spacing = 7;
    content.translatesAutoresizingMaskIntoConstraints = NO;
    [card addSubview:content];
    [NSLayoutConstraint activateConstraints:@[
        [content.centerXAnchor constraintEqualToAnchor:card.centerXAnchor],
        [content.topAnchor constraintEqualToAnchor:card.topAnchor constant:10],
        [icon.widthAnchor constraintEqualToConstant:20],
        [icon.heightAnchor constraintEqualToConstant:20],
    ]];
    card.layer.cornerRadius = 11;
    return card;
}
- (void)applicationDidFinishLaunching:(NSNotification *)notification {
    (void)notification;
    self.statusItem = [NSStatusBar.systemStatusBar statusItemWithLength:NSVariableStatusItemLength];
    self.statusItem.button.image = [NSImage imageWithSystemSymbolName:@"waveform" accessibilityDescription:@"RimV status"];
    self.statusItem.button.image.template = YES;
    self.statusItem.button.imagePosition = NSImageLeft;
    self.statusItem.button.font = [NSFont monospacedDigitSystemFontOfSize:12 weight:NSFontWeightRegular];
    self.statusItem.button.title = @" RimV";
    self.statusItem.button.target = self;
    self.statusItem.button.action = @selector(togglePopover:);
    self.popoverView = [[RimvPopoverView alloc] initWithFrame:NSMakeRect(0, 0, 380, 560)];
    self.popoverView.wantsLayer = YES;
    NSViewController *controller = [[NSViewController alloc] init];
    controller.view = self.popoverView;
    self.popover = [[NSPopover alloc] init];
    self.popover.behavior = NSPopoverBehaviorTransient;
    self.popover.delegate = self;
    self.popover.contentSize = self.popoverView.bounds.size;
    self.popover.contentViewController = controller;
    __weak RimvMenu *weakSelf = self;
    self.popoverView.appearanceChanged = ^{
        dispatch_async(dispatch_get_main_queue(), ^{
            RimvMenu *strongSelf = weakSelf;
            if (strongSelf) [strongSelf applySnapshot:strongSelf.snapshot];
        });
    };
    self.keyMonitor = [NSEvent addLocalMonitorForEventsMatchingMask:NSEventMaskKeyDown handler:^NSEvent *(NSEvent *event) {
        RimvMenu *strongSelf = weakSelf;
        if (!strongSelf.popover.shown) return event;
        if (event.keyCode == 53) {
            if (strongSelf.languagePopover.shown) {
                [strongSelf.languagePopover performClose:nil];
                return nil;
            }
            [strongSelf.popover performClose:nil];
            return nil;
        }
        if (!(event.modifierFlags & NSEventModifierFlagCommand)) return event;
        NSString *key = event.charactersIgnoringModifiers.lowercaseString;
        if ([key isEqualToString:@"r"]) [strongSelf toggleCapture:nil];
        else if ([key isEqualToString:@"m"]) [strongSelf toggleMicrophone:nil];
        else if ([key isEqualToString:@"o"]) [strongSelf openRecordings:nil];
        else if ([key isEqualToString:@"q"]) [strongSelf quit:nil];
        else return event;
        return nil;
    }];
    self.globalClickMonitor = [NSEvent addGlobalMonitorForEventsMatchingMask:(NSEventMaskLeftMouseDown | NSEventMaskRightMouseDown)
                                                                       handler:^(__unused NSEvent *event) {
        dispatch_async(dispatch_get_main_queue(), ^{
            [weakSelf.languagePopover performClose:nil];
            [weakSelf.popover performClose:nil];
        });
    }];
    self.localClickMonitor = [NSEvent addLocalMonitorForEventsMatchingMask:(NSEventMaskLeftMouseDown | NSEventMaskRightMouseDown)
                                                                      handler:^NSEvent *(NSEvent *event) {
        RimvMenu *strongSelf = weakSelf;
        NSWindow *popoverWindow = strongSelf.popover.contentViewController.view.window;
        NSWindow *languageWindow = strongSelf.languagePopover.contentViewController.view.window;
        if (strongSelf.languagePopover.shown && event.window != languageWindow) [strongSelf.languagePopover performClose:nil];
        if (strongSelf.popover.shown && event.window != popoverWindow && event.window != languageWindow) [strongSelf.popover performClose:nil];
        return event;
    }];

    NSView *mark = [[NSView alloc] initWithFrame:NSMakeRect(16, 34, 40, 40)];
    mark.wantsLayer = YES;
    mark.layer.cornerRadius = 11;
    mark.layer.backgroundColor = [self color:23 green:153 blue:143 darkRed:23 green:153 blue:143].CGColor;
    [self.popoverView addSubview:mark];
    NSImageView *markImage = [[NSImageView alloc] initWithFrame:NSMakeRect(8, 8, 24, 24)];
    markImage.image = [NSImage imageWithSystemSymbolName:@"waveform" accessibilityDescription:@"RimV"];
    markImage.contentTintColor = NSColor.whiteColor;
    [mark addSubview:markImage];
    self.brandTitle = [self label:@"RimV" frame:NSMakeRect(66, 31, 160, 26) size:23 weight:NSFontWeightBold];
    self.brandDetail = [self label:@"Real-time transcription" frame:NSMakeRect(66, 57, 190, 18) size:14 weight:NSFontWeightRegular];
    self.brandState = [self label:@"●  Ready" frame:NSMakeRect(284, 42, 80, 18) size:12 weight:NSFontWeightMedium];
    self.statusLine = [self label:@"Ready" frame:NSZeroRect size:11 weight:NSFontWeightRegular];
    self.statusLine.hidden = YES;
    [self separatorAt:104];
    self.captureArea = [[NSView alloc] initWithFrame:NSMakeRect(16, 118, 348, 184)];
    self.captureArea.wantsLayer = YES;
    self.captureArea.layer.cornerRadius = 13;
    [self.popoverView addSubview:self.captureArea];
    self.source = [self sourceCard:@"System" symbol:@"desktopcomputer" action:@selector(selectSystem:) frame:NSMakeRect(28, 160, 103, 74)];
    self.systemCard = self.source;
    self.microphoneCard = [self sourceCard:@"Microphone" symbol:@"mic" action:@selector(selectMicrophone:) frame:NSMakeRect(139, 160, 103, 74)];
    self.bothCard = [self sourceCard:@"Both" symbol:@"waveform" action:@selector(selectBoth:) frame:NSMakeRect(250, 160, 103, 74)];
    self.sourceDetail = [self label:@"CAPTURE SOURCE" frame:NSMakeRect(28, 134, 180, 16) size:12 weight:NSFontWeightBold];
    self.microphone = [NSButton buttonWithTitle:@"Microphone" target:self action:@selector(toggleMicrophone:)];
    self.system = [NSButton buttonWithTitle:@"System Audio" target:self action:@selector(toggleSystem:)];
    self.capture = [self row:@"" symbol:@"" action:@selector(toggleCapture:) frame:NSMakeRect(28, 249, 324, 49)];
    self.capture.image = nil;
    self.capture.alignment = NSTextAlignmentCenter;
    self.captureIcon = [[NSImageView alloc] initWithFrame:NSMakeRect(0, 0, 18, 18)];
    self.captureIcon.contentTintColor = NSColor.whiteColor;
    self.captureLabel = [NSTextField labelWithString:@"Start Listening"];
    self.captureLabel.font = [NSFont systemFontOfSize:15 weight:NSFontWeightBold];
    self.captureIcon.image = [NSImage imageWithSystemSymbolName:@"play.fill" accessibilityDescription:@"Start Listening"];
    self.captureLabel.textColor = NSColor.whiteColor;
    self.captureGroup = [NSStackView stackViewWithViews:@[self.captureIcon, self.captureLabel]];
    self.captureGroup.orientation = NSUserInterfaceLayoutOrientationHorizontal;
    self.captureGroup.spacing = 8;
    self.captureGroup.alignment = NSLayoutAttributeCenterY;
    [self.capture addSubview:self.captureGroup];
    NSTextField *languageValue;
    NSImageView *languageIcon;
    NSImageView *languageChevron;
    self.language = [self selectorRow:@"Language" symbol:@"globe" action:@selector(showLanguages:) frame:NSMakeRect(30, 319, 344, 36) value:&languageValue icon:&languageIcon chevron:&languageChevron];
    self.languageValue = languageValue;
    self.languageIcon = languageIcon;
    self.languageChevron = languageChevron;
    NSTextField *modelValue;
    NSImageView *modelIcon;
    NSImageView *modelChevron;
    self.model = [self selectorRow:@"Model" symbol:@"cpu" action:@selector(showModels:) frame:NSMakeRect(30, 363, 344, 36) value:&modelValue icon:&modelIcon chevron:&modelChevron];
    self.modelValue = modelValue;
    self.modelIcon = modelIcon;
    self.modelChevron = modelChevron;
    [self separatorAt:412];
    self.errorItem = [self row:@"Show Capture Error…" symbol:@"exclamationmark.triangle" action:@selector(errorDetails:) frame:NSMakeRect(30, 428, 344, 28)];
    self.errorItem.hidden = YES;
    self.drops = [self label:@"" frame:NSZeroRect size:12 weight:NSFontWeightRegular];
    self.recordings = [self row:@"Open Recordings" symbol:@"folder" action:@selector(openRecordings:) frame:NSMakeRect(30, 428, 344, 28)];
    self.recordings.image.template = YES;
    NSButton *preferences = [self row:@"Preferences" symbol:@"gearshape" action:NULL frame:NSMakeRect(30, 464, 344, 28)];
    preferences.enabled = NO;
    [self separatorAt:500];
    NSButton *quit = [self row:@"Quit RimV                                             ⌘Q" symbol:@"power" action:@selector(quit:) frame:NSMakeRect(30, 510, 344, 28)];
    quit.font = [NSFont systemFontOfSize:14 weight:NSFontWeightRegular];
    [self applySnapshot:self.snapshot];
}
- (void)applySnapshot:(NSDictionary *)snapshot {
    self.snapshot = snapshot;
    if (!self.statusItem) return;
    NSColor *primary = [self color:17 green:20 blue:24 darkRed:244 green:247 blue:250];
    NSColor *secondary = [self color:107 green:114 blue:128 darkRed:158 green:168 blue:182];
    self.captureArea.layer.backgroundColor = [self color:243 green:244 blue:246 darkRed:21 green:34 blue:56].CGColor;
    for (NSView *view in self.popoverView.subviews) {
        if ([view isKindOfClass:NSTextField.class]) ((NSTextField *)view).textColor = primary;
        if ([view isKindOfClass:NSButton.class]) ((NSButton *)view).contentTintColor = primary;
    }
    self.brandTitle.textColor = primary;
    self.brandDetail.textColor = secondary;
    self.sourceDetail.textColor = primary;
    for (NSImageView *icon in @[self.languageIcon, self.modelIcon]) icon.contentTintColor = primary;
    for (NSImageView *chevron in @[self.languageChevron, self.modelChevron]) chevron.contentTintColor = secondary;
    self.languageValue.textColor = secondary;
    self.modelValue.textColor = secondary;
    NSString *status = snapshot[@"status"];
    BOOL recording = [status isEqualToString:@"recording"];
    BOOL starting = [status isEqualToString:@"starting"];
    BOOL stopping = [status isEqualToString:@"stopping"];
    BOOL transitioning = starting || stopping;
    NSString *duration = elapsed([snapshot[@"elapsed_ms"] unsignedLongLongValue]);
    NSDictionary *transcription = snapshot[@"transcription"];
    NSString *statusText = @"Ready";
    if (recording) statusText = [NSString stringWithFormat:@"Listening · %@", duration];
    else if (stopping) statusText = @"Saving transcript…";
    else if (starting && [transcription[@"status"] isEqualToString:@"loading"]) statusText = @"Preparing transcription…";
    else if (starting) statusText = @"Starting…";
    else if ([status isEqualToString:@"error"]) statusText = @"Capture needs attention";
    NSString *title = recording ? [NSString stringWithFormat:@" ● %@", duration] : @" RimV";
    if (![self.statusItem.button.title isEqualToString:title]) self.statusItem.button.title = title;
    self.statusItem.button.toolTip = [NSString stringWithFormat:@"RimV · %@", statusText];
    self.statusLine.stringValue = statusText;
    self.statusLine.textColor = [self color:125 green:129 blue:147 darkRed:165 green:176 blue:196];
    self.brandDetail.stringValue = recording
        ? [NSString stringWithFormat:@"%@ · %@", [self selectedSourceSummary:snapshot[@"microphone"] system:snapshot[@"system_audio"]], duration]
        : ([status isEqualToString:@"error"] ? @"Capture needs attention" : @"Real-time transcription");
    self.brandDetail.textColor = [self color:125 green:129 blue:147 darkRed:165 green:176 blue:196];
    self.brandState.stringValue = recording ? @"●  LIVE" : ([status isEqualToString:@"error"] ? @"●  Error" : (transitioning ? @"●  Loading" : @"●  Ready"));
    self.brandState.textColor = recording
        ? [self color:22 green:163 blue:74 darkRed:34 green:197 blue:94]
        : ([status isEqualToString:@"error"]
            ? [self color:212 green:71 blue:71 darkRed:248 green:133 blue:133]
            : [self color:22 green:163 blue:74 darkRed:34 green:197 blue:94]);
    NSString *captureTitle = recording ? @"Stop Listening" : ([status isEqualToString:@"error"] ? @"Retry Listening" : @"Start Listening");
    self.capture.title = @"";
    self.captureLabel.stringValue = captureTitle;
    self.captureLabel.textColor = NSColor.whiteColor;
    self.captureIcon.image = [[NSImage imageWithSystemSymbolName:(recording ? @"stop.fill" : ([status isEqualToString:@"error"] ? @"arrow.clockwise" : @"play.fill"))
                                           accessibilityDescription:captureTitle]
        imageWithSymbolConfiguration:[NSImageSymbolConfiguration configurationWithPointSize:16 weight:NSFontWeightSemibold]];
    self.captureIcon.image.template = YES;
    self.captureIcon.image.size = NSMakeSize(16, 16);
    self.captureIcon.contentTintColor = NSColor.whiteColor;
    NSSize groupSize = self.captureGroup.fittingSize;
    self.captureGroup.frame = NSMakeRect((NSWidth(self.capture.bounds) - groupSize.width) / 2,
                                         (NSHeight(self.capture.bounds) - groupSize.height) / 2,
                                         groupSize.width, groupSize.height);
    self.capture.contentTintColor = recording
        ? [self color:195 green:77 blue:83 darkRed:247 green:133 blue:133]
        : NSColor.whiteColor;
    self.recordings.image.template = YES;
    self.recordings.contentTintColor = primary;
    self.capture.layer.backgroundColor = (recording
        ? NSColor.clearColor
        : ([status isEqualToString:@"error"]
            ? [self color:234 green:246 blue:234 darkRed:24 green:53 blue:34]
            : [self color:23 green:153 blue:143 darkRed:23 green:153 blue:143])).CGColor;
    self.capture.font = [NSFont systemFontOfSize:14 weight:NSFontWeightSemibold];
    self.capture.enabled = !transitioning && !self.quitting;
    NSDictionary *capabilities = snapshot[@"capabilities"];
    self.microphone.enabled = !transitioning && !self.quitting && [capabilities[@"microphone_capture"] boolValue];
    self.system.enabled = !transitioning && !self.quitting && [capabilities[@"system_audio_capture"] boolValue];
    self.systemCard.enabled = self.system.enabled;
    self.microphoneCard.enabled = self.microphone.enabled;
    self.bothCard.enabled = self.system.enabled && self.microphone.enabled;
    NSDictionary *mic = snapshot[@"microphone"];
    NSDictionary *system = snapshot[@"system_audio"];
    self.microphone.state = [mic[@"enabled"] boolValue] ? NSControlStateValueOn : NSControlStateValueOff;
    self.system.state = [system[@"enabled"] boolValue] ? NSControlStateValueOn : NSControlStateValueOff;
    self.transcription.enabled = !transitioning && [transcription[@"available"] boolValue];
    self.transcription.state = [transcription[@"enabled"] boolValue] ? NSControlStateValueOn : NSControlStateValueOff;
    self.transcription.title = self.transcription.enabled ? @"Transcription" : @"Transcription — model required";
    self.microphone.title = [self sourceTitle:@"Microphone" source:mic recording:recording];
    self.system.title = [self sourceTitle:@"System Audio" source:system recording:recording];
    BOOL microphoneEnabled = [mic[@"enabled"] boolValue];
    BOOL systemEnabled = [system[@"enabled"] boolValue];
    NSArray<NSButton *> *cards = @[self.systemCard, self.microphoneCard, self.bothCard];
    NSArray<NSNumber *> *selected = @[@(systemEnabled && !microphoneEnabled), @(microphoneEnabled && !systemEnabled), @(microphoneEnabled && systemEnabled)];
    for (NSUInteger index = 0; index < cards.count; index++) {
        NSButton *card = cards[index];
        BOOL active = selected[index].boolValue;
        card.layer.backgroundColor = [self color:(active ? 250 : 255) green:(active ? 252 : 255) blue:(active ? 252 : 255)
                                        darkRed:(active ? 19 : 24) green:(active ? 34 : 38) blue:(active ? 41 : 59)].CGColor;
        card.layer.borderWidth = active ? 2 : 1.2;
        card.layer.borderColor = (active ? [self color:23 green:153 blue:143 darkRed:63 green:200 blue:188]
                                        : [self color:213 green:214 blue:218 darkRed:45 green:61 blue:85]).CGColor;
        NSColor *contentColor = active ? [self color:23 green:153 blue:143 darkRed:63 green:200 blue:188]
                                       : [self color:100 green:100 blue:106 darkRed:179 green:182 blue:190];
        card.contentTintColor = contentColor;
        for (NSStackView *content in card.subviews) {
            for (NSView *contentView in content.subviews) {
                if ([contentView isKindOfClass:NSImageView.class]) ((NSImageView *)contentView).contentTintColor = contentColor;
                if ([contentView isKindOfClass:NSTextField.class]) ((NSTextField *)contentView).textColor = contentColor;
            }
        }
    }
    self.transcription.hidden = YES;
    self.language.hidden = NO;
    self.language.enabled = !recording && !transitioning && !self.quitting;
    self.model.enabled = !transitioning && !self.quitting;
    self.drops.stringValue = [NSString stringWithFormat:@"Dropped blocks: mic %@ · system %@",
                       snapshot[@"dropped_microphone_blocks"], snapshot[@"dropped_system_blocks"]];
    self.drops.hidden = [snapshot[@"dropped_microphone_blocks"] unsignedLongLongValue] == 0
        && [snapshot[@"dropped_system_blocks"] unsignedLongLongValue] == 0;
    id error = snapshot[@"last_error"];
    if ([error isKindOfClass:NSDictionary.class]) {
        [self showError:error[@"message"]];
    } else if (![status isEqualToString:@"error"]) {
        self.displayedError = nil;
        self.errorItem.hidden = YES;
        self.errorItem.title = @"Show Capture Error…";
    }
}
- (void)dealloc {
    if (self.keyMonitor) [NSEvent removeMonitor:self.keyMonitor];
    if (self.localClickMonitor) [NSEvent removeMonitor:self.localClickMonitor];
    if (self.globalClickMonitor) [NSEvent removeMonitor:self.globalClickMonitor];
}
- (NSString *)selectedSourceSummary:(NSDictionary *)mic system:(NSDictionary *)system {
    BOOL microphone = [mic[@"enabled"] boolValue];
    BOOL systemAudio = [system[@"enabled"] boolValue];
    if (microphone && systemAudio) return @"Mic + System";
    if (microphone) return @"Microphone";
    if (systemAudio) return @"System Audio";
    return @"None";
}
- (NSString *)sourceTitle:(NSString *)name source:(NSDictionary *)source recording:(BOOL)recording {
    if (![source[@"enabled"] boolValue]) return [name stringByAppendingString:@" — off"];
    if (recording && ![source[@"active"] boolValue]) return [name stringByAppendingString:@" — unavailable (click to disable)"];
    return [name stringByAppendingString:recording ? @" — recording" : @" — on"];
}
- (void)togglePopover:(id)sender {
    (void)sender;
    if (self.popover.shown) {
        [self.popover performClose:nil];
    } else {
        [self.popover showRelativeToRect:self.statusItem.button.bounds
                                  ofView:self.statusItem.button
                           preferredEdge:NSRectEdgeMinY];
    }
}
- (void)applicationDidResignActive:(NSNotification *)notification {
    (void)notification;
    [self.popover performClose:nil];
}
- (BOOL)popoverShouldDetach:(NSPopover *)popover {
    (void)popover;
    return NO;
}
- (void)showSources:(id)sender {
    NSMenu *sources = [[NSMenu alloc] initWithTitle:@"Audio Sources"];
    NSMenuItem *microphone = [[NSMenuItem alloc] initWithTitle:@"Microphone" action:@selector(toggleMicrophone:) keyEquivalent:@"m"];
    microphone.target = self;
    microphone.state = self.microphone.state;
    microphone.image = [NSImage imageWithSystemSymbolName:@"mic" accessibilityDescription:@"Microphone source"];
    [sources addItem:microphone];
    NSMenuItem *system = [[NSMenuItem alloc] initWithTitle:@"System Audio" action:@selector(toggleSystem:) keyEquivalent:@""];
    system.target = self;
    system.state = self.system.state;
    system.image = [NSImage imageWithSystemSymbolName:@"speaker.wave.2" accessibilityDescription:@"System audio source"];
    [sources addItem:system];
    [sources popUpMenuPositioningItem:nil atLocation:NSMakePoint(NSMinX(self.source.frame), NSMaxY(self.source.frame)) inView:self.popoverView];
    (void)sender;
}
- (void)selectSystem:(id)sender {
    (void)sender;
    self.command(3, 0);
    self.command(4, 1);
}
- (void)selectMicrophone:(id)sender {
    (void)sender;
    self.command(3, 1);
    self.command(4, 0);
}
- (void)selectBoth:(id)sender {
    (void)sender;
    self.command(3, 1);
    self.command(4, 1);
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
- (NSString *)languageTitle:(NSString *)code {
    if ([code isEqualToString:@"en"]) return @"English";
    if ([code isEqualToString:@"es"]) return @"Spanish";
    if ([code isEqualToString:@"fr"]) return @"French";
    if ([code isEqualToString:@"de"]) return @"German";
    if ([code isEqualToString:@"pt"]) return @"Portuguese";
    if ([code isEqualToString:@"it"]) return @"Italian";
    if ([code isEqualToString:@"ja"]) return @"Japanese";
    if ([code isEqualToString:@"zh"]) return @"Mandarin Chinese";
    if ([code isEqualToString:@"hi"]) return @"Hindi";
    if ([code isEqualToString:@"ar"]) return @"Arabic";
    if ([code isEqualToString:@"bg"]) return @"Bulgarian";
    if ([code isEqualToString:@"hr"]) return @"Croatian";
    if ([code isEqualToString:@"cs"]) return @"Czech";
    if ([code isEqualToString:@"da"]) return @"Danish";
    if ([code isEqualToString:@"nl"]) return @"Dutch";
    if ([code isEqualToString:@"et"]) return @"Estonian";
    if ([code isEqualToString:@"fi"]) return @"Finnish";
    if ([code isEqualToString:@"el"]) return @"Greek";
    if ([code isEqualToString:@"hu"]) return @"Hungarian";
    if ([code isEqualToString:@"lv"]) return @"Latvian";
    if ([code isEqualToString:@"lt"]) return @"Lithuanian";
    if ([code isEqualToString:@"mt"]) return @"Maltese";
    if ([code isEqualToString:@"pl"]) return @"Polish";
    if ([code isEqualToString:@"ro"]) return @"Romanian";
    if ([code isEqualToString:@"sk"]) return @"Slovak";
    if ([code isEqualToString:@"sl"]) return @"Slovenian";
    if ([code isEqualToString:@"sv"]) return @"Swedish";
    if ([code isEqualToString:@"uk"]) return @"Ukrainian";
    if ([code isEqualToString:@"ru"]) return @"Russian";
    return @"Auto Detect";
}
- (NSString *)languageFlag:(NSString *)code {
    NSDictionary<NSString *, NSString *> *flags = @{
        @"ar": @"🇸🇦", @"bg": @"🇧🇬", @"cs": @"🇨🇿", @"da": @"🇩🇰", @"de": @"🇩🇪",
        @"el": @"🇬🇷", @"en": @"🇺🇸", @"es": @"🇪🇸", @"et": @"🇪🇪", @"fi": @"🇫🇮",
        @"fr": @"🇫🇷", @"hi": @"🇮🇳", @"hr": @"🇭🇷", @"hu": @"🇭🇺", @"it": @"🇮🇹",
        @"ja": @"🇯🇵", @"lt": @"🇱🇹", @"lv": @"🇱🇻", @"mt": @"🇲🇹", @"nl": @"🇳🇱",
        @"pl": @"🇵🇱", @"pt": @"🇵🇹", @"ro": @"🇷🇴", @"ru": @"🇷🇺", @"sk": @"🇸🇰",
        @"sl": @"🇸🇮", @"sv": @"🇸🇪", @"uk": @"🇺🇦", @"zh": @"🇨🇳",
    };
    return flags[code] ?: @"🌐";
}
- (void)updateLanguageTitle {
    self.languageValue.stringValue = (self.supportedLanguages.count == 0 && !self.supportsLanguageDetection)
        ? @"Model required"
        : [self languageTitle:self.selectedLanguage];
    self.languageSummary.stringValue = self.languageValue.stringValue;
}
- (NSColor *)languageColor:(CGFloat)lightR green:(CGFloat)lightG blue:(CGFloat)lightB darkR:(CGFloat)darkR green:(CGFloat)darkG blue:(CGFloat)darkB {
    BOOL dark = RimvIsDark(self.languagePopoverView.effectiveAppearance ?: NSApp.effectiveAppearance);
    return [NSColor colorWithRed:(dark ? darkR : lightR) / 255.0
                           green:(dark ? darkG : lightG) / 255.0
                            blue:(dark ? darkB : lightB) / 255.0 alpha:1];
}
- (NSTextField *)languageLabel:(NSString *)text frame:(NSRect)frame size:(CGFloat)size weight:(NSFontWeight)weight color:(NSColor *)color {
    NSTextField *label = [NSTextField labelWithString:text];
    label.frame = frame;
    label.font = [NSFont systemFontOfSize:size weight:weight];
    label.textColor = color;
    label.lineBreakMode = NSLineBreakByTruncatingTail;
    return label;
}
- (void)buildLanguagePopover {
    if (self.languagePopover) return;
    self.languagePopoverView = [[RimvLanguageSelectorView alloc] initWithFrame:NSMakeRect(0, 0, 336, 466)];
    self.languagePopoverView.wantsLayer = YES;
    NSViewController *controller = [[NSViewController alloc] init];
    controller.view = self.languagePopoverView;
    self.languagePopover = [[NSPopover alloc] init];
    self.languagePopover.behavior = NSPopoverBehaviorTransient;
    self.languagePopover.delegate = self;
    self.languagePopover.contentSize = self.languagePopoverView.bounds.size;
    self.languagePopover.contentViewController = controller;

    NSColor *primary = [self languageColor:17 green:20 blue:24 darkR:242 green:246 blue:250];
    NSColor *secondary = [self languageColor:107 green:114 blue:128 darkR:174 green:186 blue:201];
    NSTextField *heading = [self languageLabel:@"Language" frame:NSMakeRect(20, 18, 150, 23) size:17 weight:NSFontWeightSemibold color:primary];
    self.languageSummary = [self languageLabel:self.languageValue.stringValue frame:NSMakeRect(185, 20, 130, 18) size:12 weight:NSFontWeightRegular color:secondary];
    self.languageSummary.alignment = NSTextAlignmentRight;
    [self.languagePopoverView addSubview:heading];
    [self.languagePopoverView addSubview:self.languageSummary];
    NSBox *separator = [[NSBox alloc] initWithFrame:NSMakeRect(18, 51, 300, 1)];
    separator.boxType = NSBoxSeparator;
    [self.languagePopoverView addSubview:separator];
    self.languageSearch = [[NSSearchField alloc] initWithFrame:NSMakeRect(18, 67, 300, 40)];
    self.languageSearch.placeholderString = @"Search languages…";
    self.languageSearch.delegate = self;
    self.languageSearch.sendsSearchStringImmediately = YES;
    self.languageSearch.accessibilityLabel = @"Search supported languages";
    self.languageSearch.wantsLayer = YES;
    self.languageSearch.layer.cornerRadius = 10;
    self.languageSearch.layer.borderWidth = 1;
    [self.languagePopoverView addSubview:self.languageSearch];
    NSScrollView *scroll = [[NSScrollView alloc] initWithFrame:NSMakeRect(18, 119, 300, 278)];
    scroll.hasVerticalScroller = YES;
    scroll.autohidesScrollers = YES;
    scroll.drawsBackground = NO;
    self.languageListDocument = [[NSView alloc] initWithFrame:NSMakeRect(0, 0, 300, 1)];
    scroll.documentView = self.languageListDocument;
    [self.languagePopoverView addSubview:scroll];
    self.allLanguagesButton = [NSButton buttonWithTitle:@"All supported languages  ›" target:self action:@selector(toggleAllLanguages:)];
    self.allLanguagesButton.frame = NSMakeRect(18, 405, 300, 28);
    self.allLanguagesButton.bordered = NO;
    self.allLanguagesButton.alignment = NSTextAlignmentLeft;
    self.allLanguagesButton.font = [NSFont systemFontOfSize:12 weight:NSFontWeightMedium];
    self.allLanguagesButton.contentTintColor = [self languageColor:20 green:156 blue:146 darkR:55 green:208 blue:199];
    self.allLanguagesButton.accessibilityLabel = @"Show all supported languages";
    [self.languagePopoverView addSubview:self.allLanguagesButton];
    NSTextField *hint = [self languageLabel:@"Only languages supported by the selected transcription model are shown." frame:NSMakeRect(22, 435, 292, 22) size:11 weight:NSFontWeightRegular color:secondary];
    hint.maximumNumberOfLines = 2;
    [self.languagePopoverView addSubview:hint];
    __weak RimvMenu *weakSelf = self;
    self.languagePopoverView.appearanceChanged = ^{
        RimvMenu *strongSelf = weakSelf;
        if (strongSelf) [strongSelf reloadLanguageOptions];
    };
}
- (void)addLanguageSection:(NSString *)title atY:(CGFloat *)y {
    NSTextField *label = [self languageLabel:title frame:NSMakeRect(8, *y, 284, 16) size:11 weight:NSFontWeightBold color:[self languageColor:94 green:104 blue:120 darkR:164 green:181 blue:200]];
    label.stringValue = title.uppercaseString;
    [self.languageListDocument addSubview:label];
    *y += 21;
}
- (void)addLanguageOption:(NSString *)code atY:(CGFloat *)y {
    BOOL selected = [code isEqualToString:self.selectedLanguage];
    NSButton *button = [NSButton buttonWithTitle:@"" target:self action:@selector(selectLanguageButton:)];
    button.frame = NSMakeRect(0, *y, 292, 40);
    button.bordered = NO;
    button.wantsLayer = YES;
    button.layer.cornerRadius = 8;
    button.layer.backgroundColor = (selected
        ? [self languageColor:229 green:245 blue:243 darkR:38 green:61 blue:80]
        : NSColor.clearColor).CGColor;
    button.identifier = code;
    button.accessibilityLabel = [self languageTitle:code];
    button.accessibilityValue = selected ? @"Selected" : @"Not selected";
    CGFloat nameX = 40;
    if ([code isEqualToString:@"auto"]) {
        NSImageView *icon = [[NSImageView alloc] initWithFrame:NSMakeRect(10, 11, 18, 18)];
        icon.image = [[NSImage imageWithSystemSymbolName:@"wand.and.stars" accessibilityDescription:nil]
            imageWithSymbolConfiguration:[NSImageSymbolConfiguration configurationWithPointSize:15 weight:NSFontWeightRegular]];
        icon.image.template = YES;
        icon.contentTintColor = selected ? [self languageColor:20 green:156 blue:146 darkR:55 green:208 blue:199] : [self languageColor:107 green:114 blue:128 darkR:174 green:186 blue:201];
        [button addSubview:icon];
    } else {
        NSTextField *flag = [self languageLabel:[self languageFlag:code] frame:NSMakeRect(10, 9, 22, 22) size:16 weight:NSFontWeightRegular color:NSColor.labelColor];
        [button addSubview:flag];
    }
    NSTextField *name = [self languageLabel:[self languageTitle:code] frame:NSMakeRect(nameX, 10, 205, 20) size:14 weight:(selected ? NSFontWeightMedium : NSFontWeightRegular) color:[self languageColor:17 green:20 blue:24 darkR:242 green:246 blue:250]];
    [button addSubview:name];
    if (selected) {
        NSImageView *check = [[NSImageView alloc] initWithFrame:NSMakeRect(264, 11, 18, 18)];
        check.image = [[NSImage imageWithSystemSymbolName:@"checkmark" accessibilityDescription:@"Selected"] imageWithSymbolConfiguration:[NSImageSymbolConfiguration configurationWithPointSize:15 weight:NSFontWeightBold]];
        check.image.template = YES;
        check.contentTintColor = [self languageColor:20 green:156 blue:146 darkR:55 green:208 blue:199];
        [button addSubview:check];
    }
    [self.languageListDocument addSubview:button];
    *y += 40;
}
- (void)reloadLanguageOptions {
    if (!self.languageListDocument) return;
    self.languageSearch.backgroundColor = [self languageColor:245 green:247 blue:249 darkR:15 green:30 blue:48];
    self.languageSearch.textColor = [self languageColor:17 green:20 blue:24 darkR:242 green:246 blue:250];
    self.languageSearch.layer.borderColor = [self languageColor:229 green:231 blue:235 darkR:59 green:77 blue:100].CGColor;
    for (NSView *view in self.languageListDocument.subviews.copy) [view removeFromSuperview];
    NSString *query = self.languageSearch.stringValue.lowercaseString;
    NSMutableArray<NSString *> *codes = [NSMutableArray array];
    if (self.supportsLanguageDetection) [codes addObject:@"auto"];
    for (NSString *code in self.supportedLanguages) if ([self languageCommand:code] != 0) [codes addObject:code];
    NSPredicate *matches = [NSPredicate predicateWithBlock:^BOOL(NSString *code, NSDictionary *bindings) {
        (void)bindings;
        return query.length == 0 || [[self languageTitle:code].lowercaseString containsString:query] || [code containsString:query];
    }];
    NSArray<NSString *> *visible = [codes filteredArrayUsingPredicate:matches];
    NSArray<NSString *> *popularOrder = @[@"en", @"es", @"fr", @"de", @"pt", @"it", @"nl", @"pl", @"ru", @"uk"];
    NSMutableArray<NSString *> *popular = [NSMutableArray array];
    if ([visible containsObject:@"auto"]) [popular addObject:@"auto"];
    for (NSString *code in popularOrder) if ([visible containsObject:code]) [popular addObject:code];
    NSMutableArray<NSString *> *remaining = [visible mutableCopy];
    [remaining removeObjectsInArray:popular];
    BOOL searching = query.length > 0;
    BOOL hasAdditionalLanguages = remaining.count > 0;
    if (!self.showAllLanguages && !searching) [remaining removeAllObjects];
    self.allLanguagesButton.hidden = searching || (!hasAdditionalLanguages && !self.showAllLanguages);
    self.allLanguagesButton.title = self.showAllLanguages ? @"Show popular languages  ⌃" : @"All supported languages  ›";
    self.allLanguagesButton.accessibilityLabel = self.showAllLanguages ? @"Show popular languages" : @"Show all supported languages";
    CGFloat y = 0;
    if (popular.count) {
        [self addLanguageSection:@"Popular languages" atY:&y];
        for (NSString *code in popular) [self addLanguageOption:code atY:&y];
    }
    if (remaining.count) {
        if (y) y += 8;
        [self addLanguageSection:@"All supported languages" atY:&y];
        for (NSString *code in remaining) [self addLanguageOption:code atY:&y];
    }
    if (!visible.count) {
        NSString *message = codes.count
            ? @"No matching supported languages"
            : @"Select a transcription model to choose a language.";
        NSTextField *empty = [self languageLabel:message frame:NSMakeRect(10, 12, 270, 36) size:13 weight:NSFontWeightRegular color:[self languageColor:107 green:114 blue:128 darkR:174 green:186 blue:201]];
        empty.maximumNumberOfLines = 2;
        [self.languageListDocument addSubview:empty];
        y = 56;
    }
    self.languageListDocument.frame = NSMakeRect(0, 0, 300, MAX(y, 1));
}
- (void)toggleAllLanguages:(id)sender {
    (void)sender;
    self.showAllLanguages = !self.showAllLanguages;
    [self reloadLanguageOptions];
}
- (void)showLanguages:(id)sender {
    (void)sender;
    if (!self.language.enabled) return;
    [self buildLanguagePopover];
    self.languageSearch.stringValue = @"";
    [self reloadLanguageOptions];
    [self.languagePopover showRelativeToRect:self.language.bounds ofView:self.language preferredEdge:NSRectEdgeMaxX];
    [self.languagePopover.contentViewController.view.window makeFirstResponder:self.languageSearch];
}
- (void)controlTextDidChange:(NSNotification *)notification {
    if (notification.object == self.languageSearch) [self reloadLanguageOptions];
}
- (uint32_t)languageCommand:(NSString *)code {
    NSDictionary<NSString *, NSNumber *> *commands = @{
        @"auto": @10, @"en": @11, @"es": @12, @"fr": @13, @"de": @14, @"pt": @15,
        @"it": @16, @"ja": @17, @"zh": @18, @"hi": @19, @"ar": @20, @"ru": @21,
        @"bg": @22, @"hr": @23, @"cs": @24, @"da": @25, @"nl": @26, @"et": @27,
        @"fi": @28, @"el": @29, @"hu": @30, @"lv": @31, @"lt": @32, @"mt": @33,
        @"pl": @34, @"ro": @35, @"sk": @36, @"sl": @37, @"sv": @38, @"uk": @39,
    };
    return [commands[code] unsignedIntValue];
}
- (void)selectLanguageButton:(NSButton *)sender {
    self.selectedLanguage = sender.identifier;
    [self updateLanguageTitle];
    self.command([self languageCommand:self.selectedLanguage], 0);
    [self.languagePopover performClose:nil];
}
- (void)showError:(NSString *)message {
    self.displayedError = message;
    self.errorItem.hidden = NO;
    self.errorItem.title = message;
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
- (void)showAbout:(id)sender {
    (void)sender;
    [NSApp orderFrontStandardAboutPanel:nil];
    [NSApp activateIgnoringOtherApps:YES];
}
- (void)showModels:(id)sender {
    (void)sender;
    rimv_model_manager_show_selector(self.model);
}
- (void)openPermissions:(id)sender {
    (void)sender;
    [NSWorkspace.sharedWorkspace openURL:[NSURL URLWithString:@"x-apple.systempreferences:com.apple.preference.security?Privacy"]];
}
- (void)quit:(id)sender {
    (void)sender;
    if (self.quitting) return;
    self.quitting = YES;
    self.statusLine.stringValue = @"Finalizing recordings…";
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
    });
}

void rimv_menu_set_language(const char *language) {
    NSString *selected = [NSString stringWithUTF8String:language];
    dispatch_async(dispatch_get_main_queue(), ^{
        menu.selectedLanguage = selected ?: @"auto";
        [menu updateLanguageTitle];
    });
}

void rimv_menu_set_selector_state(const char *state) {
    NSData *data = [[NSString stringWithUTF8String:state] dataUsingEncoding:NSUTF8StringEncoding];
    NSDictionary *selectors = [NSJSONSerialization JSONObjectWithData:data options:0 error:NULL];
    if (![selectors isKindOfClass:NSDictionary.class]) return;
    dispatch_async(dispatch_get_main_queue(), ^{
        menu.modelValue.stringValue = selectors[@"model"] ?: @"Model required";
        menu.supportedLanguages = selectors[@"languages"] ?: @[];
        menu.supportsLanguageDetection = [selectors[@"auto_detect"] boolValue];
        if (![menu.selectedLanguage isEqualToString:@"auto"] && ![menu.supportedLanguages containsObject:menu.selectedLanguage]) {
            menu.selectedLanguage = @"auto";
        }
        [menu updateLanguageTitle];
        [menu reloadLanguageOptions];
        [menu applySnapshot:menu.snapshot];
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
        passed &= [menu.captureLabel.stringValue isEqualToString:@"Start Listening"];
        passed &= [menu.statusLine.stringValue isEqualToString:@"Ready"];
        menu.supportedLanguages = @[@"en", @"es", @"ru"];
        menu.supportsLanguageDetection = YES;
        menu.modelValue.stringValue = @"Whisper Tiny";
        [menu applySnapshot:state];
        passed &= menu.language.enabled && [menu.modelValue.stringValue isEqualToString:@"Whisper Tiny"];
        [menu buildLanguagePopover];
        menu.languageSearch.stringValue = @"span";
        [menu reloadLanguageOptions];
        passed &= menu.languageListDocument.subviews.count == 2;
        NSButton *spanish = [NSButton buttonWithTitle:@"" target:nil action:NULL];
        spanish.identifier = @"es";
        [menu selectLanguageButton:spanish];
        passed &= testedCommand == 12 && [menu.languageValue.stringValue isEqualToString:@"Spanish"];
        menu.supportedLanguages = @[@"bg", @"hr", @"cs", @"da", @"nl", @"en", @"et", @"fi", @"fr", @"de", @"el", @"hu", @"it", @"lv", @"lt", @"mt", @"pl", @"pt", @"ro", @"sk", @"sl", @"es", @"sv", @"ru", @"uk"];
        menu.showAllLanguages = NO;
        menu.languageSearch.stringValue = @"";
        [menu reloadLanguageOptions];
        passed &= !menu.allLanguagesButton.hidden;
        NSButton *ukrainian = [NSButton buttonWithTitle:@"" target:nil action:NULL];
        ukrainian.identifier = @"uk";
        [menu selectLanguageButton:ukrainian];
        passed &= testedCommand == 39 && [menu.languageValue.stringValue isEqualToString:@"Ukrainian"];
        passed &= menu.microphone.state == NSControlStateValueOn;
        [menu toggleCapture:nil];
        passed &= testedCommand == 1;
        state[@"status"] = @"starting";
        state[@"transcription"] = @{@"available": @YES, @"enabled": @YES, @"status": @"loading"};
        [menu applySnapshot:state];
        passed &= !menu.capture.enabled && !menu.microphone.enabled;
        passed &= [menu.statusLine.stringValue isEqualToString:@"Preparing transcription…"];
        state[@"status"] = @"recording";
        state[@"elapsed_ms"] = @754000;
        state[@"microphone"] = @{@"enabled": @YES, @"active": @YES};
        [menu applySnapshot:state];
        passed &= [menu.statusLine.stringValue isEqualToString:@"Listening · 00:12:34"];
        passed &= [menu.captureLabel.stringValue isEqualToString:@"Stop Listening"];
        passed &= !menu.language.enabled;
        [menu toggleCapture:nil];
        passed &= testedCommand == 2;
        state[@"status"] = @"stopping";
        [menu applySnapshot:state];
        passed &= [menu.statusLine.stringValue isEqualToString:@"Saving transcript…"] && !menu.capture.enabled;
        state[@"status"] = @"recording";
        [menu applySnapshot:state];
        [menu toggleMicrophone:nil];
        passed &= testedCommand == 3 && testedEnabled == 0;
        state[@"microphone"] = @{@"enabled": @NO, @"active": @NO};
        [menu applySnapshot:state];
        passed &= menu.microphone.state == NSControlStateValueOff;
        passed &= [menu.statusLine.stringValue hasPrefix:@"Listening"];
        [menu toggleMicrophone:nil];
        passed &= testedCommand == 3 && testedEnabled == 1;
        [menu toggleSystem:nil];
        passed &= testedCommand == 4 && testedEnabled == 1;
        state[@"system_audio"] = @{@"enabled": @YES, @"active": @NO};
        [menu applySnapshot:state];
        passed &= [menu.system.title containsString:@"unavailable"];
        state[@"last_error"] = @{@"message": @"Permission denied"};
        state[@"status"] = @"error";
        [menu applySnapshot:state];
        passed &= !menu.errorItem.hidden && [menu.displayedError isEqualToString:@"Permission denied"];
        passed &= [menu.statusLine.stringValue isEqualToString:@"Capture needs attention"];
        passed &= [menu.captureLabel.stringValue isEqualToString:@"Retry Listening"];
        state[@"status"] = @"idle";
        state[@"last_error"] = [NSNull null];
        [menu applySnapshot:state];
        passed &= [menu.captureLabel.stringValue isEqualToString:@"Start Listening"];
        passed &= [menu.statusLine.stringValue isEqualToString:@"Ready"];
        [menu quit:nil];
        passed &= testedCommand == 5 && !menu.capture.enabled;
        menu.command = original;
        fprintf(stdout, "Native menu self-test: %s\n", passed ? "PASS" : "FAIL");
        return passed;
    }
}
