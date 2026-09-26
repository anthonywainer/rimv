#import "recordings_selector.h"
#import "transcription_window.h"

extern void rimv_menu_selector_did_close(NSInteger);
extern bool rimv_menu_release_popover_focus(NSWindow *, NSInteger);
extern void rimv_recording_rename(const char *, const char *);
extern void rimv_recording_delete(const char *);

static NSDate *RimvRecordingDate(NSDictionary *session) {
    return [NSDate dateWithTimeIntervalSince1970:[session[@"started_at_unix_ms"] doubleValue] / 1000.0];
}

static NSString *RimvRecordingName(NSDictionary *session) {
    NSString *name = [session[@"name"] isKindOfClass:NSString.class] ? session[@"name"] : nil;
    if (name.length) return name;
    NSDateFormatter *formatter = [NSDateFormatter new];
    formatter.locale = NSLocale.currentLocale;
    formatter.dateFormat = @"HH:mm";
    return [NSString stringWithFormat:@"Recording — %@", [formatter stringFromDate:RimvRecordingDate(session)]];
}

static NSString *RimvRecordingDetail(NSDictionary *session) {
    NSDateFormatter *formatter = [NSDateFormatter new];
    formatter.locale = NSLocale.currentLocale;
    formatter.dateStyle = NSDateFormatterMediumStyle;
    formatter.timeStyle = NSDateFormatterShortStyle;
    unsigned long long duration = [session[@"duration_ms"] unsignedLongLongValue] / 1000;
    NSString *durationText = duration >= 3600
        ? [NSString stringWithFormat:@"%02llu:%02llu:%02llu", duration / 3600, (duration / 60) % 60, duration % 60]
        : [NSString stringWithFormat:@"%02llu:%02llu", duration / 60, duration % 60];
    return [NSString stringWithFormat:@"%@ · %@", [formatter stringFromDate:RimvRecordingDate(session)], durationText];
}

@interface RimvRecordingListView : NSView
@end
@implementation RimvRecordingListView
- (BOOL)isFlipped { return NO; }
@end

@interface RimvRecordingLabel : NSTextField
@end
@implementation RimvRecordingLabel
- (NSView *)hitTest:(NSPoint)point { (void)point; return nil; }
@end

@interface RimvRecordingIcon : NSImageView
@end
@implementation RimvRecordingIcon
- (NSView *)hitTest:(NSPoint)point { (void)point; return nil; }
@end

@interface RimvRecordingRow : NSView <NSTextFieldDelegate>
@property(nonatomic, weak) id owner;
@property(nonatomic, copy) NSDictionary *session;
@property(nonatomic, strong) NSTextField *titleLabel;
@property(nonatomic, strong) NSTextField *titleEditor;
@property(nonatomic, strong) NSTextField *detailLabel;
@property(nonatomic, strong) NSTextField *stateLabel;
@property(nonatomic, strong) NSButton *actionsButton;
@property(nonatomic, strong) NSButton *saveButton;
@property(nonatomic, strong) NSButton *cancelButton;
@property(nonatomic, strong) NSTrackingArea *rowTrackingArea;
@property(nonatomic) BOOL rowHovered;
- (instancetype)initWithSession:(NSDictionary *)session owner:(id)owner;
- (void)reload;
- (void)beginRenameWithValue:(NSString *)value;
@end

@interface RimvRecordings : NSObject <NSSearchFieldDelegate, NSPopoverDelegate>
@property(nonatomic,strong) NSPopover *popover;
@property(nonatomic,strong) NSArray<NSDictionary *> *sessions;
@property(nonatomic,strong) NSSearchField *search;
@property(nonatomic,strong) NSView *list;
@property(nonatomic,copy) NSString *filter;
@property(nonatomic,strong) NSArray<NSButton *> *filterButtons;
@property(nonatomic,copy) NSString *editingSessionPath;
@property(nonatomic,copy) NSString *editingTitle;
- (void)reload;
- (void)openSession:(NSDictionary *)session;
- (void)beginRename:(NSDictionary *)session;
- (void)deleteSession:(NSDictionary *)session;
@end

@implementation RimvRecordingRow
- (instancetype)initWithSession:(NSDictionary *)session owner:(id)owner {
    if ((self = [super initWithFrame:NSMakeRect(0, 0, 300, 62)])) {
        _session = [session copy];
        _owner = owner;
        self.wantsLayer = YES;
        self.layer.cornerRadius = 8;
        self.titleLabel = (NSTextField *)[RimvRecordingLabel labelWithString:RimvRecordingName(session)];
        self.titleLabel.font = [NSFont systemFontOfSize:13 weight:NSFontWeightMedium];
        self.titleLabel.lineBreakMode = NSLineBreakByTruncatingTail;
        self.titleLabel.frame = NSMakeRect(36, 34, 207, 20);
        [self addSubview:self.titleLabel];
        self.detailLabel = (NSTextField *)[RimvRecordingLabel labelWithString:RimvRecordingDetail(session)];
        self.detailLabel.font = [NSFont systemFontOfSize:11];
        self.detailLabel.textColor = NSColor.secondaryLabelColor;
        self.detailLabel.lineBreakMode = NSLineBreakByTruncatingTail;
        self.detailLabel.frame = NSMakeRect(36, 12, 230, 18);
        [self addSubview:self.detailLabel];
        BOOL recording = [session[@"state"] isEqual:@"recording"];
        NSImageView *icon = [[RimvRecordingIcon alloc] initWithFrame:NSMakeRect(10, 23, 18, 18)];
        icon.image = [NSImage imageWithSystemSymbolName:recording ? @"record.circle.fill" : @"play.circle.fill" accessibilityDescription:recording ? @"Recording" : @"Completed recording"];
        icon.contentTintColor = recording ? NSColor.systemRedColor : NSColor.secondaryLabelColor;
        [self addSubview:icon];
        self.stateLabel = (NSTextField *)[RimvRecordingLabel labelWithString:recording ? @"Recording" : @"Completed"];
        self.stateLabel.font = [NSFont systemFontOfSize:10 weight:NSFontWeightMedium];
        self.stateLabel.textColor = recording ? NSColor.systemRedColor : NSColor.secondaryLabelColor;
        self.stateLabel.alignment = NSTextAlignmentRight;
        self.stateLabel.frame = NSMakeRect(206, 12, 78, 17);
        [self addSubview:self.stateLabel];
        self.actionsButton = [NSButton buttonWithTitle:@"" target:self action:@selector(showActions:)];
        self.actionsButton.frame = NSMakeRect(264, 31, 28, 26);
        self.actionsButton.bordered = NO;
        self.actionsButton.image = [NSImage imageWithSystemSymbolName:@"ellipsis" accessibilityDescription:@"Recording actions"];
        self.actionsButton.imagePosition = NSImageOnly;
        self.actionsButton.contentTintColor = NSColor.secondaryLabelColor;
        self.actionsButton.accessibilityLabel = @"Recording actions";
        [self addSubview:self.actionsButton];
        [self reload];
    }
    return self;
}
- (void)reload {
    NSString *best = [self.effectiveAppearance bestMatchFromAppearancesWithNames:@[NSAppearanceNameAqua, NSAppearanceNameDarkAqua]];
    BOOL dark = [best isEqualToString:NSAppearanceNameDarkAqua];
    CGFloat alpha = self.rowHovered ? 0.11 : 0.035;
    self.layer.backgroundColor = (dark
        ? [NSColor colorWithWhite:1 alpha:self.rowHovered ? 0.11 : 0.055]
        : [NSColor colorWithWhite:0 alpha:alpha]).CGColor;
}
- (void)viewDidChangeEffectiveAppearance { [super viewDidChangeEffectiveAppearance]; [self reload]; }
- (void)updateTrackingAreas {
    [super updateTrackingAreas];
    if (self.rowTrackingArea) [self removeTrackingArea:self.rowTrackingArea];
    self.rowTrackingArea = [[NSTrackingArea alloc] initWithRect:NSZeroRect
        options:(NSTrackingMouseEnteredAndExited | NSTrackingActiveAlways | NSTrackingInVisibleRect)
        owner:self userInfo:nil];
    [self addTrackingArea:self.rowTrackingArea];
}
- (void)resetCursorRects {
    [super resetCursorRects];
    [self addCursorRect:self.bounds cursor:NSCursor.pointingHandCursor];
}
- (void)mouseEntered:(NSEvent *)event { (void)event; self.rowHovered = YES; [self reload]; }
- (void)mouseExited:(NSEvent *)event { (void)event; self.rowHovered = NO; [self reload]; }
- (void)mouseDown:(NSEvent *)event {
    (void)event;
    if (!self.titleEditor) [self.owner openSession:self.session];
}
- (void)beginRenameWithValue:(NSString *)value {
    if (self.titleEditor) return;
    self.titleLabel.hidden = YES;
    self.detailLabel.hidden = YES;
    self.titleEditor = [[NSTextField alloc] initWithFrame:NSMakeRect(36, 29, 164, 24)];
    self.titleEditor.stringValue = value ?: @"";
    self.titleEditor.accessibilityLabel = @"Recording name";
    [self addSubview:self.titleEditor];
    self.saveButton = [NSButton buttonWithTitle:@"Save" target:self.owner action:@selector(saveRename:)];
    self.saveButton.frame = NSMakeRect(202, 28, 46, 26);
    self.saveButton.identifier = self.session[@"path"];
    [self addSubview:self.saveButton];
    self.cancelButton = [NSButton buttonWithTitle:@"Cancel" target:self.owner action:@selector(cancelRename:)];
    self.cancelButton.frame = NSMakeRect(250, 28, 46, 26);
    [self addSubview:self.cancelButton];
}
- (void)showActions:(id)sender {
    (void)sender;
    NSMenu *menu = [NSMenu new];
    NSMenuItem *open = [[NSMenuItem alloc] initWithTitle:@"Open" action:@selector(open:) keyEquivalent:@""];
    open.target = self;
    [menu addItem:open];
    NSMenuItem *rename = [[NSMenuItem alloc] initWithTitle:@"Rename…" action:@selector(rename:) keyEquivalent:@""];
    rename.target = self;
    [menu addItem:rename];
    [menu addItem:NSMenuItem.separatorItem];
    NSMenuItem *delete = [[NSMenuItem alloc] initWithTitle:@"Delete…" action:@selector(delete:) keyEquivalent:@""];
    delete.target = self;
    delete.enabled = ![self.session[@"state"] isEqual:@"recording"];
    [menu addItem:delete];
    [NSMenu popUpContextMenu:menu withEvent:NSApp.currentEvent forView:self.actionsButton];
}
- (void)open:(id)sender { (void)sender; [self.owner openSession:self.session]; }
- (void)rename:(id)sender { (void)sender; [self.owner beginRename:self.session]; }
- (void)delete:(id)sender { (void)sender; [self.owner deleteSession:self.session]; }
@end

@implementation RimvRecordings
- (void)show:(NSView *)anchor {
    if (self.popover.shown) return;
    NSView *view = [[NSView alloc] initWithFrame:NSMakeRect(0, 0, 336, 440)];
    NSTextField *heading = [NSTextField labelWithString:@"Recordings"];
    heading.frame = NSMakeRect(18, 404, 180, 22);
    heading.font = [NSFont systemFontOfSize:17 weight:NSFontWeightSemibold];
    [view addSubview:heading];
    self.search = [[NSSearchField alloc] initWithFrame:NSMakeRect(18, 356, 300, 36)];
    self.search.placeholderString = @"Search recordings...";
    self.search.delegate = self;
    [view addSubview:self.search];
    self.filter = @"all";
    NSArray *titles = @[@"All", @"Recording", @"Completed"];
    NSMutableArray *buttons = [NSMutableArray array];
    for (NSUInteger index = 0; index < titles.count; index++) {
        NSButton *button = [NSButton buttonWithTitle:titles[index] target:self action:@selector(filter:)];
        button.tag = (NSInteger)index;
        button.frame = NSMakeRect(18 + index * 94, 320, 88, 25);
        [view addSubview:button];
        [buttons addObject:button];
    }
    self.filterButtons = buttons;
    NSScrollView *scroll = [[NSScrollView alloc] initWithFrame:NSMakeRect(18, 50, 300, 260)];
    scroll.hasVerticalScroller = YES;
    scroll.autohidesScrollers = YES;
    self.list = [[RimvRecordingListView alloc] initWithFrame:NSMakeRect(0, 0, 300, 1)];
    scroll.documentView = self.list;
    [view addSubview:scroll];
    NSButton *folder = [NSButton buttonWithTitle:@"Open Recordings Folder" target:self action:@selector(folder:)];
    folder.frame = NSMakeRect(18, 14, 200, 28);
    [view addSubview:folder];
    NSViewController *controller = [NSViewController new];
    controller.view = view;
    self.popover = [NSPopover new];
    self.popover.behavior = NSPopoverBehaviorApplicationDefined;
    self.popover.delegate = self;
    self.popover.contentSize = view.bounds.size;
    self.popover.contentViewController = controller;
    [self reload];
    [self.popover showRelativeToRect:anchor.bounds ofView:anchor preferredEdge:NSRectEdgeMaxX];
}
- (void)focusSearch {
    NSWindow *window = self.popover.contentViewController.view.window;
    if (!self.popover.shown || !window || !self.search) return;
    (void)[window makeFirstResponder:self.search];
}
- (BOOL)releaseSelectorFocus {
    NSWindow *window = self.popover.contentViewController.view.window;
    if (!self.popover.shown) return YES;
    return rimv_menu_release_popover_focus(window, 3);
}
- (BOOL)closeSelectorSafely {
    if (!self.popover.shown) return YES;
    if (![self releaseSelectorFocus]) return NO;
    [self.popover performClose:nil];
    return YES;
}
- (void)filter:(NSButton *)button {
    self.filter = @[@"all", @"recording", @"completed"][button.tag];
    for (NSButton *item in self.filterButtons) item.state = item == button ? NSControlStateValueOn : NSControlStateValueOff;
    [self reload];
}
- (void)controlTextDidChange:(NSNotification *)notification { (void)notification; [self reload]; }
- (void)reload {
    if (!self.list) return;
    for (RimvRecordingRow *row in self.list.subviews) {
        if ([row isKindOfClass:RimvRecordingRow.class] && row.titleEditor) {
            self.editingSessionPath = row.session[@"path"];
            self.editingTitle = row.titleEditor.stringValue;
            break;
        }
    }
    for (NSView *view in self.list.subviews.copy) [view removeFromSuperview];
    NSString *query = self.search.stringValue.lowercaseString ?: @"";
    NSMutableArray<NSDictionary *> *matches = [NSMutableArray array];
    for (NSDictionary *session in self.sessions ?: @[]) {
        NSString *state = session[@"state"] ?: @"completed";
        if (![self.filter isEqual:@"all"] && ![state isEqual:self.filter]) continue;
        NSString *name = RimvRecordingName(session);
        if (query.length && ![name.lowercaseString containsString:query]) continue;
        [matches addObject:session];
    }
    CGFloat rowHeight = 66;
    CGFloat height = MAX(1, matches.count * rowHeight);
    self.list.frame = NSMakeRect(0, 0, 300, height);
    for (NSUInteger index = 0; index < matches.count; index++) {
        NSDictionary *session = matches[index];
        RimvRecordingRow *row = [[RimvRecordingRow alloc] initWithSession:session owner:self];
        row.frame = NSMakeRect(0, height - (index + 1) * rowHeight, 300, 62);
        [self.list addSubview:row];
        if ([session[@"path"] isEqual:self.editingSessionPath]) [row beginRenameWithValue:self.editingTitle ?: RimvRecordingName(session)];
    }
    if (!matches.count) {
        NSString *message = self.sessions.count ? @"No recordings match your search." : @"No recordings yet.";
        NSTextField *empty = [NSTextField labelWithString:message];
        empty.frame = NSMakeRect(12, 120, 276, 22);
        empty.alignment = NSTextAlignmentCenter;
        empty.textColor = NSColor.secondaryLabelColor;
        [self.list addSubview:empty];
    }
}
- (void)openSession:(NSDictionary *)session {
    NSString *path = session[@"path"];
    if (![path isKindOfClass:NSString.class]) return;
    if ([session[@"state"] isEqual:@"recording"]) rimv_transcription_window_open();
    else rimv_transcription_window_open_session(path.UTF8String);
}
- (void)beginRename:(NSDictionary *)session {
    NSString *path = session[@"path"];
    if (![path isKindOfClass:NSString.class]) return;
    for (RimvRecordingRow *row in self.list.subviews) {
        if (![row isKindOfClass:RimvRecordingRow.class] || ![row.session[@"path"] isEqual:path]) continue;
        self.editingSessionPath = path;
        self.editingTitle = RimvRecordingName(session);
        [row beginRenameWithValue:self.editingTitle];
        [row.titleEditor becomeFirstResponder];
        break;
    }
}
- (void)saveRename:(NSButton *)button {
    RimvRecordingRow *row = (RimvRecordingRow *)button.superview;
    NSString *title = row.titleEditor.stringValue;
    const char *path = button.identifier.UTF8String;
    const char *name = title.UTF8String;
    if (path && name) rimv_recording_rename(path, name);
    self.editingSessionPath = nil;
    self.editingTitle = nil;
    [row.titleEditor removeFromSuperview];
    [row.saveButton removeFromSuperview];
    [row.cancelButton removeFromSuperview];
    row.titleEditor = nil;
    row.saveButton = nil;
    row.cancelButton = nil;
    row.titleLabel.hidden = NO;
    row.detailLabel.hidden = NO;
}
- (void)cancelRename:(NSButton *)button {
    RimvRecordingRow *row = (RimvRecordingRow *)button.superview;
    self.editingSessionPath = nil;
    self.editingTitle = nil;
    [row.titleEditor removeFromSuperview];
    [row.saveButton removeFromSuperview];
    [row.cancelButton removeFromSuperview];
    row.titleEditor = nil;
    row.saveButton = nil;
    row.cancelButton = nil;
    row.titleLabel.hidden = NO;
    row.detailLabel.hidden = NO;
}
- (void)deleteSession:(NSDictionary *)session {
    if ([session[@"state"] isEqual:@"recording"]) return;
    NSAlert *alert = [NSAlert new];
    alert.messageText = @"Delete this recording?";
    alert.informativeText = @"Its audio and transcript files will be permanently deleted.";
    [alert addButtonWithTitle:@"Delete Recording"];
    [alert addButtonWithTitle:@"Cancel"];
    if ([alert runModal] != NSAlertFirstButtonReturn) return;
    NSString *path = session[@"path"];
    if (path.length) rimv_recording_delete(path.UTF8String);
}
- (void)folder:(id)sender {
    (void)sender;
    NSString *path = self.sessions.firstObject[@"root"];
    if (path) [[NSWorkspace sharedWorkspace] openURL:[NSURL fileURLWithPath:path isDirectory:YES]];
}
- (void)popoverDidClose:(NSNotification *)notification {
    if (notification.object == self.popover) rimv_menu_selector_did_close(3);
}
@end

static RimvRecordings *rs;
static NSArray *parse(const char *json) {
    if (!json) return @[];
    NSData *data = [[NSString stringWithUTF8String:json] dataUsingEncoding:NSUTF8StringEncoding];
    id value = data ? [NSJSONSerialization JSONObjectWithData:data options:0 error:nil] : nil;
    return [value isKindOfClass:NSArray.class] ? value : @[];
}
void rimv_recordings_selector_update(const char *json) {
    NSArray *sessions = parse(json);
    dispatch_async(dispatch_get_main_queue(), ^{
        if (!rs) rs = [RimvRecordings new];
        rs.sessions = sessions;
        [rs reload];
    });
}
void rimv_recordings_selector_show(NSView *anchor) {
    if (!rs) rs = [RimvRecordings new];
    [rs show:anchor];
    [rs focusSearch];
}
bool rimv_recordings_selector_contains_window(NSWindow *window) { return rs.popover.shown && rs.popover.contentViewController.view.window == window; }
bool rimv_recordings_selector_is_shown(void) { return rs.popover.isShown; }
bool rimv_recordings_selector_close(void) { return [rs closeSelectorSafely]; }
