#import <AppKit/AppKit.h>
#import <signal.h>
#import "model_manager.h"
#import "transcription_window.h"
#import "recordings_selector.h"
#import "hand_cursor_button.h"

typedef void (*CommandCallback)(uint32_t, uint8_t);
typedef void (*LanguageCommandCallback)(const char *);
static const CGFloat RimvMainContentInset = 12.0;
static const CGFloat RimvMainContentWidth = 380.0;
static const CGFloat RimvMainContentHeight = 560.0;

static BOOL RimvIsDark(NSAppearance *appearance) {
    return [[appearance bestMatchFromAppearancesWithNames:@[
        NSAppearanceNameAqua, NSAppearanceNameDarkAqua
    ]] isEqualToString:NSAppearanceNameDarkAqua];
}

@implementation RimvHandCursorButton
- (void)resetCursorRects {
    [super resetCursorRects];
    [self addCursorRect:self.bounds cursor:NSCursor.pointingHandCursor];
}
@end

@interface RimvPopoverView : NSView
@property(nonatomic, copy) dispatch_block_t appearanceChanged;
@end

@interface RimvLanguageSelectorView : NSView
@property(nonatomic, copy) dispatch_block_t appearanceChanged;
@end

@interface RimvFlippedContentView : NSView
@end

@implementation RimvFlippedContentView
- (BOOL)isFlipped { return YES; }
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
    // NSPopover already supplies and clips the outer rounded body and arrow.
    // Painting a second rounded panel over a contrasting root color exposes
    // that root at AppKit's safe-area/antialiasing edges as apparent padding.
    // Fill the complete content bounds with one surface instead.
    NSColor *panel = dark
        ? [NSColor colorWithRed:16.0/255.0 green:26.0/255.0 blue:43.0/255.0 alpha:1]
        : NSColor.whiteColor;
    [panel setFill];
    NSRectFill(self.bounds);
}
@end

@interface RimvMenu : NSObject <NSApplicationDelegate, NSPopoverDelegate, NSSearchFieldDelegate>
@property(nonatomic) CommandCallback command;
@property(nonatomic) LanguageCommandCallback languageCommand;
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
@property(nonatomic, strong) NSView *errorCard;
@property(nonatomic, strong) NSTextField *errorTitle;
@property(nonatomic, strong) NSTextField *errorMessage;
@property(nonatomic) BOOL errorDismissed;
@property(nonatomic, strong) NSPopover *popover;
@property(nonatomic, strong) RimvPopoverView *popoverView;
@property(nonatomic, strong) RimvFlippedContentView *mainContentView;
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
@property(nonatomic, strong) NSTextField *languageFlagIcon;
@property(nonatomic, strong) NSImageView *modelIcon;
@property(nonatomic, strong) NSImageView *languageChevron;
@property(nonatomic, strong) NSImageView *modelChevron;
@property(nonatomic, strong) NSImageView *recordingsIcon;
@property(nonatomic, strong) NSImageView *recordingsChevron;
@property(nonatomic, copy) NSString *selectedLanguage;
@property(nonatomic, copy) NSString *effectiveModelTitle;
@property(nonatomic, copy) NSArray<NSString *> *supportedLanguages;
@property(nonatomic, copy) NSDictionary<NSString *, NSDictionary *> *languagePresentations;
@property(nonatomic) BOOL supportsLanguageDetection;
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
@property(nonatomic, strong) NSButton *quitButton;
@property(nonatomic, strong) NSDictionary *snapshot;
@property(nonatomic, copy) NSString *root;
@property(nonatomic, copy) NSString *displayedError;
@property(nonatomic) BOOL quitting;
@property(nonatomic) BOOL systemTermination;
@property(nonatomic) NSInteger pendingSelector;
@property(nonatomic) NSInteger activeSelector;
@property(nonatomic) NSInteger closingSelector;
@property(nonatomic) NSUInteger interactionGeneration;
@property(nonatomic, strong) dispatch_source_t interruptSignal;
@property(nonatomic, strong) dispatch_source_t terminateSignal;
- (NSDictionary *)languagePresentation:(NSString *)code;
- (void)applySnapshot:(NSDictionary *)snapshot;
- (void)showError:(NSString *)message;
- (void)setErrorCardVisible:(BOOL)visible;
- (void)requestSelector:(NSInteger)selector;
- (void)openPendingSelector;
- (void)selectorDidClose:(NSInteger)selector;
- (BOOL)closeSelector:(NSInteger)selector reason:(NSString *)reason;
- (BOOL)releasePopoverFocusInWindow:(NSWindow *)window selector:(NSInteger)selector;
- (void)dismissSelectorsForReason:(NSString *)reason;
- (void)logSelectorEvent:(NSString *)event selector:(NSInteger)selector reason:(NSString *)reason;
@end

static RimvMenu *menu;
static NSLock *mailboxLock;
static NSDictionary *pendingSnapshot;
static NSString *pendingError;
static BOOL updateScheduled;
void rimv_menu_selector_did_close(NSInteger selector) {
    dispatch_async(dispatch_get_main_queue(), ^{ [menu selectorDidClose:selector]; });
}
bool rimv_menu_release_popover_focus(NSWindow *window, NSInteger selector) {
    return [menu releasePopoverFocusInWindow:window selector:selector];
}

static void RimvSelectorUncaughtException(NSException *exception) {
    if (![NSProcessInfo.processInfo.environment[@"RIMV_SELECTOR_DIAGNOSTICS"] boolValue]) return;
    NSLog(@"[RimV selectors] uncaught Objective-C exception=%@ backtrace=%@",
          exception, exception.callStackSymbols);
}

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
    [self.mainContentView addSubview:label];
    return label;
}
- (NSButton *)row:(NSString *)title symbol:(NSString *)symbol action:(SEL)action frame:(NSRect)frame {
    NSButton *button = [RimvHandCursorButton buttonWithTitle:title target:self action:action];
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
    [self.mainContentView addSubview:button];
    return button;
}
- (void)separatorAt:(CGFloat)y {
    NSBox *separator = [[NSBox alloc] initWithFrame:NSMakeRect(16, y, 348, 1)];
    separator.boxType = NSBoxSeparator;
    [self.mainContentView addSubview:separator];
}
- (NSButton *)selectorRow:(NSString *)labelText symbol:(NSString *)symbol action:(SEL)action frame:(NSRect)frame value:(NSTextField **)valueOut icon:(NSImageView **)iconOut chevron:(NSImageView **)chevronOut {
    NSButton *button = [self row:@"" symbol:@"" action:action frame:frame];
    button.accessibilityLabel = labelText;
    NSImageView *icon = [[NSImageView alloc] initWithFrame:NSZeroRect];
    if (symbol.length > 0) {
        icon.image = [[NSImage imageWithSystemSymbolName:symbol accessibilityDescription:labelText]
            imageWithSymbolConfiguration:[NSImageSymbolConfiguration configurationWithPointSize:16 weight:NSFontWeightRegular]];
        icon.image.template = YES;
    }
    icon.translatesAutoresizingMaskIntoConstraints = NO;
    NSTextField *label = [NSTextField labelWithString:labelText];
    label.font = [NSFont systemFontOfSize:14 weight:NSFontWeightRegular];
    label.translatesAutoresizingMaskIntoConstraints = NO;
    NSTextField *value = nil;
    if (valueOut) {
        value = [NSTextField labelWithString:@""];
        value.font = [NSFont systemFontOfSize:14 weight:NSFontWeightRegular];
        value.alignment = NSTextAlignmentRight;
        value.lineBreakMode = NSLineBreakByTruncatingHead;
        value.translatesAutoresizingMaskIntoConstraints = NO;
        [value setContentCompressionResistancePriority:NSLayoutPriorityDefaultLow forOrientation:NSLayoutConstraintOrientationHorizontal];
        [value setContentHuggingPriority:NSLayoutPriorityDefaultLow forOrientation:NSLayoutConstraintOrientationHorizontal];
    }
    NSImageView *chevron = [[NSImageView alloc] initWithFrame:NSZeroRect];
    chevron.image = [[NSImage imageWithSystemSymbolName:@"chevron.right" accessibilityDescription:@"Choose"]
        imageWithSymbolConfiguration:[NSImageSymbolConfiguration configurationWithPointSize:12 weight:NSFontWeightSemibold]];
    chevron.image.template = YES;
    chevron.translatesAutoresizingMaskIntoConstraints = NO;
    [button addSubview:icon];
    [button addSubview:label];
    if (value) [button addSubview:value];
    [button addSubview:chevron];
    NSMutableArray<NSLayoutConstraint *> *constraints = [NSMutableArray arrayWithArray:@[
        [icon.leadingAnchor constraintEqualToAnchor:button.leadingAnchor constant:12],
        [icon.centerYAnchor constraintEqualToAnchor:button.centerYAnchor],
        [icon.widthAnchor constraintEqualToConstant:16], [icon.heightAnchor constraintEqualToConstant:16],
        [label.leadingAnchor constraintEqualToAnchor:icon.trailingAnchor constant:10],
        [label.centerYAnchor constraintEqualToAnchor:button.centerYAnchor],
        [chevron.trailingAnchor constraintEqualToAnchor:button.trailingAnchor constant:-12],
        [chevron.centerYAnchor constraintEqualToAnchor:button.centerYAnchor],
        [chevron.widthAnchor constraintEqualToConstant:12], [chevron.heightAnchor constraintEqualToConstant:12],
    ]];
    [label setContentCompressionResistancePriority:NSLayoutPriorityRequired forOrientation:NSLayoutConstraintOrientationHorizontal];
    if (value) {
        [constraints addObjectsFromArray:@[
            [value.trailingAnchor constraintEqualToAnchor:chevron.leadingAnchor constant:-8],
            [value.leadingAnchor constraintGreaterThanOrEqualToAnchor:label.trailingAnchor constant:12],
            [value.centerYAnchor constraintEqualToAnchor:button.centerYAnchor],
        ]];
    }
    [NSLayoutConstraint activateConstraints:constraints];
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
    self.statusItem.button.image = [NSImage imageNamed:@"MenuBarIcon"];
    self.statusItem.button.image.template = YES;
    self.statusItem.button.imagePosition = NSImageLeft;
    self.statusItem.button.font = [NSFont monospacedDigitSystemFontOfSize:12 weight:NSFontWeightRegular];
    self.statusItem.button.title = @" RimV";
    self.statusItem.button.target = self;
    self.statusItem.button.action = @selector(togglePopover:);
    self.popoverView = [[RimvPopoverView alloc] initWithFrame:NSMakeRect(
        0, 0,
        RimvMainContentWidth + (RimvMainContentInset * 2),
        RimvMainContentHeight + (RimvMainContentInset * 2))];
    self.popoverView.wantsLayer = YES;
    self.mainContentView = [[RimvFlippedContentView alloc] initWithFrame:NSMakeRect(
        RimvMainContentInset, RimvMainContentInset,
        RimvMainContentWidth, RimvMainContentHeight)];
    [self.popoverView addSubview:self.mainContentView];
    NSViewController *controller = [[NSViewController alloc] init];
    controller.view = self.popoverView;
    self.popover = [[NSPopover alloc] init];
    SEL fullSizeContent = NSSelectorFromString(@"setHasFullSizeContent:");
    if ([self.popover respondsToSelector:fullSizeContent]) {
        // Fill the popover window, including its chevron-safe region. Without
        // this AppKit reserves top/trailing space around the content view,
        // which appears as asymmetric padding on current macOS releases. KVC
        // keeps the macOS 13 deployment build free of availability-runtime
        // linker symbols while the selector check protects older systems.
        [self.popover setValue:@YES forKey:@"hasFullSizeContent"];
    }
    // Child selectors are separate AppKit windows. A transient parent treats
    // a click in those windows as an outside click and closes mid-switch.
    // Outside dismissal is handled explicitly by the scoped event monitors.
    self.popover.behavior = NSPopoverBehaviorApplicationDefined;
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
            if (strongSelf.pendingSelector != 0) {
                [strongSelf logSelectorEvent:@"pending-selector-cleared" selector:strongSelf.pendingSelector reason:@"Escape cancelled pending transition"];
            }
            strongSelf.pendingSelector = 0;
            NSInteger visible = [strongSelf visibleSelector];
            if (visible != 0) {
                [strongSelf logSelectorEvent:@"escape-close-child" selector:visible reason:@"Escape key"];
                [strongSelf closeSelector:visible reason:@"Escape key"];
                return nil;
            }
            [strongSelf logSelectorEvent:@"escape-close-parent" selector:0 reason:@"Escape key"];
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
    self.globalClickMonitor = [NSEvent addGlobalMonitorForEventsMatchingMask:(NSEventMaskLeftMouseDown | NSEventMaskRightMouseDown) handler:^(__unused NSEvent *event) {
        RimvMenu *strongSelf = weakSelf;
        if (!strongSelf) return;
        NSUInteger observedGeneration = strongSelf.interactionGeneration;
        dispatch_async(dispatch_get_main_queue(), ^{
            // Global monitors observe clicks in other applications. Ignore a
            // stale event if a newer local interaction has since reopened or
            // switched RimV's popovers.
            if (strongSelf.interactionGeneration != observedGeneration) {
                [strongSelf logSelectorEvent:@"ignore-stale-global-click" selector:strongSelf.activeSelector reason:@"a newer local interaction occurred"];
                return;
            }
            [strongSelf logSelectorEvent:@"global-outside-click" selector:strongSelf.activeSelector reason:@"dismiss active selector and parent"];
            [strongSelf dismissSelectorsForReason:@"global outside click"];
        });
    }];
    self.localClickMonitor = [NSEvent addLocalMonitorForEventsMatchingMask:(NSEventMaskLeftMouseDown | NSEventMaskRightMouseDown)
                                                                      handler:^NSEvent *(NSEvent *event) {
        RimvMenu *strongSelf = weakSelf;
        if (!strongSelf) return event;
        strongSelf.interactionGeneration += 1;
        NSWindow *popoverWindow = strongSelf.popover.contentViewController.view.window;
        NSWindow *languageWindow = strongSelf.languagePopover.contentViewController.view.window;
        NSWindow *statusWindow = strongSelf.statusItem.button.window;
        if (event.window && event.window == statusWindow) return event;
        // The language selector is a separate AppKit window, just like the
        // Model and Recordings selectors. Treat its controls as inside clicks
        // or the outside-click monitor closes both popovers before the action
        // can complete.
        if (strongSelf.languagePopover.shown && event.window == languageWindow) {
            [strongSelf logSelectorEvent:@"internal-click" selector:1 reason:@"Language selector window"];
            return event;
        }
        if (rimv_recordings_selector_contains_window(event.window)) {
            [strongSelf logSelectorEvent:@"internal-click" selector:3 reason:@"Recordings selector window"];
            return event;
        }
        if (rimv_model_manager_selector_contains_window(event.window)) {
            [strongSelf logSelectorEvent:@"internal-click" selector:2 reason:@"Model selector window"];
            return event;
        }
        if (event.window == popoverWindow) {
            [strongSelf logSelectorEvent:@"parent-click" selector:strongSelf.activeSelector reason:@"main popover window"];
            return event;
        }
        [strongSelf logSelectorEvent:@"local-outside-click" selector:strongSelf.activeSelector reason:[NSString stringWithFormat:@"window=%p", event.window]];
        [strongSelf dismissSelectorsForReason:@"local outside click"];
        return event;
    }];

    NSView *mark = [[NSView alloc] initWithFrame:NSMakeRect(16, 34, 40, 40)];
    mark.wantsLayer = YES;
    mark.layer.cornerRadius = 11;
    mark.layer.backgroundColor = NSColor.clearColor.CGColor;
    [self.mainContentView addSubview:mark];
    NSImageView *markImage = [[NSImageView alloc] initWithFrame:NSMakeRect(8, 8, 24, 24)];
    markImage.image = [NSImage imageNamed:@"RimVMark"];
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
    [self.mainContentView addSubview:self.captureArea];
    self.source = [self sourceCard:@"System" symbol:@"desktopcomputer" action:@selector(selectSystem:) frame:NSMakeRect(28, 160, 103, 74)];
    self.systemCard = self.source;
    self.microphoneCard = [self sourceCard:@"Microphone" symbol:@"mic" action:@selector(selectMicrophone:) frame:NSMakeRect(139, 160, 103, 74)];
    self.bothCard = [self sourceCard:@"Both" symbol:@"waveform" action:@selector(selectBoth:) frame:NSMakeRect(250, 160, 103, 74)];
    self.sourceDetail = [self label:@"CAPTURE SOURCE" frame:NSMakeRect(28, 134, 180, 16) size:12 weight:NSFontWeightBold];
    self.microphone = [RimvHandCursorButton buttonWithTitle:@"Microphone" target:self action:@selector(toggleMicrophone:)];
    self.system = [RimvHandCursorButton buttonWithTitle:@"System Audio" target:self action:@selector(toggleSystem:)];
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
    self.errorCard = [[RimvFlippedContentView alloc] initWithFrame:NSMakeRect(20, 312, 340, 116)];
    self.errorCard.wantsLayer = YES;
    self.errorCard.layer.cornerRadius = 12;
    self.errorCard.layer.borderWidth = 1;
    self.errorCard.hidden = YES;
    [self.mainContentView addSubview:self.errorCard];
    NSImageView *errorIcon = [[NSImageView alloc] initWithFrame:NSMakeRect(14, 15, 22, 22)];
    errorIcon.image = [NSImage imageWithSystemSymbolName:@"exclamationmark.circle.fill" accessibilityDescription:@"Capture error"];
    errorIcon.contentTintColor = NSColor.systemRedColor;
    [self.errorCard addSubview:errorIcon];
    self.errorTitle = [NSTextField labelWithString:@"No active source"];
    self.errorTitle.frame = NSMakeRect(47, 12, 245, 22);
    self.errorTitle.font = [NSFont systemFontOfSize:15 weight:NSFontWeightSemibold];
    [self.errorCard addSubview:self.errorTitle];
    NSButton *dismissError = [RimvHandCursorButton buttonWithImage:[NSImage imageWithSystemSymbolName:@"xmark" accessibilityDescription:@"Dismiss capture error"] target:self action:@selector(dismissError:)];
    dismissError.frame = NSMakeRect(302, 10, 26, 26);
    dismissError.bordered = NO;
    [self.errorCard addSubview:dismissError];
    self.errorMessage = [NSTextField wrappingLabelWithString:@""];
    self.errorMessage.frame = NSMakeRect(47, 36, 274, 38);
    self.errorMessage.font = [NSFont systemFontOfSize:12 weight:NSFontWeightRegular];
    [self.errorCard addSubview:self.errorMessage];
    NSButton *soundSettings = [RimvHandCursorButton buttonWithTitle:@"Open Sound Settings" target:self action:@selector(openPermissions:)];
    soundSettings.frame = NSMakeRect(47, 78, 156, 28);
    soundSettings.bezelStyle = NSBezelStyleRounded;
    [self.errorCard addSubview:soundSettings];
    NSTextField *languageValue;
    NSImageView *languageIcon;
    NSImageView *languageChevron;
    self.language = [self selectorRow:@"Language" symbol:@"" action:@selector(showLanguages:) frame:NSMakeRect(30, 319, 344, 36) value:&languageValue icon:&languageIcon chevron:&languageChevron];
    self.languageValue = languageValue;
    self.languageIcon = languageIcon;
    self.languageIcon.hidden = YES;
    self.languageFlagIcon = [NSTextField labelWithString:@"🌐"];
    self.languageFlagIcon.font = [NSFont fontWithName:@"Apple Color Emoji" size:16] ?: [NSFont systemFontOfSize:16];
    self.languageFlagIcon.alignment = NSTextAlignmentCenter;
    self.languageFlagIcon.translatesAutoresizingMaskIntoConstraints = NO;
    [self.language addSubview:self.languageFlagIcon];
    [NSLayoutConstraint activateConstraints:@[
        [self.languageFlagIcon.leadingAnchor constraintEqualToAnchor:self.language.leadingAnchor constant:10],
        [self.languageFlagIcon.centerYAnchor constraintEqualToAnchor:self.language.centerYAnchor],
        [self.languageFlagIcon.widthAnchor constraintEqualToConstant:20],
        [self.languageFlagIcon.heightAnchor constraintEqualToConstant:20],
    ]];
    self.languageChevron = languageChevron;
    NSTextField *modelValue;
    NSImageView *modelIcon;
    NSImageView *modelChevron;
    self.model = [self selectorRow:@"Model" symbol:@"cpu" action:@selector(showModels:) frame:NSMakeRect(30, 363, 344, 36) value:&modelValue icon:&modelIcon chevron:&modelChevron];
    self.modelValue = modelValue;
    self.modelValue.stringValue = self.effectiveModelTitle.length ? self.effectiveModelTitle : @"";
    self.modelIcon = modelIcon;
    self.modelChevron = modelChevron;
    self.model.accessibilityValue = self.modelValue.stringValue;
    [self separatorAt:412];
    self.errorItem = [self row:@"Show Capture Error…" symbol:@"exclamationmark.triangle" action:@selector(errorDetails:) frame:NSMakeRect(30, 428, 344, 28)];
    self.errorItem.hidden = YES;
    self.drops = [self label:@"" frame:NSZeroRect size:12 weight:NSFontWeightRegular];
    NSImageView *recordingsIcon;
    NSImageView *recordingsChevron;
    self.recordings = [self selectorRow:@"Recordings" symbol:@"waveform" action:@selector(showRecordings:) frame:NSMakeRect(30, 428, 344, 36) value:NULL icon:&recordingsIcon chevron:&recordingsChevron];
    self.recordingsIcon = recordingsIcon;
    self.recordingsChevron = recordingsChevron;
    [self separatorAt:500];
    self.quitButton = [self row:@"" symbol:@"" action:@selector(quit:) frame:NSMakeRect(30, 510, 344, 28)];
    self.quitButton.font = [NSFont systemFontOfSize:14 weight:NSFontWeightRegular];
    self.quitButton.accessibilityLabel = @"Quit RimV";
    NSImageView *quitIcon = [[NSImageView alloc] initWithFrame:NSZeroRect];
    quitIcon.image = [[NSImage imageWithSystemSymbolName:@"power" accessibilityDescription:@"Quit RimV"]
        imageWithSymbolConfiguration:[NSImageSymbolConfiguration configurationWithPointSize:16 weight:NSFontWeightRegular]];
    quitIcon.image.template = YES;
    quitIcon.translatesAutoresizingMaskIntoConstraints = NO;
    NSTextField *quitLabel = [NSTextField labelWithString:@"Quit RimV"];
    quitLabel.font = [NSFont systemFontOfSize:14 weight:NSFontWeightRegular];
    quitLabel.translatesAutoresizingMaskIntoConstraints = NO;
    NSTextField *quitShortcut = [NSTextField labelWithString:@"⌘Q"];
    quitShortcut.font = [NSFont systemFontOfSize:12 weight:NSFontWeightRegular];
    quitShortcut.translatesAutoresizingMaskIntoConstraints = NO;
    [self.quitButton addSubview:quitIcon];
    [self.quitButton addSubview:quitLabel];
    [self.quitButton addSubview:quitShortcut];
    [NSLayoutConstraint activateConstraints:@[
        [quitIcon.leadingAnchor constraintEqualToAnchor:self.quitButton.leadingAnchor constant:12],
        [quitIcon.centerYAnchor constraintEqualToAnchor:self.quitButton.centerYAnchor],
        [quitIcon.widthAnchor constraintEqualToConstant:16], [quitIcon.heightAnchor constraintEqualToConstant:16],
        [quitLabel.leadingAnchor constraintEqualToAnchor:quitIcon.trailingAnchor constant:10],
        [quitLabel.centerYAnchor constraintEqualToAnchor:self.quitButton.centerYAnchor],
        [quitShortcut.trailingAnchor constraintEqualToAnchor:self.quitButton.trailingAnchor constant:-12],
        [quitShortcut.centerYAnchor constraintEqualToAnchor:self.quitButton.centerYAnchor],
    ]];
    [self updateLanguageTitle];
    [self applySnapshot:self.snapshot];
}
- (void)applySnapshot:(NSDictionary *)snapshot {
    self.snapshot = snapshot;
    if (!self.statusItem) return;
    NSColor *primary = [self color:17 green:20 blue:24 darkRed:244 green:247 blue:250];
    NSColor *secondary = [self color:94 green:100 blue:117 darkRed:203 green:213 blue:225];
    NSColor *action = [self color:19 green:141 blue:132 darkRed:11 green:110 blue:105];
    NSColor *selectedSurface = [self color:229 green:245 blue:243 darkRed:23 green:59 blue:57];
    NSColor *liveText = [self color:6 green:95 blue:70 darkRed:134 green:239 blue:172];
    NSColor *errorText = [self color:153 green:27 blue:27 darkRed:252 green:165 blue:165];
    self.captureArea.layer.backgroundColor = [self color:246 green:247 blue:251 darkRed:23 green:32 blue:51].CGColor;
    self.errorCard.layer.borderColor = errorText.CGColor;
    for (NSView *view in self.mainContentView.subviews) {
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
    self.recordingsIcon.contentTintColor = primary;
    self.recordingsChevron.contentTintColor = secondary;
    NSString *status = snapshot[@"status"];
    BOOL recording = [status isEqualToString:@"recording"];
    BOOL starting = [status isEqualToString:@"starting"];
    BOOL stopping = [status isEqualToString:@"stopping"];
    BOOL transitioning = starting || stopping;
    BOOL showingError = [status isEqualToString:@"error"] && !self.errorDismissed;
    NSString *duration = elapsed([snapshot[@"elapsed_ms"] unsignedLongLongValue]);
    NSDictionary *transcription = snapshot[@"transcription"];
    NSString *statusText = @"Ready";
    if (recording) statusText = [NSString stringWithFormat:@"Listening · %@", duration];
    else if (stopping) statusText = @"Saving transcript…";
    else if (starting && [transcription[@"status"] isEqualToString:@"loading"]) statusText = @"Preparing transcription…";
    else if (starting) statusText = @"Starting…";
    else if (showingError) statusText = @"Capture needs attention";
    NSString *title = recording ? [NSString stringWithFormat:@" ● %@", duration] : @" RimV";
    if (![self.statusItem.button.title isEqualToString:title]) self.statusItem.button.title = title;
    self.statusItem.button.toolTip = [NSString stringWithFormat:@"RimV · %@", statusText];
    self.statusLine.stringValue = statusText;
    self.statusLine.textColor = secondary;
    self.brandDetail.stringValue = recording
        ? [NSString stringWithFormat:@"%@ · %@", [self selectedSourceSummary:snapshot[@"microphone"] system:snapshot[@"system_audio"]], duration]
        : (showingError ? @"Capture needs attention" : @"Real-time transcription");
    self.brandDetail.textColor = secondary;
    self.brandState.stringValue = recording ? @"●  LIVE" : (showingError ? @"●  Error" : (transitioning ? @"●  Loading" : @"●  Ready"));
    self.brandState.textColor = recording
        ? liveText
        : (showingError ? errorText : liveText);
    NSString *captureTitle = starting ? @"Preparing…"
        : stopping ? @"Stopping…"
        : recording ? @"Stop Listening"
        : showingError ? @"Retry Listening"
        : @"Start Listening";
    NSColor *captureForeground = transitioning ? secondary
        : recording ? errorText
        : [self color:255 green:255 blue:255 darkRed:249 green:250 blue:251];
    NSColor *captureBackground = transitioning
        ? [self color:246 green:247 blue:251 darkRed:23 green:32 blue:51]
        : recording
            ? [self color:254 green:226 blue:226 darkRed:127 green:29 blue:29]
            : action;
    self.capture.title = @"";
    self.captureLabel.stringValue = captureTitle;
    self.captureLabel.textColor = captureForeground;
    self.captureIcon.image = [[NSImage imageWithSystemSymbolName:(transitioning ? @"hourglass" : recording ? @"stop.fill" : (showingError ? @"arrow.clockwise" : @"play.fill"))
                                           accessibilityDescription:captureTitle]
        imageWithSymbolConfiguration:[NSImageSymbolConfiguration configurationWithPointSize:16 weight:NSFontWeightSemibold]];
    self.captureIcon.image.template = YES;
    self.captureIcon.image.size = NSMakeSize(16, 16);
    self.captureIcon.contentTintColor = captureForeground;
    NSSize groupSize = self.captureGroup.fittingSize;
    self.captureGroup.frame = NSMakeRect((NSWidth(self.capture.bounds) - groupSize.width) / 2,
                                         (NSHeight(self.capture.bounds) - groupSize.height) / 2,
                                         groupSize.width, groupSize.height);
    self.capture.contentTintColor = captureForeground;
    self.capture.layer.backgroundColor = captureBackground.CGColor;
    self.capture.font = [NSFont systemFontOfSize:14 weight:NSFontWeightBold];
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
    NSArray<NSNumber *> *available = @[
        @([capabilities[@"system_audio_capture"] boolValue]),
        @([capabilities[@"microphone_capture"] boolValue]),
        @([capabilities[@"system_audio_capture"] boolValue] && [capabilities[@"microphone_capture"] boolValue]),
    ];
    for (NSUInteger index = 0; index < cards.count; index++) {
        NSButton *card = cards[index];
        BOOL active = selected[index].boolValue;
        BOOL sourceAvailable = available[index].boolValue;
        card.layer.backgroundColor = (active ? selectedSurface
                                             : [self color:255 green:255 blue:255 darkRed:24 green:38 blue:59]).CGColor;
        card.layer.borderWidth = active ? 2 : 1.2;
        card.layer.borderColor = (active ? [self color:19 green:141 blue:132 darkRed:114 green:215 blue:208]
                                        : [self color:213 green:214 blue:218 darkRed:45 green:61 blue:85]).CGColor;
        NSColor *contentColor = !sourceAvailable ? [self color:94 green:100 blue:117 darkRed:148 green:163 blue:184]
            : active ? [self color:11 green:110 blue:105 darkRed:114 green:215 blue:208]
            : secondary;
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
    if ([error isKindOfClass:NSDictionary.class] && [status isEqualToString:@"error"] && !self.errorDismissed) {
        NSString *message = error[@"user_message"] ?: error[@"message"];
        [self showError:message];
    } else if (![status isEqualToString:@"error"]) {
        self.errorDismissed = NO;
        self.displayedError = nil;
        [self setErrorCardVisible:NO];
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
    self.interactionGeneration += 1;
    if (self.popover.shown) {
        [self logSelectorEvent:@"toggle-close-parent" selector:0 reason:@"status item clicked"];
        [self.popover performClose:nil];
    } else {
        [self logSelectorEvent:@"toggle-open-parent" selector:0 reason:@"status item clicked"];
        [self.popover showRelativeToRect:self.statusItem.button.bounds
                                  ofView:self.statusItem.button
                           preferredEdge:NSRectEdgeMinY];
    }
}
- (void)applicationDidResignActive:(NSNotification *)notification {
    (void)notification;
    // A child NSPopover may briefly become key while its parent remains the
    // active menu-bar UI. Global click monitoring owns real outside dismissal.
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
    self.command(50, 0);
}
- (void)selectMicrophone:(id)sender {
    (void)sender;
    self.command(50, 1);
}
- (void)selectBoth:(id)sender {
    (void)sender;
    self.command(50, 2);
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
- (NSString *)languageBaseCode:(NSString *)code {
    NSString *normalized = [code stringByReplacingOccurrencesOfString:@"_" withString:@"-"];
    return [[normalized componentsSeparatedByString:@"-"] firstObject].lowercaseString;
}
- (NSString *)languageTitle:(NSString *)code {
    if ([code isEqualToString:@"auto"]) return @"Auto Detect";
    NSDictionary *presentation = [self languagePresentation:code];
    NSString *sharedTitle = presentation[@"language_name"];
    return sharedTitle.length ? sharedTitle : code;
}
- (NSString *)languageOptionTitle:(NSString *)code {
    if ([code isEqualToString:@"auto"]) return [self languageTitle:code];
    NSDictionary *presentation = [self languagePresentation:code];
    NSString *sharedTitle = presentation[@"language_name"];
    NSString *sharedRegion = presentation[@"region_name"];
    if (sharedTitle.length) {
        if (sharedRegion.length) {
            return [NSString stringWithFormat:@"%@ (%@)", sharedTitle, sharedRegion];
        }
        return sharedTitle;
    }
    return [self languageTitle:code];
}
- (NSString *)languageFlag:(NSString *)code {
    if ([code isEqualToString:@"auto"]) return @"✨";
    NSString *flag = [self languagePresentation:code][@"flag"];
    return flag.length ? flag : @"🌐";
}
- (NSDictionary *)languagePresentation:(NSString *)code {
    NSDictionary *presentation = self.languagePresentations[code.lowercaseString];
    if (presentation) return presentation;
    for (NSString *key in self.languagePresentations) {
        if ([key caseInsensitiveCompare:code] == NSOrderedSame) return self.languagePresentations[key];
    }
    return @{};
}
- (NSArray<NSString *> *)sortedLanguageCodes:(NSArray<NSString *> *)codes {
    return [codes sortedArrayUsingComparator:^NSComparisonResult(NSString *left, NSString *right) {
        NSComparisonResult languageOrder = [[self languageTitle:left] compare:[self languageTitle:right] options:NSCaseInsensitiveSearch];
        if (languageOrder != NSOrderedSame) return languageOrder;
        NSString *leftRegion = [self languageOptionTitle:left];
        NSString *rightRegion = [self languageOptionTitle:right];
        NSComparisonResult regionOrder = [leftRegion compare:rightRegion options:NSCaseInsensitiveSearch];
        return regionOrder == NSOrderedSame ? [left compare:right options:NSCaseInsensitiveSearch] : regionOrder;
    }];
}
- (void)updateLanguageTitle {
    self.languageValue.stringValue = (self.supportedLanguages.count == 0 && !self.supportsLanguageDetection)
        ? @"Model required"
        : [self languageTitle:self.selectedLanguage];
    self.language.accessibilityValue = self.languageValue.stringValue;
    self.languageFlagIcon.stringValue = [self languageFlag:self.selectedLanguage];
    self.languageFlagIcon.accessibilityLabel = [self languageTitle:self.selectedLanguage];
    self.languageSummary.stringValue = [NSString stringWithFormat:@"%@ %@", [self languageFlag:self.selectedLanguage], self.languageValue.stringValue];
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
    self.languagePopover.behavior = NSPopoverBehaviorApplicationDefined;
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
    // The scroll view's document must share the selector's top-down coordinate
    // system or later sections are laid out above earlier ones when displayed.
    self.languageListDocument = [[RimvFlippedContentView alloc] initWithFrame:NSMakeRect(0, 0, 300, 1)];
    scroll.documentView = self.languageListDocument;
    [self.languagePopoverView addSubview:scroll];
    self.allLanguagesButton = [RimvHandCursorButton buttonWithTitle:@"All supported languages  ›" target:self action:@selector(toggleAllLanguages:)];
    self.allLanguagesButton.frame = NSMakeRect(18, 405, 300, 28);
    self.allLanguagesButton.bordered = NO;
    self.allLanguagesButton.alignment = NSTextAlignmentLeft;
    self.allLanguagesButton.font = [NSFont systemFontOfSize:12 weight:NSFontWeightMedium];
    self.allLanguagesButton.contentTintColor = [self languageColor:11 green:110 blue:105 darkR:114 green:215 blue:208];
    self.allLanguagesButton.accessibilityLabel = @"Show all supported languages";
    [self.languagePopoverView addSubview:self.allLanguagesButton];
    NSTextField *hint = [self languageLabel:@"Only languages available with the selected transcription provider are shown." frame:NSMakeRect(22, 435, 292, 22) size:11 weight:NSFontWeightRegular color:secondary];
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
    NSButton *button = [RimvHandCursorButton buttonWithTitle:@"" target:self action:@selector(selectLanguageButton:)];
    button.frame = NSMakeRect(0, *y, 292, 40);
    button.bordered = NO;
    button.wantsLayer = YES;
    button.layer.cornerRadius = 8;
    button.layer.backgroundColor = (selected
        ? [self languageColor:229 green:245 blue:243 darkR:23 green:59 blue:57]
        : NSColor.clearColor).CGColor;
    button.identifier = code;
    button.accessibilityLabel = [self languageOptionTitle:code];
    button.accessibilityValue = selected ? @"Selected" : @"Not selected";
    CGFloat nameX = 40;
    if ([code isEqualToString:@"auto"]) {
        NSImageView *icon = [[NSImageView alloc] initWithFrame:NSMakeRect(10, 11, 18, 18)];
        icon.image = [[NSImage imageWithSystemSymbolName:@"wand.and.stars" accessibilityDescription:nil]
            imageWithSymbolConfiguration:[NSImageSymbolConfiguration configurationWithPointSize:15 weight:NSFontWeightRegular]];
        icon.image.template = YES;
        icon.contentTintColor = selected ? [self languageColor:19 green:141 blue:132 darkR:114 green:215 blue:208] : [self languageColor:94 green:100 blue:117 darkR:203 green:213 blue:225];
        [button addSubview:icon];
    } else {
        NSTextField *flag = [self languageLabel:[self languageFlag:code] frame:NSMakeRect(10, 9, 22, 22) size:16 weight:NSFontWeightRegular color:NSColor.labelColor];
        flag.font = [NSFont fontWithName:@"Apple Color Emoji" size:16] ?: [NSFont systemFontOfSize:16];
        [button addSubview:flag];
    }
    NSTextField *name = [self languageLabel:[self languageOptionTitle:code] frame:NSMakeRect(nameX, 10, 214, 20) size:14 weight:(selected ? NSFontWeightMedium : NSFontWeightRegular) color:[self languageColor:17 green:20 blue:24 darkR:242 green:246 blue:250]];
    [button addSubview:name];
    if (selected) {
        NSImageView *check = [[NSImageView alloc] initWithFrame:NSMakeRect(264, 11, 18, 18)];
        check.image = [[NSImage imageWithSystemSymbolName:@"checkmark" accessibilityDescription:@"Selected"] imageWithSymbolConfiguration:[NSImageSymbolConfiguration configurationWithPointSize:15 weight:NSFontWeightBold]];
        check.image.template = YES;
        check.contentTintColor = [self languageColor:19 green:141 blue:132 darkR:114 green:215 blue:208];
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
    NSMutableSet<NSString *> *seen = [NSMutableSet set];
    if (self.supportsLanguageDetection) {
        [codes addObject:@"auto"];
        [seen addObject:@"auto"];
    }
    for (NSString *code in self.supportedLanguages) {
        NSString *key = code.lowercaseString;
        if (code.length && ![seen containsObject:key]) {
            [codes addObject:code];
            [seen addObject:key];
        }
    }
    NSArray<NSString *> *sortedCodes = [self sortedLanguageCodes:codes];
    NSPredicate *matches = [NSPredicate predicateWithBlock:^BOOL(NSString *code, NSDictionary *bindings) {
        (void)bindings;
        if (query.length == 0) return YES;
        NSDictionary *presentation = [self languagePresentation:code];
        NSArray *terms = [presentation[@"search_terms"] isKindOfClass:NSArray.class]
            ? presentation[@"search_terms"] : @[];
        if ([[self languageOptionTitle:code].lowercaseString containsString:query]
            || [code.lowercaseString containsString:query]) return YES;
        for (id term in terms) {
            if ([term isKindOfClass:NSString.class]
                && [((NSString *)term).lowercaseString containsString:query]) return YES;
        }
        return NO;
    }];
    NSArray<NSString *> *visible = [sortedCodes filteredArrayUsingPredicate:matches];
    BOOL searching = query.length > 0;
    NSMutableArray<NSString *> *popular = [NSMutableArray array];
    if (!searching && [visible containsObject:@"auto"]) [popular addObject:@"auto"];
    // One stable, useful representative per common language. Prefer the
    // user's exact locale, then the familiar regional default, then the
    // alphabetically sorted available locale for that language.
    NSArray<NSString *> *popularLanguages = @[@"en", @"es", @"fr", @"de", @"it", @"pt", @"ja", @"zh", @"ko"];
    if (!searching) {
        for (NSString *language in popularLanguages) {
            NSString *preferred = nil;
            if ([[self languageBaseCode:self.selectedLanguage] isEqualToString:language]
                && [visible containsObject:self.selectedLanguage]) {
                preferred = self.selectedLanguage;
            } else {
                for (NSString *code in visible) {
                    NSDictionary *presentation = [self languagePresentation:code];
                    if ([[self languageBaseCode:code] isEqualToString:language]
                        && [presentation[@"locale"] isEqual:presentation[@"canonical_locale"]]) {
                        preferred = code;
                        break;
                    }
                }
                if (!preferred) {
                    for (NSString *code in visible) {
                        if ([[self languageBaseCode:code] isEqualToString:language]) {
                            preferred = code;
                            break;
                        }
                    }
                }
            }
            if (preferred && ![popular containsObject:preferred]) [popular addObject:preferred];
        }
    }
    NSMutableArray<NSString *> *remaining = [visible mutableCopy];
    if (!searching) [remaining removeObjectsInArray:popular];
    if (!searching && !popular.count) remaining = [visible mutableCopy];
    BOOL hasAdditionalLanguages = popular.count > 0 && remaining.count > 0;
    BOOL showSimpleCompleteList = !searching && popular.count > 0 && !hasAdditionalLanguages;
    if (showSimpleCompleteList) remaining = [visible mutableCopy];
    if (!self.showAllLanguages && !searching && popular.count) [remaining removeAllObjects];
    if (showSimpleCompleteList) remaining = [visible mutableCopy];
    self.allLanguagesButton.hidden = searching || !hasAdditionalLanguages;
    self.allLanguagesButton.title = self.showAllLanguages ? @"Show fewer languages  ⌃" : @"Show all supported languages  ⌄";
    self.allLanguagesButton.accessibilityLabel = self.showAllLanguages ? @"Show fewer languages" : @"Show all supported languages";
    CGFloat y = 0;
    if (searching) {
        if (visible.count) {
            [self addLanguageSection:@"Search results" atY:&y];
            for (NSString *code in remaining) [self addLanguageOption:code atY:&y];
        }
    } else if (showSimpleCompleteList) {
        for (NSString *code in visible) [self addLanguageOption:code atY:&y];
    } else if (popular.count) {
        [self addLanguageSection:@"Popular languages" atY:&y];
        for (NSString *code in popular) [self addLanguageOption:code atY:&y];
        if (self.showAllLanguages && remaining.count) {
            y += 8;
            [self addLanguageSection:@"All supported languages" atY:&y];
            for (NSString *code in remaining) [self addLanguageOption:code atY:&y];
        }
    } else if (visible.count) {
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
    [self requestSelector:1];
}
- (NSString *)selectorName:(NSInteger)selector {
    switch (selector) {
        case 1: return @"Language";
        case 2: return @"Model";
        case 3: return @"Recordings";
        default: return @"none";
    }
}
- (BOOL)selectorIsShown:(NSInteger)selector {
    switch (selector) {
        case 1: return self.languagePopover.shown;
        case 2: return rimv_model_manager_selector_is_shown();
        case 3: return rimv_recordings_selector_is_shown();
        default: return NO;
    }
}
- (NSInteger)visibleSelector {
    if (self.languagePopover.shown) return 1;
    if (rimv_model_manager_selector_is_shown()) return 2;
    if (rimv_recordings_selector_is_shown()) return 3;
    return 0;
}
- (void)logSelectorEvent:(NSString *)event selector:(NSInteger)selector reason:(NSString *)reason {
    if (![NSProcessInfo.processInfo.environment[@"RIMV_SELECTOR_DIAGNOSTICS"] boolValue]) return;
    NSLog(@"[RimV selectors] time=%@ event=%@ selector=%@ active=%@ pending=%@ visible={language:%@ model:%@ recordings:%@ parent:%@} generation=%lu reason=%@",
          NSDate.date, event, [self selectorName:selector], [self selectorName:self.activeSelector],
          [self selectorName:self.pendingSelector], self.languagePopover.shown ? @"yes" : @"no",
          rimv_model_manager_selector_is_shown() ? @"yes" : @"no",
          rimv_recordings_selector_is_shown() ? @"yes" : @"no",
          self.popover.shown ? @"yes" : @"no", (unsigned long)self.interactionGeneration, reason);
}
- (BOOL)releasePopoverFocusInWindow:(NSWindow *)window selector:(NSInteger)selector {
    if (!window || window.firstResponder == window) return YES;
    NSResponder *responder = window.firstResponder;
    // AppKit documents nil as the request to make the window itself first
    // responder; use that public path to resign a search field's field editor.
    BOOL resigned = [window makeFirstResponder:nil];
    NSString *reason = [NSString stringWithFormat:@"firstResponder %@; makeFirstResponder(nil)=%@; resulting firstResponder %@",
                        NSStringFromClass(responder.class), resigned ? @"YES" : @"NO",
                        NSStringFromClass(window.firstResponder.class)];
    [self logSelectorEvent:@"focus-transfer" selector:selector reason:reason];
    return resigned && window.firstResponder == window;
}
- (BOOL)closeSelector:(NSInteger)selector reason:(NSString *)reason {
    if (self.closingSelector != 0) {
        [self logSelectorEvent:@"duplicate-close-suppressed" selector:selector reason:[NSString stringWithFormat:@"%@ is already closing", [self selectorName:self.closingSelector]]];
        return NO;
    }
    if (![self selectorIsShown:selector]) return YES;
    self.closingSelector = selector;
    BOOL closed = NO;
    if (selector == 1) {
        NSWindow *window = self.languagePopover.contentViewController.view.window;
        closed = [self releasePopoverFocusInWindow:window selector:selector];
        if (closed) [self.languagePopover performClose:nil];
    } else if (selector == 2) {
        closed = rimv_model_manager_close_selector();
    } else if (selector == 3) {
        closed = rimv_recordings_selector_close();
    }
    if (!closed) {
        self.closingSelector = 0;
        if (self.pendingSelector != 0) {
            [self logSelectorEvent:@"pending-selector-cleared" selector:self.pendingSelector reason:@"first responder refused transfer"];
            self.pendingSelector = 0;
        }
        [self logSelectorEvent:@"selector-close-blocked" selector:selector reason:@"could not safely transfer first responder"];
        return NO;
    }
    [self logSelectorEvent:@"close-selector" selector:selector reason:reason];
    return YES;
}
- (void)dismissSelectorsForReason:(NSString *)reason {
    self.interactionGeneration += 1;
    if (self.pendingSelector != 0) {
        [self logSelectorEvent:@"pending-selector-cleared" selector:self.pendingSelector reason:reason];
    }
    self.pendingSelector = 0;
    self.activeSelector = 0;
    [self logSelectorEvent:@"dismiss-all" selector:0 reason:reason];
    NSInteger visible = [self visibleSelector];
    if (visible != 0 && ![self closeSelector:visible reason:reason]) return;
    if (self.popover.shown) [self.popover performClose:nil];
}
- (void)requestSelector:(NSInteger)selector {
    if (selector < 1 || selector > 3) return;
    self.interactionGeneration += 1;
    if (self.closingSelector != 0) {
        if (self.pendingSelector == selector) {
            [self logSelectorEvent:@"duplicate-transition-request-suppressed" selector:selector reason:[NSString stringWithFormat:@"%@ is closing", [self selectorName:self.closingSelector]]];
            return;
        }
        if (self.pendingSelector != 0) {
            [self logSelectorEvent:@"pending-selector-replaced" selector:selector reason:[NSString stringWithFormat:@"replaced %@ while %@ closes", [self selectorName:self.pendingSelector], [self selectorName:self.closingSelector]]];
        }
        self.pendingSelector = selector;
        [self logSelectorEvent:@"pending-selector-set-during-close" selector:selector reason:[NSString stringWithFormat:@"waiting for %@ delegate close", [self selectorName:self.closingSelector]]];
        return;
    }
    NSInteger visible = [self visibleSelector];
    if (visible == selector) {
        [self logSelectorEvent:@"user-requested-close" selector:selector reason:@"same selector row clicked"];
        self.pendingSelector = 0;
        self.activeSelector = selector;
        [self closeSelector:selector reason:@"same selector toggled"];
        return;
    }
    self.pendingSelector = selector;
    [self logSelectorEvent:@"pending-selector-set" selector:selector reason:@"selector switch requested"];
    [self logSelectorEvent:@"request-selector" selector:selector reason:@"selector row clicked"];
    if (visible != 0) {
        self.activeSelector = visible;
        [self closeSelector:visible reason:[NSString stringWithFormat:@"switch to %@", [self selectorName:selector]]];
        return;
    }
    [self openPendingSelector];
}
- (void)openPendingSelector {
    if (!self.popover.shown) {
        if (self.pendingSelector != 0) [self logSelectorEvent:@"cancel-pending" selector:self.pendingSelector reason:@"parent popover is closed"];
        self.pendingSelector = 0;
        self.activeSelector = 0;
        return;
    }
    if (self.closingSelector != 0) {
        [self logSelectorEvent:@"wait-for-close-callback" selector:self.pendingSelector reason:[NSString stringWithFormat:@"%@ has not completed closing", [self selectorName:self.closingSelector]]];
        return;
    }
    NSInteger visible = [self visibleSelector];
    if (visible != 0) {
        self.activeSelector = visible;
        if (visible != self.pendingSelector && self.pendingSelector != 0) {
            [self logSelectorEvent:@"wait-for-visible-selector" selector:self.pendingSelector reason:[NSString stringWithFormat:@"%@ has not finished closing", [self selectorName:visible]]];
        }
        return;
    }
    NSInteger selector = self.pendingSelector;
    if (selector == 0) { self.activeSelector = 0; return; }
    [self logSelectorEvent:@"pending-selector-consumed" selector:selector reason:@"opening queued selector"];
    self.pendingSelector = 0;
    self.activeSelector = selector;
    [self logSelectorEvent:@"open-selector" selector:selector reason:@"no child selector is visible"];
    if (selector == 1) {
        [self buildLanguagePopover]; self.languageSearch.stringValue=@""; [self reloadLanguageOptions];
        [self.languagePopover showRelativeToRect:self.language.bounds ofView:self.language preferredEdge:NSRectEdgeMaxX];
        NSWindow *window = self.languagePopover.contentViewController.view.window;
        BOOL focused = [window makeFirstResponder:self.languageSearch];
        [self logSelectorEvent:@"focus-search" selector:selector reason:[NSString stringWithFormat:@"Language search focused=%@ firstResponder=%@", focused ? @"YES" : @"NO", NSStringFromClass(window.firstResponder.class)]];
    } else if (selector == 2) rimv_model_manager_show_selector(self.model);
    else if (selector == 3) rimv_recordings_selector_show(self.recordings);
    if (![self selectorIsShown:selector]) {
        self.activeSelector = 0;
        [self logSelectorEvent:@"open-failed" selector:selector reason:@"AppKit did not present the child popover"];
    }
}
- (void)selectorDidClose:(NSInteger)selector {
    BOOL isShown = [self selectorIsShown:selector];
    [self logSelectorEvent:(isShown ? @"ignore-close-still-visible" : @"selector-did-close") selector:selector reason:@"popover delegate callback"];
    // A queued callback from an earlier presentation must not clear a newly
    // reopened instance of that selector.
    if (isShown) { self.activeSelector = selector; return; }
    if (self.closingSelector != 0 && self.closingSelector != selector) {
        [self logSelectorEvent:@"stale-close-callback-ignored" selector:selector reason:[NSString stringWithFormat:@"%@ is the active close transition", [self selectorName:self.closingSelector]]];
        return;
    }
    if (self.closingSelector == selector) self.closingSelector = 0;
    if (self.activeSelector == selector) self.activeSelector = 0;
    [self openPendingSelector];
}
- (void)popoverDidClose:(NSNotification *)notification {
    if (notification.object == self.languagePopover) {
        [self logSelectorEvent:@"delegate-close" selector:1 reason:@"Language popoverDidClose"];
        rimv_menu_selector_did_close(1);
    } else if (notification.object == self.popover) {
        [self logSelectorEvent:@"parent-popover-dismissed" selector:0 reason:@"main popoverDidClose"];
        [self dismissSelectorsForReason:@"parent popover closed"];
    }
}
- (void)controlTextDidChange:(NSNotification *)notification {
    if (notification.object == self.languageSearch) [self reloadLanguageOptions];
}
- (void)selectLanguageButton:(NSButton *)sender {
    self.selectedLanguage = sender.identifier;
    [self updateLanguageTitle];
    if (self.languageCommand) self.languageCommand(self.selectedLanguage.UTF8String);
    [self closeSelector:1 reason:@"language selected"];
}
- (void)showError:(NSString *)message {
    if (![self.displayedError isEqualToString:message]) self.errorDismissed = NO;
    self.displayedError = message;
    self.errorTitle.stringValue = [message localizedCaseInsensitiveContainsString:@"model in Model Manager"]
        ? @"Models required" : @"No active source";
    self.errorMessage.stringValue = message ?: @"All selected sources failed to start. Check your audio permissions and input devices.";
    [self setErrorCardVisible:!self.errorDismissed];
}
- (void)setErrorCardVisible:(BOOL)visible {
    self.errorCard.hidden = !visible;
    CGFloat offset = visible ? 124 : 0;
    self.language.frame = NSMakeRect(30, 319 + offset, 344, 36);
    self.model.frame = NSMakeRect(30, 363 + offset, 344, 36);
    self.recordings.frame = NSMakeRect(30, 428 + offset, 344, 36);
    self.quitButton.frame = NSMakeRect(30, 510 + offset, 344, 28);
    for (NSView *view in self.mainContentView.subviews) {
        if ([view isKindOfClass:NSBox.class]) {
            if (fabs(NSMinY(view.frame) - (412 + (visible ? 0 : 124))) < 1 || fabs(NSMinY(view.frame) - 412) < 1)
                view.frame = NSMakeRect(16, 412 + offset, 348, 1);
            if (fabs(NSMinY(view.frame) - (500 + (visible ? 0 : 124))) < 1 || fabs(NSMinY(view.frame) - 500) < 1)
                view.frame = NSMakeRect(16, 500 + offset, 348, 1);
        }
    }
    self.popoverView.frame = NSMakeRect(
        0, 0,
        RimvMainContentWidth + (RimvMainContentInset * 2),
        RimvMainContentHeight + (RimvMainContentInset * 2) + offset);
    self.mainContentView.frame = NSMakeRect(
        RimvMainContentInset, RimvMainContentInset,
        RimvMainContentWidth, RimvMainContentHeight + offset);
    self.popover.contentSize = self.popoverView.bounds.size;
}
- (void)dismissError:(id)sender {
    (void)sender;
    self.errorDismissed = YES;
    [self setErrorCardVisible:NO];
    [self applySnapshot:self.snapshot];
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
- (void)openLiveView:(id)sender { (void)sender; rimv_transcription_window_open(); }
- (void)showRecordings:(id)sender { (void)sender; [self requestSelector:3]; }
- (void)showAbout:(id)sender {
    (void)sender;
    [NSApp orderFrontStandardAboutPanel:nil];
    [NSApp activateIgnoringOtherApps:YES];
}
- (void)showModels:(id)sender {
    (void)sender;
    [self requestSelector:2];
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
    [self logSelectorEvent:@"application-termination-requested" selector:0 reason:@"AppKit requested termination"];
    self.systemTermination = YES;
    [self quit:nil];
    return NSTerminateLater;
}
- (BOOL)applicationShouldTerminateAfterLastWindowClosed:(NSApplication *)sender {
    (void)sender;
    // RimV is a menu-bar app; closing a viewer is never an application quit.
    return NO;
}
- (void)applicationWillTerminate:(NSNotification *)notification {
    (void)notification;
    rimv_transcription_window_shutdown_playback();
    [self logSelectorEvent:@"application-will-terminate" selector:0 reason:@"AppKit termination callback"];
}
@end

void rimv_menu_create(const char *root, const char *snapshot, CommandCallback command) {
    @autoreleasepool {
        [NSApplication sharedApplication];
        [NSApp setActivationPolicy:NSApplicationActivationPolicyAccessory];
        if ([NSProcessInfo.processInfo.environment[@"RIMV_SELECTOR_DIAGNOSTICS"] boolValue]) {
            NSSetUncaughtExceptionHandler(RimvSelectorUncaughtException);
        }
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
    void (^update)(void) = ^{
        menu.selectedLanguage = selected ?: @"auto";
        [menu updateLanguageTitle];
    };
    if (NSThread.isMainThread) update();
    else dispatch_async(dispatch_get_main_queue(), update);
}

void rimv_menu_set_language_command_callback(LanguageCommandCallback callback) {
    dispatch_async(dispatch_get_main_queue(), ^{
        menu.languageCommand = callback;
    });
}

void rimv_menu_set_selector_state(const char *state) {
    NSData *data = [[NSString stringWithUTF8String:state] dataUsingEncoding:NSUTF8StringEncoding];
    NSDictionary *selectors = [NSJSONSerialization JSONObjectWithData:data options:0 error:NULL];
    if (![selectors isKindOfClass:NSDictionary.class]) return;
    void (^update)(void) = ^{
        NSString *modelTitle = [selectors[@"model"] isKindOfClass:NSString.class] ? selectors[@"model"] : @"No model";
        BOOL providerChanged = ![menu.effectiveModelTitle isEqualToString:modelTitle];
        menu.effectiveModelTitle = modelTitle;
        if (providerChanged) menu.showAllLanguages = NO;
        menu.modelValue.stringValue = modelTitle;
        menu.model.accessibilityValue = modelTitle;
        menu.supportedLanguages = selectors[@"languages"] ?: @[];
        NSMutableDictionary *presentations = [NSMutableDictionary dictionary];
        for (NSDictionary *presentation in selectors[@"language_presentations"]) {
            if (![presentation isKindOfClass:NSDictionary.class]) continue;
            NSString *languageId = presentation[@"id"];
            if (![languageId isKindOfClass:NSString.class] || languageId.length == 0) continue;
            presentations[languageId.lowercaseString] = presentation;
        }
        menu.languagePresentations = presentations;
        menu.supportsLanguageDetection = [selectors[@"auto_detect"] boolValue];
        if ([menu.selectedLanguage isEqualToString:@"auto"] && !menu.supportsLanguageDetection) {
            menu.selectedLanguage = menu.supportedLanguages.firstObject ?: @"auto";
        } else if (![menu.selectedLanguage isEqualToString:@"auto"] && ![menu.supportedLanguages containsObject:menu.selectedLanguage]) {
            NSString *base = [menu languageBaseCode:menu.selectedLanguage];
            NSString *matchingLanguage = nil;
            for (NSString *language in menu.supportedLanguages) {
                if ([[menu languageBaseCode:language] isEqualToString:base]) { matchingLanguage = language; break; }
            }
            menu.selectedLanguage = matchingLanguage
                ?: (menu.supportsLanguageDetection ? @"auto" : menu.supportedLanguages.firstObject ?: @"auto");
        }
        [menu updateLanguageTitle];
        [menu reloadLanguageOptions];
        [menu applySnapshot:menu.snapshot];
    };
    if (NSThread.isMainThread) update();
    else dispatch_async(dispatch_get_main_queue(), update);
}

void rimv_menu_exit(void) {
    dispatch_async(dispatch_get_main_queue(), ^{
        // Recorded-audio playback is independent of the active Rust capture.
        rimv_transcription_window_shutdown_playback();
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
static NSString *testedLanguage;
static void testCommand(uint32_t command, uint8_t enabled) {
    testedCommand = command;
    testedEnabled = enabled;
}
static void testLanguageCommand(const char *language) {
    testedLanguage = [NSString stringWithUTF8String:language ?: ""];
}
static NSDictionary *testLanguagePresentations(NSArray<NSString *> *codes, BOOL supportsAutoDetect) {
    NSDictionary *metadata = @{
        @"en": @[@"en-US", @"English", @"United States", @"🇺🇸"],
        @"en-us": @[@"en-US", @"English", @"United States", @"🇺🇸"],
        @"en-gb": @[@"en-GB", @"English", @"United Kingdom", @"🇬🇧"],
        @"en-au": @[@"en-AU", @"English", @"Australia", @"🇦🇺"],
        @"en-in": @[@"en-IN", @"English", @"India", @"🇮🇳"],
        @"es": @[@"es-ES", @"Spanish", @"Spain", @"🇪🇸"],
        @"es-es": @[@"es-ES", @"Spanish", @"Spain", @"🇪🇸"],
        @"es-mx": @[@"es-MX", @"Spanish", @"Mexico", @"🇲🇽"],
        @"fr": @[@"fr-FR", @"French", @"France", @"🇫🇷"],
        @"fr-fr": @[@"fr-FR", @"French", @"France", @"🇫🇷"],
        @"fr-ca": @[@"fr-CA", @"French", @"Canada", @"🇨🇦"],
        @"de": @[@"de-DE", @"German", @"Germany", @"🇩🇪"],
        @"de-de": @[@"de-DE", @"German", @"Germany", @"🇩🇪"],
        @"it": @[@"it-IT", @"Italian", @"Italy", @"🇮🇹"],
        @"it-it": @[@"it-IT", @"Italian", @"Italy", @"🇮🇹"],
        @"pt": @[@"pt-PT", @"Portuguese", @"Portugal", @"🇵🇹"],
        @"pt-br": @[@"pt-BR", @"Portuguese", @"Brazil", @"🇧🇷"],
        @"pt-pt": @[@"pt-PT", @"Portuguese", @"Portugal", @"🇵🇹"],
        @"ru": @[@"ru-RU", @"Russian", @"Russia", @"🇷🇺"],
        @"ca-es": @[@"ca-ES", @"Catalan", @"Spain", @"🇪🇸"],
        @"ar-sa": @[@"ar-SA", @"Arabic", @"Saudi Arabia", @"🇸🇦"],
        @"zh-tw": @[@"zh-TW", @"Chinese", @"Taiwan", @"🇹🇼"],
        @"hi-in": @[@"hi-IN", @"Hindi", @"India", @"🇮🇳"],
    };
    NSMutableDictionary *result = [NSMutableDictionary dictionary];
    NSDictionary *canonicalLocales = @{
        @"en": @"en-US", @"es": @"es-ES", @"fr": @"fr-FR", @"de": @"de-DE",
        @"it": @"it-IT", @"pt": @"pt-PT",
    };
    for (NSString *code in codes) {
        NSArray *values = metadata[code.lowercaseString];
        if (!values) continue;
        NSString *base = [[code componentsSeparatedByString:@"-"] firstObject].lowercaseString;
        result[code.lowercaseString] = @{
            @"id": code,
            @"locale": values[0],
            @"canonical_locale": canonicalLocales[base] ?: values[0],
            @"language_name": values[1],
            @"region_name": values[2],
            @"flag": values[3],
            @"search_terms": @[code, values[0], values[1], values[2]],
            @"supports_auto_detect": @(supportsAutoDetect),
        };
    }
    return result;
}

// Runs on the real AppKit main thread, without opening hardware or simulating
// mouse input. Checks native rendering and action-to-command wiring together.
bool rimv_menu_self_test(void) {
    @autoreleasepool {
        [menu applicationDidFinishLaunching:[NSNotification notificationWithName:NSApplicationDidFinishLaunchingNotification object:NSApp]];
        CommandCallback original = menu.command;
        LanguageCommandCallback originalLanguageCommand = menu.languageCommand;
        menu.command = testCommand;
        menu.languageCommand = testLanguageCommand;
        NSImage *menuBarAsset = [NSImage imageNamed:@"MenuBarIcon"];
        NSImage *rimvMarkAsset = [NSImage imageNamed:@"RimVMark"];
        fprintf(stdout, "asset=MenuBarIcon found=%s size=%.0fx%.0f reps=%lu\n",
                menuBarAsset ? "true" : "false", menuBarAsset.size.width, menuBarAsset.size.height,
                (unsigned long)menuBarAsset.representations.count);
        fprintf(stdout, "asset=RimVMark found=%s size=%.0fx%.0f reps=%lu\n",
                rimvMarkAsset ? "true" : "false", rimvMarkAsset.size.width, rimvMarkAsset.size.height,
                (unsigned long)rimvMarkAsset.representations.count);
        BOOL passed = menuBarAsset != nil && rimvMarkAsset != nil
            && NSEqualSizes(menuBarAsset.size, NSMakeSize(18, 18))
            && NSEqualSizes(rimvMarkAsset.size, NSMakeSize(64, 64));
        NSMutableDictionary *state = [menu.snapshot mutableCopy];
        state[@"microphone"] = @{@"enabled": @YES, @"active": @YES};
        state[@"system_audio"] = @{@"enabled": @NO, @"active": @NO};
        passed &= menu.statusItem != nil && menu.capture.enabled;
        passed &= menu.statusItem.button.image.template;
        passed &= NSEqualPoints(menu.mainContentView.frame.origin,
                                NSMakePoint(RimvMainContentInset, RimvMainContentInset));
        passed &= NSWidth(menu.popoverView.bounds) - NSWidth(menu.mainContentView.frame) == RimvMainContentInset * 2;
        passed &= NSHeight(menu.popoverView.bounds) - NSHeight(menu.mainContentView.frame) == RimvMainContentInset * 2;
        passed &= [menu.captureLabel.stringValue isEqualToString:@"Start Listening"];
        passed &= [menu.statusLine.stringValue isEqualToString:@"Ready"];
        passed &= menu.effectiveModelTitle.length > 0 && [menu.modelValue.stringValue isEqualToString:menu.effectiveModelTitle];
        passed &= menu.languageValue.stringValue.length > 0;
        passed &= menu.recordingsIcon.image != nil && menu.recordingsChevron.image != nil;
        passed &= NSEqualSizes(menu.recordings.frame.size, menu.language.frame.size);

        // Repeated row actions during a close must update one pending request,
        // not send another close to the same NSPopover.
        NSInteger previousPending = menu.pendingSelector;
        NSInteger previousClosing = menu.closingSelector;
        menu.closingSelector = 2;
        menu.pendingSelector = 1;
        [menu requestSelector:1];
        passed &= menu.closingSelector == 2 && menu.pendingSelector == 1;
        [menu requestSelector:3];
        passed &= menu.closingSelector == 2 && menu.pendingSelector == 3;
        menu.pendingSelector = previousPending;
        menu.closingSelector = previousClosing;

        menu.supportedLanguages = @[@"en", @"es", @"ru"];
        menu.languagePresentations = testLanguagePresentations(menu.supportedLanguages, YES);
        menu.supportsLanguageDetection = YES;
        menu.modelValue.stringValue = @"Whisper Tiny";
        [menu applySnapshot:state];
        passed &= menu.language.enabled && [menu.modelValue.stringValue isEqualToString:@"Whisper Tiny"];
        [menu buildLanguagePopover];
        menu.selectedLanguage = @"en";
        [menu updateLanguageTitle];
        passed &= [menu.languageFlagIcon.stringValue isEqualToString:@"🇺🇸"];
        menu.languageSearch.stringValue = @"span";
        [menu reloadLanguageOptions];
        passed &= menu.languageListDocument.subviews.count == 2;
        NSButton *spanish = [RimvHandCursorButton buttonWithTitle:@"" target:nil action:NULL];
        spanish.identifier = @"es";
        [menu selectLanguageButton:spanish];
        passed &= [testedLanguage isEqualToString:@"es"]
            && menu.languageValue.stringValue.length > 0
            && [menu.languageFlagIcon.stringValue isEqualToString:@"🇪🇸"];
        fprintf(stdout, "selector-check=base-language %s selected=%s title=%s flag=%s\n",
                passed ? "PASS" : "FAIL", menu.selectedLanguage.UTF8String,
                menu.languageValue.stringValue.UTF8String, menu.languageFlagIcon.stringValue.UTF8String);
        menu.supportedLanguages = @[@"en-US", @"en-GB", @"es-ES", @"fr-FR", @"de-DE", @"it-IT", @"pt-BR"];
        menu.languagePresentations = testLanguagePresentations(@[
            @"en", @"en-US", @"en-GB", @"en-AU", @"en-IN", @"es-ES", @"es-MX",
            @"fr-FR", @"de-DE", @"it-IT", @"pt-BR", @"pt-PT",
        ], YES);
        menu.selectedLanguage = @"es-ES";
        [menu updateLanguageTitle];
        passed &= [menu.languageFlagIcon.stringValue isEqualToString:@"🇪🇸"];
        passed &= [[menu languageOptionTitle:@"en-GB"] isEqualToString:@"English (United Kingdom)"];
        passed &= [[menu languageOptionTitle:@"es-MX"] isEqualToString:@"Spanish (Mexico)"];
        passed &= [[menu languageOptionTitle:@"fr-FR"] isEqualToString:@"French (France)"];
        passed &= [[menu languageOptionTitle:@"de-DE"] isEqualToString:@"German (Germany)"];
        passed &= [[menu languageOptionTitle:@"it-IT"] isEqualToString:@"Italian (Italy)"];
        passed &= [[menu languageOptionTitle:@"pt-BR"] isEqualToString:@"Portuguese (Brazil)"];
        NSArray<NSArray<NSString *> *> *flagCases = @[
            @[@"en-US", @"🇺🇸"], @[@"en-GB", @"🇬🇧"], @[@"en-AU", @"🇦🇺"],
            @[@"en-IN", @"🇮🇳"], @[@"es-ES", @"🇪🇸"], @[@"es-MX", @"🇲🇽"],
            @[@"fr-FR", @"🇫🇷"], @[@"de-DE", @"🇩🇪"], @[@"it-IT", @"🇮🇹"], @[@"pt-PT", @"🇵🇹"],
            @[@"es-419", @"🌐"], @[@"en", @"🇺🇸"],
        ];
        for (NSArray<NSString *> *flagCase in flagCases) {
            menu.selectedLanguage = flagCase[0];
            [menu updateLanguageTitle];
            passed &= [menu.languageFlagIcon.stringValue isEqualToString:flagCase[1]];
        }
        fprintf(stdout, "selector-check=regional-presentation %s\n", passed ? "PASS" : "FAIL");
        menu.selectedLanguage = @"es-ES";
        menu.languageSearch.stringValue = @"span";
        [menu reloadLanguageOptions];
        NSButton *selectedSpanish = nil;
        for (NSView *view in menu.languageListDocument.subviews) {
            if ([view isKindOfClass:NSButton.class] && [((NSButton *)view).identifier isEqualToString:@"es-ES"]) {
                selectedSpanish = (NSButton *)view;
            }
        }
        passed &= selectedSpanish != nil && [selectedSpanish.accessibilityValue isEqualToString:@"Selected"];
        menu.supportedLanguages = @[
            @"en-US", @"en-GB", @"en-AU", @"en-IN", @"es-ES", @"es-MX",
            @"fr-FR", @"fr-CA", @"de-DE", @"it-IT", @"pt-BR", @"pt-PT",
            @"ca-ES", @"ar-SA", @"zh-TW", @"hi-IN",
        ];
        menu.languagePresentations = testLanguagePresentations(menu.supportedLanguages, NO);
        menu.supportsLanguageDetection = NO;
        menu.selectedLanguage = @"en-GB";
        menu.showAllLanguages = NO;
        menu.languageSearch.stringValue = @"";
        [menu reloadLanguageOptions];
        NSMutableArray<NSString *> *collapsedCodes = [NSMutableArray array];
        NSInteger collapsedSelectedCount = 0;
        BOOL foundUnitedKingdomTitle = NO;
        for (NSView *view in menu.languageListDocument.subviews) {
            if (![view isKindOfClass:NSButton.class]) continue;
            NSButton *option = (NSButton *)view;
            if (!option.identifier.length) continue;
            [collapsedCodes addObject:option.identifier];
            collapsedSelectedCount += [option.accessibilityValue isEqualToString:@"Selected"] ? 1 : 0;
            if ([option.identifier isEqualToString:@"en-GB"]
                && [option.accessibilityLabel isEqualToString:@"English (United Kingdom)"]) {
                foundUnitedKingdomTitle = YES;
            }
        }
        passed &= [collapsedCodes containsObject:@"en-GB"] && foundUnitedKingdomTitle;
        passed &= collapsedSelectedCount == 1 && !menu.allLanguagesButton.hidden;
        passed &= [menu.allLanguagesButton.title isEqualToString:@"Show all supported languages  ⌄"];
        [menu toggleAllLanguages:nil];
        NSMutableArray<NSString *> *expandedCodes = [NSMutableArray array];
        NSInteger expandedSelectedCount = 0;
        NSTextField *popularHeader = nil;
        NSTextField *allHeader = nil;
        for (NSView *view in menu.languageListDocument.subviews) {
            if ([view isKindOfClass:NSTextField.class]) {
                NSTextField *label = (NSTextField *)view;
                if ([label.stringValue isEqualToString:@"POPULAR LANGUAGES"]) popularHeader = label;
                if ([label.stringValue isEqualToString:@"ALL SUPPORTED LANGUAGES"]) allHeader = label;
            }
            if (![view isKindOfClass:NSButton.class]) continue;
            NSButton *option = (NSButton *)view;
            if (!option.identifier.length) continue;
            [expandedCodes addObject:option.identifier];
            expandedSelectedCount += [option.accessibilityValue isEqualToString:@"Selected"] ? 1 : 0;
        }
        passed &= expandedCodes.count == menu.supportedLanguages.count;
        passed &= [NSSet setWithArray:expandedCodes].count == expandedCodes.count;
        passed &= expandedSelectedCount == 1;
        passed &= popularHeader != nil && allHeader != nil
            && popularHeader.frame.origin.y < allHeader.frame.origin.y;
        passed &= [menu.allLanguagesButton.title isEqualToString:@"Show fewer languages  ⌃"];
        menu.languageSearch.stringValue = @"united kingdom";
        [menu reloadLanguageOptions];
        passed &= menu.languageListDocument.subviews.count == 2
            && [((NSButton *)menu.languageListDocument.subviews.lastObject).identifier isEqualToString:@"en-GB"];
        menu.languageSearch.stringValue = @"en-gb";
        [menu reloadLanguageOptions];
        passed &= menu.languageListDocument.subviews.count == 2
            && [((NSButton *)menu.languageListDocument.subviews.lastObject).identifier isEqualToString:@"en-GB"];
        menu.languageSearch.stringValue = @"english";
        [menu reloadLanguageOptions];
        NSMutableSet<NSString *> *englishSearchCodes = [NSMutableSet set];
        for (NSView *view in menu.languageListDocument.subviews) {
            if ([view isKindOfClass:NSButton.class] && ((NSButton *)view).identifier.length) {
                [englishSearchCodes addObject:((NSButton *)view).identifier];
            }
        }
        passed &= [englishSearchCodes isEqualToSet:[NSSet setWithArray:@[@"en-US", @"en-GB", @"en-AU", @"en-IN"]]];
        menu.languageSearch.stringValue = @"spanish";
        [menu reloadLanguageOptions];
        NSMutableSet<NSString *> *spanishSearchCodes = [NSMutableSet set];
        for (NSView *view in menu.languageListDocument.subviews) {
            if ([view isKindOfClass:NSButton.class] && ((NSButton *)view).identifier.length) {
                [spanishSearchCodes addObject:((NSButton *)view).identifier];
            }
        }
        passed &= [spanishSearchCodes isEqualToSet:[NSSet setWithArray:@[@"es-ES", @"es-MX"]]];
        menu.languageSearch.stringValue = @"es-mx";
        [menu reloadLanguageOptions];
        passed &= menu.languageListDocument.subviews.count == 2
            && [((NSButton *)menu.languageListDocument.subviews.lastObject).identifier isEqualToString:@"es-MX"];
        menu.supportedLanguages = @[@"th-TH", @"vi-VN"];
        menu.languagePresentations = testLanguagePresentations(menu.supportedLanguages, NO);
        menu.languageSearch.stringValue = @"";
        menu.showAllLanguages = NO;
        [menu reloadLanguageOptions];
        passed &= menu.languageListDocument.subviews.count == 3
            && [((NSButton *)menu.languageListDocument.subviews[1]).identifier isEqualToString:@"th-TH"]
            && [((NSButton *)menu.languageListDocument.subviews[2]).identifier isEqualToString:@"vi-VN"]
            && menu.allLanguagesButton.hidden;
        NSDictionary *parakeetSelector = @{
            @"model": @"Parakeet", @"languages": @[@"en", @"es"], @"auto_detect": @YES,
            @"language_presentations": [testLanguagePresentations(@[@"en", @"es"], YES) allValues],
        };
        menu.selectedLanguage = @"en-GB";
        NSData *parakeetData = [NSJSONSerialization dataWithJSONObject:parakeetSelector options:0 error:nil];
        rimv_menu_set_selector_state([[NSString alloc] initWithData:parakeetData encoding:NSUTF8StringEncoding].UTF8String);
        passed &= [menu.supportedLanguages isEqualToArray:@[@"en", @"es"]]
            && [menu.selectedLanguage isEqualToString:@"en"]
            && [menu.languageFlagIcon.stringValue isEqualToString:@"🇺🇸"]
            && menu.supportsLanguageDetection
            && menu.languageListDocument.subviews.count == 3
            && menu.allLanguagesButton.hidden;
        menu.languageSearch.stringValue = @"united states";
        [menu reloadLanguageOptions];
        passed &= menu.languageListDocument.subviews.count == 2
            && [((NSButton *)menu.languageListDocument.subviews.lastObject).identifier isEqualToString:@"en"];
        menu.languageSearch.stringValue = @"";
        [menu reloadLanguageOptions];
        fprintf(stdout, "selector-provider=Parakeet selected=%s title=%s flag=%s auto=%d\n",
                menu.selectedLanguage.UTF8String, menu.languageValue.stringValue.UTF8String,
                menu.languageFlagIcon.stringValue.UTF8String, menu.supportsLanguageDetection);
        menu.selectedLanguage = @"auto";
        [menu updateLanguageTitle];
        passed &= [menu.languageFlagIcon.stringValue isEqualToString:@"✨"];
        NSDictionary *whisperSelector = @{
            @"model": @"Whisper", @"languages": @[@"fr", @"de", @"en"], @"auto_detect": @YES,
            @"language_presentations": [testLanguagePresentations(@[@"fr", @"de", @"en"], YES) allValues],
        };
        NSData *whisperData = [NSJSONSerialization dataWithJSONObject:whisperSelector options:0 error:nil];
        rimv_menu_set_selector_state([[NSString alloc] initWithData:whisperData encoding:NSUTF8StringEncoding].UTF8String);
        passed &= [menu.supportedLanguages isEqualToArray:@[@"fr", @"de", @"en"]]
            && [menu.selectedLanguage isEqualToString:@"auto"]
            && [menu.languageFlagIcon.stringValue isEqualToString:@"✨"]
            && menu.languageListDocument.subviews.count == 4
            && menu.allLanguagesButton.hidden
            && menu.supportsLanguageDetection;
        menu.selectedLanguage = @"fr";
        [menu updateLanguageTitle];
        passed &= [menu.languageFlagIcon.stringValue isEqualToString:@"🇫🇷"];
        fprintf(stdout, "selector-provider=Whisper selected=%s title=%s flag=%s auto=%d\n",
                menu.selectedLanguage.UTF8String, menu.languageValue.stringValue.UTF8String,
                menu.languageFlagIcon.stringValue.UTF8String, menu.supportsLanguageDetection);
        menu.selectedLanguage = @"auto";
        [menu updateLanguageTitle];
        NSDictionary *nativeSelector = @{
            @"model": @"Native", @"languages": @[@"en-US", @"en-GB", @"es-ES"], @"auto_detect": @NO,
            @"language_presentations": [testLanguagePresentations(@[@"en-US", @"en-GB", @"es-ES"], NO) allValues],
        };
        NSData *nativeData = [NSJSONSerialization dataWithJSONObject:nativeSelector options:0 error:nil];
        rimv_menu_set_selector_state([[NSString alloc] initWithData:nativeData encoding:NSUTF8StringEncoding].UTF8String);
        passed &= [menu.selectedLanguage isEqualToString:@"en-US"]
            && [menu.languageFlagIcon.stringValue isEqualToString:@"🇺🇸"]
            && !menu.supportsLanguageDetection;
        rimv_menu_set_language("en-GB");
        passed &= [menu.supportedLanguages isEqualToArray:@[@"en-US", @"en-GB", @"es-ES"]]
            && [menu.selectedLanguage isEqualToString:@"en-GB"];
        fprintf(stdout, "selector-provider=Native selected=%s title=%s flag=%s auto=%d\n",
                menu.selectedLanguage.UTF8String, menu.languageValue.stringValue.UTF8String,
                menu.languageFlagIcon.stringValue.UTF8String, menu.supportsLanguageDetection);
        menu.supportedLanguages = @[@"bg", @"hr", @"cs", @"da", @"nl", @"en", @"et", @"fi", @"fr", @"de", @"el", @"hu", @"it", @"lv", @"lt", @"mt", @"pl", @"pt", @"ro", @"sk", @"sl", @"es", @"sv", @"ru", @"uk"];
        menu.showAllLanguages = NO;
        menu.languageSearch.stringValue = @"";
        [menu reloadLanguageOptions];
        passed &= !menu.allLanguagesButton.hidden;
        NSButton *ukrainian = [RimvHandCursorButton buttonWithTitle:@"" target:nil action:NULL];
        ukrainian.identifier = @"uk";
        [menu selectLanguageButton:ukrainian];
        passed &= [testedLanguage isEqualToString:@"uk"] && menu.languageValue.stringValue.length > 0;
        passed &= menu.microphone.state == NSControlStateValueOn;
        [menu toggleCapture:nil];
        passed &= testedCommand == 1;
        state[@"status"] = @"starting";
        state[@"transcription"] = @{@"available": @YES, @"enabled": @YES, @"status": @"loading"};
        [menu applySnapshot:state];
        passed &= !menu.capture.enabled && !menu.microphone.enabled;
        passed &= [menu.statusLine.stringValue isEqualToString:@"Preparing transcription…"];
        passed &= [menu.captureLabel.stringValue isEqualToString:@"Preparing…"];
        state[@"status"] = @"recording";
        state[@"elapsed_ms"] = @754000;
        state[@"microphone"] = @{@"enabled": @YES, @"active": @YES};
        [menu applySnapshot:state];
        passed &= [menu.statusLine.stringValue isEqualToString:@"Listening · 00:12:34"];
        passed &= [menu.captureLabel.stringValue isEqualToString:@"Stop Listening"];
        passed &= ![menu.captureLabel.textColor isEqual:NSColor.whiteColor];
        passed &= [menu.captureIcon.contentTintColor isEqual:menu.captureLabel.textColor];
        passed &= CGColorGetAlpha(menu.capture.layer.backgroundColor) == 1.0;
        passed &= !menu.language.enabled;
        [menu toggleCapture:nil];
        passed &= testedCommand == 2;
        state[@"status"] = @"stopping";
        [menu applySnapshot:state];
        passed &= [menu.statusLine.stringValue isEqualToString:@"Saving transcript…"] && !menu.capture.enabled;
        passed &= [menu.captureLabel.stringValue isEqualToString:@"Stopping…"];
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
        state[@"last_error"] = @{@"message": @"Permission denied", @"user_message": @"RimV does not have permission to access this audio source."};
        state[@"status"] = @"error";
        [menu applySnapshot:state];
        passed &= !menu.errorCard.hidden && [menu.displayedError isEqualToString:@"RimV does not have permission to access this audio source."];
        passed &= menu.errorCard.isFlipped;
        passed &= menu.errorCard.subviews.count == 5;
        passed &= [menu.statusLine.stringValue isEqualToString:@"Capture needs attention"];
        passed &= [menu.captureLabel.stringValue isEqualToString:@"Retry Listening"];
        passed &= [menu.captureIcon.contentTintColor isEqual:menu.captureLabel.textColor];
        [menu dismissError:nil];
        passed &= menu.errorCard.hidden && [menu.captureLabel.stringValue isEqualToString:@"Start Listening"];
        state[@"status"] = @"idle";
        state[@"last_error"] = [NSNull null];
        [menu applySnapshot:state];
        passed &= [menu.captureLabel.stringValue isEqualToString:@"Start Listening"];
        passed &= [menu.statusLine.stringValue isEqualToString:@"Ready"];
        [menu quit:nil];
        passed &= testedCommand == 5 && !menu.capture.enabled;
        menu.command = original;
        menu.languageCommand = originalLanguageCommand;
        fprintf(stdout, "Native menu self-test: %s\n", passed ? "PASS" : "FAIL");
        return passed;
    }
}
