#import "transcription_window.h"
#import <AVFoundation/AVFoundation.h>
#import <UniformTypeIdentifiers/UniformTypeIdentifiers.h>

extern void rimv_recording_rename(const char *, const char *);
extern void rimv_recording_delete(const char *);

static BOOL TWIsDark(NSAppearance *appearance) {
    NSAppearance *resolved = appearance ?: NSApp.effectiveAppearance;
    return [[resolved bestMatchFromAppearancesWithNames:@[NSAppearanceNameAqua, NSAppearanceNameDarkAqua]]
        isEqualToString:NSAppearanceNameDarkAqua];
}

typedef NS_ENUM(NSInteger, RimvViewerMode) {
    RimvViewerModeNone,
    RimvViewerModeLive,
    RimvViewerModeCompleted,
};

@interface RimvViewerRootView : NSView
@property(nonatomic, copy) dispatch_block_t appearanceChanged;
@end

@implementation RimvViewerRootView
- (void)viewDidChangeEffectiveAppearance {
    [super viewDidChangeEffectiveAppearance];
    if (self.appearanceChanged) self.appearanceChanged();
}
@end

@interface RimvTranscriptionWindow : NSObject <NSWindowDelegate, AVAudioPlayerDelegate>
@property(nonatomic) RimvTranscriptionCommandCallback command;
@property(nonatomic, strong) NSWindow *window;
@property(nonatomic, strong) NSView *content;
@property(nonatomic, strong) NSView *audioCard;
@property(nonatomic, strong) NSView *badge;
@property(nonatomic, strong) NSTextField *brandSubtitle;
@property(nonatomic, strong) NSTextField *badgeLabel;
@property(nonatomic, strong) NSTextField *recordingTitle;
@property(nonatomic, strong) NSTextField *recordingTitleEditor;
@property(nonatomic, strong) NSButton *renameRecordingButton;
@property(nonatomic, strong) NSButton *deleteRecordingButton;
@property(nonatomic, strong) NSButton *saveRecordingNameButton;
@property(nonatomic, strong) NSButton *cancelRecordingNameButton;
@property(nonatomic, strong) NSTextField *audioTitle;
@property(nonatomic, strong) NSTextField *source;
@property(nonatomic, strong) NSTextField *timer;
@property(nonatomic, strong) NSTextField *status;
@property(nonatomic, strong) NSTextField *transcriptTitle;
@property(nonatomic, strong) NSTextField *transcriptStatus;
@property(nonatomic, strong) NSTextField *footerMessage;
@property(nonatomic, strong) NSScrollView *transcriptScroll;
@property(nonatomic, strong) NSTextView *transcript;
@property(nonatomic, strong) NSButton *playButton;
@property(nonatomic, strong) NSSlider *progress;
@property(nonatomic, strong) NSTextField *currentTime;
@property(nonatomic, strong) NSTextField *totalTime;
@property(nonatomic, strong) NSButton *exportTXTButton;
@property(nonatomic, strong) NSButton *exportJSONButton;
@property(nonatomic, strong) NSDictionary *snapshot;
@property(nonatomic, copy) NSString *sessionPath;
@property(nonatomic, copy) NSString *liveSessionID;
@property(nonatomic) RimvViewerMode mode;
@property(nonatomic) uint64_t transcriptRevision;
@property(nonatomic) BOOL allowWindowClose;
@property(nonatomic, strong) NSMutableDictionary<NSString *, NSDictionary *> *finalUpdates;
@property(nonatomic, strong) NSMutableDictionary<NSString *, NSDictionary *> *partialUpdates;
@property(nonatomic, strong) NSArray<NSDictionary *> *savedSegments;
@property(nonatomic, strong) AVAudioPlayer *player;
@property(nonatomic, strong) NSTimer *playbackTimer;
@end

@implementation RimvTranscriptionWindow

- (BOOL)isDark {
    return TWIsDark(self.window.effectiveAppearance ?: self.content.effectiveAppearance);
}

- (NSColor *)color:(CGFloat)lightR green:(CGFloat)lightG blue:(CGFloat)lightB
              dark:(CGFloat)darkR green:(CGFloat)darkG blue:(CGFloat)darkB {
    BOOL dark = [self isDark];
    return [NSColor colorWithRed:(dark ? darkR : lightR) / 255.0
                           green:(dark ? darkG : lightG) / 255.0
                            blue:(dark ? darkB : lightB) / 255.0 alpha:1];
}

- (NSTextField *)label:(NSString *)text frame:(NSRect)frame size:(CGFloat)size weight:(NSFontWeight)weight {
    NSTextField *label = [NSTextField labelWithString:text ?: @""];
    label.frame = frame;
    label.font = [NSFont systemFontOfSize:size weight:weight];
    label.textColor = [self color:26 green:39 blue:50 dark:241 green:246 blue:250];
    label.lineBreakMode = NSLineBreakByTruncatingTail;
    return label;
}

- (NSButton *)button:(NSString *)title action:(SEL)action frame:(NSRect)frame {
    NSButton *button = [NSButton buttonWithTitle:title target:self action:action];
    button.frame = frame;
    button.bezelStyle = NSBezelStyleRounded;
    button.accessibilityLabel = title;
    return button;
}

- (void)build {
    if (self.window) return;
    self.allowWindowClose = NO;
    self.window = [[NSWindow alloc] initWithContentRect:NSMakeRect(0, 0, 920, 720)
        styleMask:(NSWindowStyleMaskTitled | NSWindowStyleMaskClosable | NSWindowStyleMaskResizable)
        backing:NSBackingStoreBuffered defer:NO];
    self.window.title = @"RimV — Transcription";
    self.window.minSize = NSMakeSize(700, 540);
    self.window.delegate = self;
    RimvViewerRootView *root = [[RimvViewerRootView alloc] initWithFrame:self.window.contentView.bounds];
    __weak RimvTranscriptionWindow *weakSelf = self;
    root.appearanceChanged = ^{ [weakSelf refreshAppearance]; };
    self.window.contentView = root;
    self.content = root;
    self.content.wantsLayer = YES;

    NSView *logo = [[NSView alloc] initWithFrame:NSMakeRect(24, 638, 44, 44)];
    logo.wantsLayer = YES;
    logo.layer.cornerRadius = 11;
    logo.layer.backgroundColor = [NSColor colorWithRed:21.0/255 green:152.0/255 blue:142.0/255 alpha:1].CGColor;
    NSImageView *logoImage = [[NSImageView alloc] initWithFrame:NSMakeRect(11, 11, 22, 22)];
    logoImage.image = [NSImage imageWithSystemSymbolName:@"waveform" accessibilityDescription:@"RimV"];
    logoImage.image.template = YES;
    logoImage.contentTintColor = NSColor.whiteColor;
    [logo addSubview:logoImage];
    [self.content addSubview:logo];

    NSTextField *brand = [self label:@"RimV" frame:NSMakeRect(82, 654, 220, 25) size:20 weight:NSFontWeightBold];
    [self.content addSubview:brand];
    self.brandSubtitle = [self label:@"Transcription viewer" frame:NSMakeRect(82, 635, 480, 17) size:12 weight:NSFontWeightRegular];
    [self.content addSubview:self.brandSubtitle];
    self.badge = [[NSView alloc] initWithFrame:NSMakeRect(794, 649, 102, 30)];
    self.badge.wantsLayer = YES;
    self.badge.layer.cornerRadius = 15;
    self.badgeLabel = [self label:@"● LIVE" frame:NSMakeRect(0, 6, 102, 17) size:12 weight:NSFontWeightSemibold];
    self.badgeLabel.alignment = NSTextAlignmentCenter;
    [self.badge addSubview:self.badgeLabel];
    [self.content addSubview:self.badge];

    self.recordingTitle = [self label:@"Recording" frame:NSMakeRect(400, 651, 194, 24) size:15 weight:NSFontWeightSemibold];
    self.recordingTitle.alignment = NSTextAlignmentRight;
    self.recordingTitle.lineBreakMode = NSLineBreakByTruncatingMiddle;
    self.recordingTitle.accessibilityLabel = @"Recording title";
    self.recordingTitle.hidden = YES;
    [self.content addSubview:self.recordingTitle];
    self.renameRecordingButton = [self button:@"Rename" action:@selector(beginRecordingRename:) frame:NSMakeRect(602, 647, 74, 30)];
    self.renameRecordingButton.hidden = YES;
    [self.content addSubview:self.renameRecordingButton];
    self.deleteRecordingButton = [self button:@"Delete" action:@selector(deleteCurrentRecording:) frame:NSMakeRect(682, 647, 74, 30)];
    self.deleteRecordingButton.hidden = YES;
    [self.content addSubview:self.deleteRecordingButton];

    self.audioCard = [[NSView alloc] initWithFrame:NSMakeRect(24, 495, 872, 122)];
    self.audioCard.autoresizingMask = NSViewWidthSizable | NSViewMinYMargin;
    self.audioCard.wantsLayer = YES;
    self.audioCard.layer.cornerRadius = 14;
    [self.content addSubview:self.audioCard];
    NSView *audioIcon = [[NSView alloc] initWithFrame:NSMakeRect(20, 67, 40, 40)];
    audioIcon.wantsLayer = YES;
    audioIcon.layer.cornerRadius = 11;
    audioIcon.layer.backgroundColor = [NSColor colorWithRed:228.0/255 green:245.0/255 blue:243.0/255 alpha:1].CGColor;
    NSImageView *audioSymbol = [[NSImageView alloc] initWithFrame:NSMakeRect(10, 10, 20, 20)];
    audioSymbol.image = [NSImage imageWithSystemSymbolName:@"waveform" accessibilityDescription:@"Audio"];
    audioSymbol.image.template = YES;
    audioSymbol.contentTintColor = [NSColor colorWithRed:21.0/255 green:152.0/255 blue:142.0/255 alpha:1];
    [audioIcon addSubview:audioSymbol];
    [self.audioCard addSubview:audioIcon];
    self.audioTitle = [self label:@"Audio capture" frame:NSMakeRect(76, 88, 300, 21) size:16 weight:NSFontWeightSemibold];
    [self.audioCard addSubview:self.audioTitle];
    self.source = [self label:@"Waiting for capture state" frame:NSMakeRect(76, 65, 440, 18) size:13 weight:NSFontWeightRegular];
    [self.audioCard addSubview:self.source];
    self.timer = [self label:@"00:00:00" frame:NSMakeRect(690, 87, 160, 22) size:17 weight:NSFontWeightSemibold];
    self.timer.alignment = NSTextAlignmentRight;
    self.timer.font = [NSFont monospacedDigitSystemFontOfSize:17 weight:NSFontWeightSemibold];
    [self.audioCard addSubview:self.timer];
    self.status = [self label:@"Ready" frame:NSMakeRect(690, 63, 160, 18) size:12 weight:NSFontWeightMedium];
    self.status.alignment = NSTextAlignmentRight;
    [self.audioCard addSubview:self.status];

    self.playButton = [self button:@"Play" action:@selector(togglePlayback:) frame:NSMakeRect(20, 12, 46, 38)];
    self.playButton.hidden = YES;
    self.playButton.bezelStyle = NSBezelStyleRegularSquare;
    self.playButton.wantsLayer = YES;
    self.playButton.layer.cornerRadius = 19;
    self.playButton.image = [NSImage imageWithSystemSymbolName:@"play.fill" accessibilityDescription:@"Play recording"];
    self.playButton.image.template = YES;
    [self.audioCard addSubview:self.playButton];
    self.currentTime = [self label:@"00:00" frame:NSMakeRect(78, 22, 52, 17) size:12 weight:NSFontWeightRegular];
    self.currentTime.font = [NSFont monospacedDigitSystemFontOfSize:12 weight:NSFontWeightRegular];
    [self.audioCard addSubview:self.currentTime];
    self.progress = [[NSSlider alloc] initWithFrame:NSMakeRect(136, 19, 650, 22)];
    self.progress.minValue = 0;
    self.progress.maxValue = 1;
    self.progress.target = self;
    self.progress.action = @selector(seek:);
    self.progress.accessibilityLabel = @"Recording playback position";
    self.progress.hidden = YES;
    self.progress.autoresizingMask = NSViewWidthSizable;
    [self.audioCard addSubview:self.progress];
    self.totalTime = [self label:@"00:00" frame:NSMakeRect(800, 22, 50, 17) size:12 weight:NSFontWeightRegular];
    self.totalTime.alignment = NSTextAlignmentRight;
    self.totalTime.font = [NSFont monospacedDigitSystemFontOfSize:12 weight:NSFontWeightRegular];
    self.totalTime.autoresizingMask = NSViewMinXMargin;
    [self.audioCard addSubview:self.totalTime];

    self.transcriptTitle = [self label:@"Live transcription" frame:NSMakeRect(24, 457, 420, 22) size:17 weight:NSFontWeightSemibold];
    [self.content addSubview:self.transcriptTitle];
    self.transcriptStatus = [self label:@"● Transcribing…" frame:NSMakeRect(620, 458, 276, 18) size:12 weight:NSFontWeightMedium];
    self.transcriptStatus.alignment = NSTextAlignmentRight;
    [self.content addSubview:self.transcriptStatus];

    self.transcriptScroll = [[NSScrollView alloc] initWithFrame:NSMakeRect(24, 92, 872, 350)];
    self.transcriptScroll.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
    self.transcriptScroll.hasVerticalScroller = YES;
    self.transcriptScroll.autohidesScrollers = YES;
    self.transcriptScroll.borderType = NSLineBorder;
    self.transcript = [[NSTextView alloc] initWithFrame:self.transcriptScroll.bounds];
    self.transcript.editable = NO;
    self.transcript.selectable = YES;
    self.transcript.richText = YES;
    self.transcript.usesFindBar = YES;
    self.transcript.font = [NSFont systemFontOfSize:15];
    self.transcript.textContainerInset = NSMakeSize(20, 17);
    self.transcript.minSize = NSMakeSize(0, 0);
    self.transcript.maxSize = NSMakeSize(CGFLOAT_MAX, CGFLOAT_MAX);
    self.transcript.verticallyResizable = YES;
    self.transcript.horizontallyResizable = NO;
    self.transcript.autoresizingMask = NSViewWidthSizable;
    self.transcript.textContainer.widthTracksTextView = YES;
    self.transcriptScroll.documentView = self.transcript;
    [self.content addSubview:self.transcriptScroll];

    self.footerMessage = [self label:@"Viewer only · Recording continues if you close this window." frame:NSMakeRect(24, 45, 570, 20) size:12 weight:NSFontWeightRegular];
    [self.content addSubview:self.footerMessage];
    self.exportTXTButton = [self button:@"↓ Export TXT" action:@selector(exportTXT:) frame:NSMakeRect(650, 38, 116, 34)];
    self.exportJSONButton = [self button:@"↓ Export JSON" action:@selector(exportJSON:) frame:NSMakeRect(776, 38, 120, 34)];
    self.exportTXTButton.hidden = YES;
    self.exportJSONButton.hidden = YES;
    self.exportTXTButton.autoresizingMask = NSViewMinXMargin;
    self.exportJSONButton.autoresizingMask = NSViewMinXMargin;
    [self.content addSubview:self.exportTXTButton];
    [self.content addSubview:self.exportJSONButton];

    [self refreshAppearance];
}

- (void)refreshAppearance {
    if (!self.content) return;
    self.content.layer.backgroundColor = [self color:237 green:242 blue:245 dark:11 green:23 blue:35].CGColor;
    self.audioCard.layer.backgroundColor = [self color:245 green:247 blue:249 dark:26 green:48 blue:68].CGColor;
    self.transcript.backgroundColor = [self color:255 green:255 blue:255 dark:18 green:37 blue:55];
    self.transcript.textColor = [self color:26 green:39 blue:50 dark:240 green:247 blue:250];
    self.transcriptScroll.backgroundColor = self.transcript.backgroundColor;
    self.brandSubtitle.textColor = [self color:101 green:117 blue:133 dark:170 green:185 blue:200];
    self.source.textColor = [self color:101 green:117 blue:133 dark:170 green:185 blue:200];
    self.status.textColor = (self.mode == RimvViewerModeLive)
        ? [self color:187 green:66 blue:82 dark:255 green:137 blue:145]
        : [self color:22 green:130 blue:94 dark:68 green:211 blue:160];
    self.timer.textColor = [self color:26 green:39 blue:50 dark:241 green:246 blue:250];
    self.transcriptTitle.textColor = [self color:26 green:39 blue:50 dark:241 green:246 blue:250];
    self.transcriptStatus.textColor = [self color:21 green:126 blue:121 dark:74 green:199 blue:187];
    self.footerMessage.textColor = [self color:101 green:117 blue:133 dark:170 green:185 blue:200];
    self.currentTime.textColor = self.footerMessage.textColor;
    self.totalTime.textColor = self.footerMessage.textColor;
    self.badge.layer.backgroundColor = (self.mode == RimvViewerModeLive
        ? [self color:255 green:240 blue:241 dark:75 green:35 blue:46]
        : [self color:224 green:246 blue:238 dark:24 green:68 blue:57]).CGColor;
    self.badgeLabel.textColor = (self.mode == RimvViewerModeLive)
        ? [self color:197 green:60 blue:74 dark:255 green:145 blue:152]
        : [self color:22 green:130 blue:94 dark:104 green:222 blue:170];
    NSColor *buttonBackground = [self color:21 green:126 blue:121 dark:21 green:126 blue:121];
    self.playButton.contentTintColor = NSColor.whiteColor;
    self.playButton.layer.backgroundColor = buttonBackground.CGColor;
    self.playButton.image.template = YES;
    self.playButton.title = @"";
}

- (NSString *)defaultRecordingTitleForStartTime:(unsigned long long)milliseconds {
    NSDateFormatter *formatter = [NSDateFormatter new];
    formatter.locale = NSLocale.currentLocale;
    formatter.dateFormat = @"HH:mm";
    return [NSString stringWithFormat:@"Recording — %@", [formatter stringFromDate:[NSDate dateWithTimeIntervalSince1970:milliseconds / 1000.0]]];
}

- (NSString *)recordingTitleAtPath:(NSString *)path sessionID:(NSString *)sessionID startedAt:(unsigned long long)startedAt {
    if (!path.length) return [self defaultRecordingTitleForStartTime:startedAt];
    NSString *metadataPath = [path stringByAppendingPathComponent:@"rimv-session.json"];
    NSData *data = [NSData dataWithContentsOfFile:metadataPath];
    NSDictionary *metadata = data ? [NSJSONSerialization JSONObjectWithData:data options:0 error:nil] : nil;
    if ([metadata[@"session_id"] isEqual:sessionID] && [metadata[@"title"] isKindOfClass:NSString.class] && [metadata[@"title"] length]) return metadata[@"title"];
    return [self defaultRecordingTitleForStartTime:startedAt];
}

- (void)displayRecordingTitle:(NSString *)title {
    self.recordingTitle.stringValue = title.length ? title : @"Recording";
    self.window.title = [NSString stringWithFormat:@"RimV — %@", self.recordingTitle.stringValue];
    self.recordingTitle.hidden = NO;
    self.renameRecordingButton.hidden = NO;
    self.deleteRecordingButton.hidden = self.mode == RimvViewerModeLive;
}

- (void)beginRecordingRename:(id)sender {
    (void)sender;
    if (!self.sessionPath.length || self.recordingTitleEditor) return;
    self.recordingTitleEditor = [[NSTextField alloc] initWithFrame:self.recordingTitle.frame];
    self.recordingTitleEditor.stringValue = self.recordingTitle.stringValue;
    self.recordingTitleEditor.accessibilityLabel = @"Recording name";
    [self.content addSubview:self.recordingTitleEditor];
    self.recordingTitle.hidden = YES;
    self.renameRecordingButton.hidden = YES;
    self.deleteRecordingButton.hidden = YES;
    self.saveRecordingNameButton = [self button:@"Save" action:@selector(saveRecordingRename:) frame:NSMakeRect(602, 647, 74, 30)];
    self.cancelRecordingNameButton = [self button:@"Cancel" action:@selector(cancelRecordingRename:) frame:NSMakeRect(682, 647, 74, 30)];
    [self.content addSubview:self.saveRecordingNameButton];
    [self.content addSubview:self.cancelRecordingNameButton];
    [self.window makeFirstResponder:self.recordingTitleEditor];
}

- (void)saveRecordingRename:(id)sender {
    (void)sender;
    NSString *title = [self.recordingTitleEditor.stringValue stringByTrimmingCharactersInSet:NSCharacterSet.whitespaceAndNewlineCharacterSet];
    if (!title.length || title.length > 120 || !self.sessionPath.length) return;
    const char *path = self.sessionPath.UTF8String;
    const char *name = title.UTF8String;
    if (path && name) rimv_recording_rename(path, name);
}

- (void)cancelRecordingRename:(id)sender {
    (void)sender;
    [self.recordingTitleEditor removeFromSuperview];
    self.recordingTitleEditor = nil;
    [self.saveRecordingNameButton removeFromSuperview];
    [self.cancelRecordingNameButton removeFromSuperview];
    self.saveRecordingNameButton = nil;
    self.cancelRecordingNameButton = nil;
    self.recordingTitle.hidden = NO;
    self.renameRecordingButton.hidden = NO;
    self.deleteRecordingButton.hidden = self.mode == RimvViewerModeLive;
}

- (void)deleteCurrentRecording:(id)sender {
    (void)sender;
    if (self.mode != RimvViewerModeCompleted || !self.sessionPath.length) return;
    NSAlert *alert = [NSAlert new];
    alert.messageText = @"Delete this recording?";
    alert.informativeText = @"Its audio and transcript files will be permanently deleted.";
    [alert addButtonWithTitle:@"Delete Recording"];
    [alert addButtonWithTitle:@"Cancel"];
    if ([alert runModal] == NSAlertFirstButtonReturn) rimv_recording_delete(self.sessionPath.UTF8String);
}

- (void)applyRecordingTitle:(NSString *)title forPath:(NSString *)path {
    if (![self.sessionPath isEqualToString:path]) return;
    [self cancelRecordingRename:nil];
    [self displayRecordingTitle:title];
}

- (void)setMode:(RimvViewerMode)mode {
    _mode = mode;
    if (!self.content) return;
    BOOL live = mode == RimvViewerModeLive;
    self.brandSubtitle.stringValue = live ? @"Live transcription · Audio capture" : @"Completed recording · Saved audio";
    self.badgeLabel.stringValue = live ? @"● LIVE" : @"✓ COMPLETED";
    self.transcriptTitle.stringValue = live ? @"Live transcription" : @"Transcription";
    self.transcriptStatus.stringValue = live ? @"● Transcribing…" : @"✓ Processed";
    self.audioTitle.stringValue = live ? @"Audio capture" : @"Recorded audio";
    self.footerMessage.stringValue = live
        ? @"Viewer only · Recording continues if you close this window."
        : @"Final transcript · Timestamped source data remains in JSON.";
    self.exportTXTButton.hidden = live;
    self.exportJSONButton.hidden = live;
    self.deleteRecordingButton.hidden = live || !self.sessionPath.length;
    self.progress.hidden = live || self.player == nil;
    self.playButton.hidden = live || self.player == nil;
    [self refreshAppearance];
}

- (NSString *)sourceDescription:(NSDictionary *)snapshot {
    NSDictionary *mic = [snapshot[@"microphone"] isKindOfClass:NSDictionary.class] ? snapshot[@"microphone"] : @{};
    NSDictionary *system = [snapshot[@"system_audio"] isKindOfClass:NSDictionary.class] ? snapshot[@"system_audio"] : @{};
    BOOL micEnabled = [mic[@"enabled"] boolValue];
    BOOL systemEnabled = [system[@"enabled"] boolValue];
    if (micEnabled && systemEnabled) return @"Microphone + System";
    if (micEnabled) return @"Microphone";
    if (systemEnabled) return @"System";
    return @"No capture source selected";
}

- (NSString *)clock:(NSTimeInterval)seconds {
    NSUInteger value = (NSUInteger)MAX(0, floor(seconds));
    return [NSString stringWithFormat:@"%02lu:%02lu", (unsigned long)(value / 60), (unsigned long)(value % 60)];
}

- (void)applySnapshot:(NSDictionary *)snapshot {
    if (!snapshot) return;
    self.snapshot = snapshot;
    if (self.mode != RimvViewerModeLive || !self.window) return;
    NSDictionary *session = [snapshot[@"session"] isKindOfClass:NSDictionary.class] ? snapshot[@"session"] : nil;
    NSString *sessionPath = [session[@"recording_directory"] isKindOfClass:NSString.class] ? session[@"recording_directory"] : nil;
    if (sessionPath.length) self.sessionPath = sessionPath;
    self.source.stringValue = [NSString stringWithFormat:@"Capturing from %@", [self sourceDescription:snapshot]];
    uint64_t elapsed = [snapshot[@"elapsed_ms"] unsignedLongLongValue];
    self.timer.stringValue = [NSString stringWithFormat:@"%02llu:%02llu:%02llu",
        (unsigned long long)(elapsed / 3600000),
        (unsigned long long)((elapsed / 60000) % 60),
        (unsigned long long)((elapsed / 1000) % 60)];
    NSString *status = snapshot[@"status"] ?: @"idle";
    if ([status isEqualToString:@"starting"]) {
        self.status.stringValue = @"Preparing transcription";
        self.transcriptStatus.stringValue = @"● Starting…";
    } else if ([status isEqualToString:@"recording"]) {
        self.status.stringValue = @"● Recording";
        self.transcriptStatus.stringValue = @"● Transcribing…";
    } else if ([status isEqualToString:@"stopping"]) {
        self.status.stringValue = @"Finalizing session…";
        self.transcriptStatus.stringValue = @"● Processing…";
    } else if ([status isEqualToString:@"error"]) {
        self.status.stringValue = @"Capture needs attention";
    } else if ([status isEqualToString:@"idle"] && self.sessionPath.length) {
        [self showCompletedSessionAtPath:self.sessionPath];
        return;
    }
    [self renderLiveTranscriptFollowing:NO];
    [self refreshAppearance];
}

- (NSArray<NSDictionary *> *)sortedUpdates:(NSDictionary<NSString *, NSDictionary *> *)updates {
    return [updates.allValues sortedArrayUsingComparator:^NSComparisonResult(NSDictionary *a, NSDictionary *b) {
        unsigned long long startA = [a[@"start_ms"] unsignedLongLongValue];
        unsigned long long startB = [b[@"start_ms"] unsignedLongLongValue];
        if (startA < startB) return NSOrderedAscending;
        if (startA > startB) return NSOrderedDescending;
        return [a[@"utterance_id"] compare:b[@"utterance_id"]];
    }];
}

- (NSString *)textForUpdate:(NSDictionary *)update {
    NSMutableArray<NSString *> *parts = [NSMutableArray array];
    NSString *stable = [update[@"stable_text"] isKindOfClass:NSString.class] ? update[@"stable_text"] : @"";
    NSString *unstable = [update[@"unstable_text"] isKindOfClass:NSString.class] ? update[@"unstable_text"] : @"";
    if (stable.length) [parts addObject:stable];
    if (unstable.length) [parts addObject:unstable];
    return [parts componentsJoinedByString:@" "];
}

- (void)renderLiveTranscriptFollowing:(BOOL)forceFollow {
    if (!self.transcript || self.mode != RimvViewerModeLive) return;
    BOOL following = forceFollow || !self.transcript.string.length || self.transcriptScroll.verticalScroller.floatValue > .95;
    NSMutableAttributedString *rendered = [[NSMutableAttributedString alloc] init];
    NSDictionary *finalStyle = @{
        NSFontAttributeName: [NSFont systemFontOfSize:15],
        NSForegroundColorAttributeName: [self color:26 green:39 blue:50 dark:240 green:247 blue:250],
    };
    for (NSDictionary *update in [self sortedUpdates:self.finalUpdates ?: @{}]) {
        NSString *text = [self textForUpdate:update];
        if (!text.length) continue;
        if (rendered.length) [rendered appendAttributedString:[[NSAttributedString alloc] initWithString:@"\n\n"]];
        [rendered appendAttributedString:[[NSAttributedString alloc] initWithString:text attributes:finalStyle]];
    }
    NSArray<NSDictionary *> *partials = [self sortedUpdates:self.partialUpdates ?: @{}];
    for (NSDictionary *update in partials) {
        NSString *text = [self textForUpdate:update];
        if (!text.length) continue;
        if (rendered.length) [rendered appendAttributedString:[[NSAttributedString alloc] initWithString:@"\n\n"]];
        NSDictionary *partialStyle = @{
            NSFontAttributeName: [[NSFontManager sharedFontManager] convertFont:[NSFont systemFontOfSize:15] toHaveTrait:NSItalicFontMask],
            NSForegroundColorAttributeName: [self color:101 green:117 blue:133 dark:170 green:185 blue:200],
        };
        [rendered appendAttributedString:[[NSAttributedString alloc] initWithString:text attributes:partialStyle]];
    }
    if (!rendered.length) {
        [rendered appendAttributedString:[[NSAttributedString alloc] initWithString:@"Waiting for speech…" attributes:@{
            NSFontAttributeName: [NSFont systemFontOfSize:15],
            NSForegroundColorAttributeName: [self color:101 green:117 blue:133 dark:170 green:185 blue:200],
        }]];
    }
    [self.transcript.textStorage setAttributedString:rendered];
    if (following) [self.transcript scrollRangeToVisible:NSMakeRange(self.transcript.string.length, 0)];
}

- (void)applyTranscriptState:(NSDictionary *)state {
    if (![state isKindOfClass:NSDictionary.class]) return;
    uint64_t revision = [state[@"revision"] unsignedLongLongValue];
    if (revision < self.transcriptRevision) return;
    self.transcriptRevision = revision;
    self.finalUpdates = [NSMutableDictionary dictionary];
    self.partialUpdates = [NSMutableDictionary dictionary];
    for (NSDictionary *update in [state[@"finalized"] isKindOfClass:NSArray.class] ? state[@"finalized"] : @[]) {
        NSString *identifier = update[@"utterance_id"];
        if (identifier.length) self.finalUpdates[identifier] = update;
    }
    for (NSDictionary *update in [state[@"partials"] isKindOfClass:NSArray.class] ? state[@"partials"] : @[]) {
        NSString *identifier = update[@"utterance_id"];
        if (identifier.length) self.partialUpdates[identifier] = update;
    }
    [self renderLiveTranscriptFollowing:YES];
}

- (void)applyTranscriptUpdateEnvelope:(NSDictionary *)envelope {
    NSDictionary *update = [envelope[@"update"] isKindOfClass:NSDictionary.class] ? envelope[@"update"] : envelope;
    uint64_t revision = [envelope[@"revision"] unsignedLongLongValue];
    if (revision && revision <= self.transcriptRevision) return;
    if (revision) self.transcriptRevision = revision;
    if (!self.finalUpdates) self.finalUpdates = [NSMutableDictionary dictionary];
    if (!self.partialUpdates) self.partialUpdates = [NSMutableDictionary dictionary];
    NSString *identifier = [update[@"utterance_id"] isKindOfClass:NSString.class] ? update[@"utterance_id"] : nil;
    if (!identifier.length) return;
    if ([update[@"is_final"] boolValue]) {
        self.finalUpdates[identifier] = update;
        [self.partialUpdates removeObjectForKey:identifier];
    } else if (![self textForUpdate:update].length) {
        [self.partialUpdates removeObjectForKey:identifier];
    } else {
        self.partialUpdates[identifier] = update;
    }
    [self renderLiveTranscriptFollowing:NO];
}

- (NSArray<NSDictionary *> *)readSavedSegmentsAtPath:(NSString *)path {
    NSString *jsonPath = [path stringByAppendingPathComponent:@"transcript.json"];
    NSData *data = [NSData dataWithContentsOfFile:jsonPath];
    id value = data ? [NSJSONSerialization JSONObjectWithData:data options:0 error:nil] : nil;
    if (![value isKindOfClass:NSArray.class]) return @[];
    NSMutableArray *segments = [NSMutableArray array];
    for (id item in value) if ([item isKindOfClass:NSDictionary.class]) [segments addObject:item];
    return segments;
}

- (NSString *)plainTextForSegments:(NSArray<NSDictionary *> *)segments {
    NSMutableArray<NSString *> *paragraphs = [NSMutableArray array];
    for (NSDictionary *segment in segments) {
        NSString *text = [segment[@"text"] isKindOfClass:NSString.class] ? [segment[@"text"] stringByTrimmingCharactersInSet:NSCharacterSet.whitespaceAndNewlineCharacterSet] : @"";
        if (text.length) [paragraphs addObject:text];
    }
    return [paragraphs componentsJoinedByString:@"\n\n"];
}

- (void)loadAudioForSessionAtPath:(NSString *)path {
    [self stopPlaybackAndReleasePlayer];
    NSArray<NSString *> *files = @[@"microphone.wav", @"system.wav"];
    for (NSString *name in files) {
        NSString *audioPath = [path stringByAppendingPathComponent:name];
        if (![[NSFileManager defaultManager] fileExistsAtPath:audioPath]) continue;
        NSError *error = nil;
        AVAudioPlayer *player = [[AVAudioPlayer alloc] initWithContentsOfURL:[NSURL fileURLWithPath:audioPath] error:&error];
        if (!player || error) continue;
        self.player = player;
        self.player.delegate = self;
        [self.player prepareToPlay];
        self.playButton.hidden = NO;
        self.progress.hidden = NO;
        self.progress.maxValue = MAX(1, self.player.duration);
        self.progress.doubleValue = 0;
        self.currentTime.stringValue = @"00:00";
        self.totalTime.stringValue = [self clock:self.player.duration];
        self.status.stringValue = [NSString stringWithFormat:@"Saved · %@", [name isEqualToString:@"microphone.wav"] ? @"Microphone" : @"System"];
        return;
    }
    self.playButton.hidden = YES;
    self.progress.hidden = YES;
    self.currentTime.stringValue = @"00:00";
    self.totalTime.stringValue = @"00:00";
    self.status.stringValue = @"Audio unavailable";
}

- (void)showCompletedSessionAtPath:(NSString *)path {
    if (!path.length) return;
    self.sessionPath = path;
    self.mode = RimvViewerModeCompleted;
    NSData *metadataData = [NSData dataWithContentsOfFile:[path stringByAppendingPathComponent:@"session.json"]];
    NSDictionary *metadata = metadataData ? [NSJSONSerialization JSONObjectWithData:metadataData options:0 error:nil] : nil;
    NSDictionary *session = [metadata isKindOfClass:NSDictionary.class] ? metadata[@"snapshot"][@"session"] : nil;
    NSString *sessionID = [session[@"id"] isKindOfClass:NSString.class] ? session[@"id"] : path.lastPathComponent;
    unsigned long long startedAt = [session[@"started_at_unix_ms"] unsignedLongLongValue];
    [self displayRecordingTitle:[self recordingTitleAtPath:path sessionID:sessionID startedAt:startedAt]];
    self.savedSegments = [self readSavedSegmentsAtPath:path];
    NSString *transcript = [self plainTextForSegments:self.savedSegments];
    if (transcript.length) {
        self.transcript.string = transcript;
    } else {
        self.transcript.string = @"Transcript unavailable";
        self.transcript.textColor = [self color:101 green:117 blue:133 dark:170 green:185 blue:200];
    }
    [self loadAudioForSessionAtPath:path];
    self.source.stringValue = @"Saved session audio";
    self.timer.stringValue = self.player ? [self clock:self.player.duration] : @"00:00:00";
    self.status.stringValue = self.player ? @"✓ Saved" : @"Audio unavailable";
    [self refreshAppearance];
}

- (void)togglePlayback:(id)sender {
    (void)sender;
    if (!self.player) return;
    if (self.player.playing) {
        [self.player pause];
        [self.playbackTimer invalidate];
        self.playbackTimer = nil;
        self.playButton.image = [NSImage imageWithSystemSymbolName:@"play.fill" accessibilityDescription:@"Play recording"];
    } else if ([self.player play]) {
        self.playButton.image = [NSImage imageWithSystemSymbolName:@"pause.fill" accessibilityDescription:@"Pause recording"];
        self.playbackTimer = [NSTimer scheduledTimerWithTimeInterval:0.1 target:self selector:@selector(updatePlaybackPosition:) userInfo:nil repeats:YES];
    }
}

- (void)updatePlaybackPosition:(NSTimer *)timer {
    (void)timer;
    if (!self.player || !self.player.playing) return;
    self.progress.doubleValue = self.player.currentTime;
    self.currentTime.stringValue = [self clock:self.player.currentTime];
}

- (void)seek:(NSSlider *)sender {
    if (!self.player) return;
    self.player.currentTime = sender.doubleValue;
    self.currentTime.stringValue = [self clock:self.player.currentTime];
}

- (void)audioPlayerDidFinishPlaying:(AVAudioPlayer *)player successfully:(BOOL)flag {
    (void)player; (void)flag;
    [self stopPlaybackAndReleasePlayer];
    if (self.sessionPath.length) [self loadAudioForSessionAtPath:self.sessionPath];
}

- (void)stopPlaybackAndReleasePlayer {
    [self.playbackTimer invalidate];
    self.playbackTimer = nil;
    if (self.player) {
        self.player.delegate = nil;
        [self.player stop];
        self.player.currentTime = 0;
    }
    self.player = nil;
    self.progress.doubleValue = 0;
    self.currentTime.stringValue = @"00:00";
    if (self.playButton) self.playButton.image = [NSImage imageWithSystemSymbolName:@"play.fill" accessibilityDescription:@"Play recording"];
}

- (void)exportTXT:(id)sender {
    (void)sender;
    NSString *text = [self plainTextForSegments:self.savedSegments ?: @[]];
    if (!text.length) return;
    NSSavePanel *panel = [NSSavePanel savePanel];
    panel.nameFieldStringValue = @"transcript.txt";
    panel.allowedContentTypes = @[[UTType typeWithFilenameExtension:@"txt"]];
    [panel beginWithCompletionHandler:^(NSModalResponse result) {
        if (result == NSModalResponseOK) [text writeToURL:panel.URL atomically:YES encoding:NSUTF8StringEncoding error:nil];
    }];
}

- (void)exportJSON:(id)sender {
    (void)sender;
    NSString *jsonPath = [self.sessionPath stringByAppendingPathComponent:@"transcript.json"];
    if (![[NSFileManager defaultManager] fileExistsAtPath:jsonPath]) return;
    NSSavePanel *panel = [NSSavePanel savePanel];
    panel.nameFieldStringValue = @"transcript.json";
    panel.allowedContentTypes = @[[UTType typeWithFilenameExtension:@"json"]];
    [panel beginWithCompletionHandler:^(NSModalResponse result) {
        if (result == NSModalResponseOK) [[NSFileManager defaultManager] copyItemAtURL:[NSURL fileURLWithPath:jsonPath] toURL:panel.URL error:nil];
    }];
}

- (void)openLiveWithSnapshot:(NSDictionary *)snapshot transcriptState:(NSDictionary *)transcriptState {
    [self stopPlaybackAndReleasePlayer];
    self.mode = RimvViewerModeLive;
    self.liveSessionID = transcriptState[@"session_id"];
    self.sessionPath = snapshot[@"session"][@"recording_directory"];
    NSDictionary *session = snapshot[@"session"];
    [self displayRecordingTitle:[self recordingTitleAtPath:self.sessionPath sessionID:self.liveSessionID ?: session[@"id"] startedAt:[session[@"started_at_unix_ms"] unsignedLongLongValue]]];
    self.transcriptRevision = 0;
    [self build];
    [self displayRecordingTitle:[self recordingTitleAtPath:self.sessionPath sessionID:self.liveSessionID ?: session[@"id"] startedAt:[session[@"started_at_unix_ms"] unsignedLongLongValue]]];
    [self applyTranscriptState:transcriptState];
    [self applySnapshot:snapshot];
    if (!self.window.visible) {
        [NSApp activateIgnoringOtherApps:YES];
        [self.window makeKeyAndOrderFront:nil];
    } else {
        [self.window makeKeyAndOrderFront:nil];
    }
}

- (BOOL)windowShouldClose:(NSWindow *)sender {
    [self stopPlaybackAndReleasePlayer];
    if (self.allowWindowClose) return YES;
    // A menu-bar app must remain alive with no visible windows. Keep the
    // single viewer instance and hide it rather than closing AppKit's last
    // window; reopening repopulates it from the authoritative session state.
    [sender orderOut:nil];
    return NO;
}

- (void)windowWillClose:(NSNotification *)notification {
    (void)notification;
    [self stopPlaybackAndReleasePlayer];
    ((RimvViewerRootView *)self.content).appearanceChanged = nil;
    self.window.delegate = nil;
    self.window = nil;
    self.allowWindowClose = NO;
}

- (void)dealloc {
    [self stopPlaybackAndReleasePlayer];
    [[NSNotificationCenter defaultCenter] removeObserver:self];
}
@end

static RimvTranscriptionWindow *tw;

static id TWJSON(const char *value) {
    if (!value) return nil;
    NSString *string = [NSString stringWithUTF8String:value];
    if (!string) return nil;
    return [NSJSONSerialization JSONObjectWithData:[string dataUsingEncoding:NSUTF8StringEncoding] options:0 error:nil];
}

void rimv_transcription_window_configure(RimvTranscriptionCommandCallback command) {
    dispatch_block_t configure = ^{
        if (!tw) tw = [RimvTranscriptionWindow new];
        tw.command = command;
    };
    if (NSThread.isMainThread) configure();
    else dispatch_sync(dispatch_get_main_queue(), configure);
}

void rimv_transcription_window_snapshot(const char *snapshot_json) {
    NSDictionary *snapshot = TWJSON(snapshot_json);
    if (![snapshot isKindOfClass:NSDictionary.class]) return;
    dispatch_async(dispatch_get_main_queue(), ^{
        if (!tw) tw = [RimvTranscriptionWindow new];
        tw.snapshot = snapshot;
        if (tw.mode == RimvViewerModeLive && tw.window) [tw applySnapshot:snapshot];
    });
}

void rimv_transcription_window_update(const char *update_json) {
    NSDictionary *update = TWJSON(update_json);
    if (![update isKindOfClass:NSDictionary.class]) return;
    dispatch_async(dispatch_get_main_queue(), ^{
        if (!tw) tw = [RimvTranscriptionWindow new];
        [tw applyTranscriptUpdateEnvelope:update];
    });
}

void rimv_transcription_window_open(void) {
    dispatch_async(dispatch_get_main_queue(), ^{
        if (!tw) tw = [RimvTranscriptionWindow new];
        if (tw.command) tw.command(6, 0);
    });
}

void rimv_transcription_window_open_live(const char *snapshot_json, const char *transcript_state_json) {
    NSDictionary *snapshot = TWJSON(snapshot_json);
    NSDictionary *transcriptState = TWJSON(transcript_state_json);
    if (![snapshot isKindOfClass:NSDictionary.class] || ![transcriptState isKindOfClass:NSDictionary.class]) return;
    dispatch_async(dispatch_get_main_queue(), ^{
        if (!tw) tw = [RimvTranscriptionWindow new];
        [tw openLiveWithSnapshot:snapshot transcriptState:transcriptState];
    });
}

void rimv_transcription_window_open_session(const char *path) {
    NSString *sessionPath = path ? [NSString stringWithUTF8String:path] : nil;
    if (!sessionPath.length) return;
    dispatch_async(dispatch_get_main_queue(), ^{
        if (!tw) tw = [RimvTranscriptionWindow new];
        [tw stopPlaybackAndReleasePlayer];
        [tw build];
        [tw showCompletedSessionAtPath:sessionPath];
        [NSApp activateIgnoringOtherApps:YES];
        [tw.window makeKeyAndOrderFront:nil];
    });
}

void rimv_transcription_window_recording_metadata(const char *path, const char *title, uint8_t deleted) {
    NSString *sessionPath = path ? [NSString stringWithUTF8String:path] : nil;
    NSString *sessionTitle = title ? [NSString stringWithUTF8String:title] : nil;
    if (!sessionPath.length) return;
    dispatch_async(dispatch_get_main_queue(), ^{
        if (!tw || ![tw.sessionPath isEqualToString:sessionPath]) return;
        if (deleted) {
            tw.allowWindowClose = YES;
            [tw.window close];
        } else {
            [tw applyRecordingTitle:sessionTitle forPath:sessionPath];
        }
    });
}

void rimv_transcription_window_close_session(const char *path) {
    NSString *sessionPath = path ? [NSString stringWithUTF8String:path] : nil;
    if (!sessionPath.length) return;
    dispatch_block_t close = ^{
        if (tw && [tw.sessionPath isEqualToString:sessionPath]) {
            [tw stopPlaybackAndReleasePlayer];
            tw.allowWindowClose = YES;
            [tw.window close];
        }
    };
    if (NSThread.isMainThread) close();
    else dispatch_sync(dispatch_get_main_queue(), close);
}

void rimv_transcription_window_shutdown_playback(void) {
    dispatch_block_t stop = ^{ [tw stopPlaybackAndReleasePlayer]; };
    if (NSThread.isMainThread) stop();
    else dispatch_sync(dispatch_get_main_queue(), stop);
}
