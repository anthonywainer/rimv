#import "transcription_window.h"
#import <AVFoundation/AVFoundation.h>

static BOOL TWIsDark(NSAppearance *a) { return [[a bestMatchFromAppearancesWithNames:@[NSAppearanceNameAqua,NSAppearanceNameDarkAqua]] isEqualToString:NSAppearanceNameDarkAqua]; }
@interface RimvTranscriptionWindow : NSObject <NSWindowDelegate>
@property(nonatomic) RimvTranscriptionCommandCallback command;
@property(nonatomic,strong) NSWindow *window;
@property(nonatomic,strong) NSTextField *source;
@property(nonatomic,strong) NSTextField *timer;
@property(nonatomic,strong) NSTextField *status;
@property(nonatomic,strong) NSTextView *transcript;
@property(nonatomic,strong) NSTextView *partial;
@property(nonatomic,strong) NSButton *stop;
@property(nonatomic,strong) NSButton *startAnotherButton;
@property(nonatomic,strong) NSButton *exportTXTButton;
@property(nonatomic,strong) NSButton *exportJSONButton;
@property(nonatomic,strong) NSDictionary *snapshot;
@property(nonatomic,copy) NSString *stableText;
@property(nonatomic,copy) NSString *unstableText;
@property(nonatomic,copy) NSString *sessionPath;
@property(nonatomic,strong) NSMutableArray<NSString *> *finalLines;
@property(nonatomic,strong) AVAudioPlayer *player;
@property(nonatomic,strong) NSButton *playButton;
@property(nonatomic) BOOL userClosed;
@end
@implementation RimvTranscriptionWindow
- (NSColor *)color:(CGFloat)lr :(CGFloat)lg :(CGFloat)lb dark:(CGFloat)dr :(CGFloat)dg :(CGFloat)db { BOOL d=TWIsDark(self.window.effectiveAppearance ?: NSApp.effectiveAppearance); return [NSColor colorWithRed:(d?dr:lr)/255.0 green:(d?dg:lg)/255.0 blue:(d?db:lb)/255.0 alpha:1]; }
- (NSTextField *)label:(NSString *)text frame:(NSRect)f size:(CGFloat)s weight:(NSFontWeight)w { NSTextField *v=[NSTextField labelWithString:text ?: @""]; v.frame=f; v.font=[NSFont systemFontOfSize:s weight:w]; v.textColor=[self color:17 :20 :24 dark:244 :247 :251]; return v; }
- (void)build {
 if(self.window)return; self.window=[[NSWindow alloc]initWithContentRect:NSMakeRect(0,0,760,610) styleMask:(NSWindowStyleMaskTitled|NSWindowStyleMaskClosable|NSWindowStyleMaskResizable) backing:NSBackingStoreBuffered defer:NO]; self.window.title=@"RimV — Transcription"; self.window.minSize=NSMakeSize(560,440); self.window.delegate=self;
 NSView *v=self.window.contentView; v.wantsLayer=YES; v.layer.backgroundColor=[self color:247 :249 :251 dark:8 :21 :34].CGColor;
 NSTextField *brand=[self label:@"≋  RimV" frame:NSMakeRect(22,568,220,24) size:20 weight:NSFontWeightBold]; [v addSubview:brand];
 NSView *audio=[[NSView alloc]initWithFrame:NSMakeRect(20,452,720,96)]; audio.autoresizingMask=NSViewWidthSizable; audio.wantsLayer=YES; audio.layer.cornerRadius=14; audio.layer.backgroundColor=[self color:243 :244 :246 dark:16 :36 :55].CGColor; [v addSubview:audio];
 NSTextField *heading=[self label:@"Audio" frame:NSMakeRect(18,59,200,20) size:17 weight:NSFontWeightSemibold]; [audio addSubview:heading]; self.source=[self label:@"Capturing from selected source" frame:NSMakeRect(18,35,350,18) size:13 weight:NSFontWeightRegular]; [audio addSubview:self.source]; self.timer=[self label:@"00:00:00" frame:NSMakeRect(570,57,132,20) size:17 weight:NSFontWeightMedium]; self.timer.alignment=NSTextAlignmentRight; [audio addSubview:self.timer]; self.status=[self label:@"● Ready" frame:NSMakeRect(570,32,132,18) size:13 weight:NSFontWeightMedium]; self.status.alignment=NSTextAlignmentRight; [audio addSubview:self.status];
 self.playButton=[NSButton buttonWithTitle:@"▶ Play Audio" target:self action:@selector(togglePlayback:)]; self.playButton.frame=NSMakeRect(385,31,120,30); self.playButton.hidden=YES; [audio addSubview:self.playButton];
 NSTextField *th=[self label:@"Real-time transcription" frame:NSMakeRect(22,416,310,22) size:18 weight:NSFontWeightSemibold]; [v addSubview:th];
 NSScrollView *scroll=[[NSScrollView alloc]initWithFrame:NSMakeRect(20,126,720,278)]; scroll.autoresizingMask=NSViewWidthSizable|NSViewHeightSizable; scroll.hasVerticalScroller=YES; self.transcript=[[NSTextView alloc]initWithFrame:scroll.bounds]; self.transcript.editable=NO; self.transcript.selectable=YES; self.transcript.font=[NSFont systemFontOfSize:17]; self.transcript.textColor=[self color:17 :20 :24 dark:244 :247 :251]; self.transcript.backgroundColor=[self color:255 :255 :255 dark:13 :31 :48]; scroll.documentView=self.transcript; [v addSubview:scroll];
 self.partial=[[NSTextView alloc]initWithFrame:NSMakeRect(28,140,704,40)]; self.partial.editable=NO; self.partial.selectable=NO; self.partial.font=[NSFont systemFontOfSize:17]; self.partial.textColor=[self color:107 :114 :128 dark:169 :183 :200]; self.partial.backgroundColor=NSColor.clearColor; [v addSubview:self.partial];
 self.stop=[NSButton buttonWithTitle:@"■  Stop" target:self action:@selector(stop:)]; self.stop.frame=NSMakeRect(20,42,155,58); self.stop.font=[NSFont systemFontOfSize:16 weight:NSFontWeightSemibold]; self.stop.bezelStyle=NSBezelStyleRounded; [v addSubview:self.stop];
 self.startAnotherButton=[NSButton buttonWithTitle:@"＋  New Recording" target:self action:@selector(newRecording:)]; self.startAnotherButton.frame=NSMakeRect(560,57,180,32); self.startAnotherButton.hidden=YES; [v addSubview:self.startAnotherButton];
 self.exportTXTButton=[NSButton buttonWithTitle:@"Export TXT" target:self action:@selector(exportTXT:)]; self.exportTXTButton.frame=NSMakeRect(190,58,105,32); self.exportTXTButton.hidden=YES; [v addSubview:self.exportTXTButton];
 self.exportJSONButton=[NSButton buttonWithTitle:@"Export JSON" target:self action:@selector(exportJSON:)]; self.exportJSONButton.frame=NSMakeRect(304,58,112,32); self.exportJSONButton.hidden=YES; [v addSubview:self.exportJSONButton];
}
- (void)stop:(id)sender { (void)sender; self.stop.enabled=NO; self.status.stringValue=@"● Processing…"; self.command(2,0); }
- (void)newRecording:(id)sender { (void)sender; self.command(1,0); }
- (void)applySnapshot:(NSDictionary *)s { self.snapshot=s; NSString *state=s[@"status"] ?: @"idle"; NSDictionary *mic=[s[@"microphone"] isKindOfClass:NSDictionary.class] ? s[@"microphone"] : @{}; NSDictionary *sys=[s[@"system_audio"] isKindOfClass:NSDictionary.class] ? s[@"system_audio"] : @{}; BOOL m=[mic[@"enabled"] boolValue], y=[sys[@"enabled"] boolValue]; self.source.stringValue=m&&y?@"Capturing from Microphone + System":m?@"Capturing from Microphone":y?@"Capturing from System":@"No capture source selected"; uint64_t ms=[s[@"elapsed_ms"] unsignedLongLongValue]; self.timer.stringValue=[NSString stringWithFormat:@"%02llu:%02llu:%02llu",ms/3600000,(ms/60000)%60,(ms/1000)%60]; id session=s[@"session"]; if([session isKindOfClass:NSDictionary.class]) self.sessionPath=session[@"recording_directory"];
 BOOL live=[state isEqual:@"recording"]||[state isEqual:@"starting"]||[state isEqual:@"stopping"]; if(live && !self.window.visible && !self.userClosed){ [self build]; [self.window makeKeyAndOrderFront:nil]; }
 if([state isEqual:@"recording"]) { self.status.stringValue=@"● Recording"; self.stop.hidden=NO; self.stop.enabled=YES; self.startAnotherButton.hidden=YES; self.exportTXTButton.hidden=YES; self.exportJSONButton.hidden=YES; }
 else if([state isEqual:@"stopping"]) { self.status.stringValue=@"● Processing…"; self.stop.enabled=NO; }
 else if([state isEqual:@"idle"] && self.sessionPath.length){ self.status.stringValue=@"✓ Completed"; self.stop.hidden=YES; self.startAnotherButton.hidden=NO; self.exportTXTButton.hidden=NO; self.exportJSONButton.hidden=NO; [self loadFinal]; }
}
- (void)applyUpdate:(NSDictionary *)u { self.stableText=u[@"stable_text"] ?: @""; self.unstableText=u[@"unstable_text"] ?: @""; if(!self.finalLines) self.finalLines=[NSMutableArray array]; if([u[@"is_final"] boolValue] && self.stableText.length) [self.finalLines addObject:self.stableText]; if(!self.window)return; BOOL follows=self.transcript.enclosingScrollView.verticalScroller.floatValue > .95; self.transcript.string=[self.finalLines componentsJoinedByString:@"\n\n"]; NSArray *parts=[@[self.stableText,self.unstableText] filteredArrayUsingPredicate:[NSPredicate predicateWithFormat:@"length > 0"]]; self.partial.string=[parts componentsJoinedByString:@" "]; if(follows||self.transcript.string.length<2)[self.transcript scrollRangeToVisible:NSMakeRange(self.transcript.string.length,0)]; }
- (void)loadFinal { NSString *file=[self.sessionPath stringByAppendingPathComponent:@"transcript.txt"]; NSString *final=[NSString stringWithContentsOfFile:file encoding:NSUTF8StringEncoding error:nil]; if(final){ self.transcript.string=final; self.partial.string=@""; } for(NSString *name in @[@"microphone.wav",@"system.wav"]){ NSString *audio=[self.sessionPath stringByAppendingPathComponent:name]; if([[NSFileManager defaultManager]fileExistsAtPath:audio]){ self.player=[[AVAudioPlayer alloc]initWithContentsOfURL:[NSURL fileURLWithPath:audio] error:nil]; [self.player prepareToPlay]; self.playButton.hidden=self.player==nil; break; } } }
- (void)togglePlayback:(id)sender { (void)sender; if(!self.player)return; if(self.player.playing){ [self.player pause]; self.playButton.title=@"▶ Play Audio"; } else { [self.player play]; self.playButton.title=@"❚❚ Pause"; } }
- (void)export:(NSString *)name { if(!self.sessionPath.length)return; NSString *source=[self.sessionPath stringByAppendingPathComponent:name]; if(![[NSFileManager defaultManager]fileExistsAtPath:source])return; NSSavePanel *panel=[NSSavePanel savePanel]; panel.nameFieldStringValue=name; if([panel runModal]==NSModalResponseOK){ [[NSFileManager defaultManager]copyItemAtURL:[NSURL fileURLWithPath:source] toURL:panel.URL error:nil]; } }
- (void)exportTXT:(id)sender { (void)sender; [self export:@"transcript.txt"]; }
- (void)exportJSON:(id)sender { (void)sender; [self export:@"transcript.json"]; }
- (BOOL)windowShouldClose:(id)sender { (void)sender; self.userClosed=YES; return YES; }
- (void)windowWillClose:(NSNotification *)notification { (void)notification; self.window=nil; }
- (void)open { self.userClosed=NO; [self build]; [self applySnapshot:self.snapshot ?: @{}]; [NSApp activateIgnoringOtherApps:YES]; [self.window makeKeyAndOrderFront:nil]; }
@end
static RimvTranscriptionWindow *tw;
static id JSON(const char *v){ if(!v)return nil; return [NSJSONSerialization JSONObjectWithData:[[NSString stringWithUTF8String:v] dataUsingEncoding:NSUTF8StringEncoding] options:0 error:nil]; }
void rimv_transcription_window_configure(RimvTranscriptionCommandCallback command){ dispatch_async(dispatch_get_main_queue(), ^{ if(!tw)tw=[RimvTranscriptionWindow new]; tw.command=command; }); }
void rimv_transcription_window_snapshot(const char *json){ NSDictionary *s=JSON(json); if(![s isKindOfClass:NSDictionary.class])return; dispatch_async(dispatch_get_main_queue(), ^{ [tw build]; [tw applySnapshot:s]; }); }
void rimv_transcription_window_update(const char *json){ NSDictionary *u=JSON(json); if(![u isKindOfClass:NSDictionary.class])return; dispatch_async(dispatch_get_main_queue(), ^{ [tw applyUpdate:u]; }); }
void rimv_transcription_window_open(void){ dispatch_async(dispatch_get_main_queue(), ^{ [tw open]; }); }
void rimv_transcription_window_open_session(const char *path){ NSString *p=path?[NSString stringWithUTF8String:path]:nil; dispatch_async(dispatch_get_main_queue(), ^{ if(!tw)tw=[RimvTranscriptionWindow new]; tw.sessionPath=p; tw.userClosed=NO; [tw build]; [tw loadFinal]; tw.status.stringValue=@"✓ Completed"; tw.stop.hidden=YES; tw.startAnotherButton.hidden=NO; tw.exportTXTButton.hidden=NO; tw.exportJSONButton.hidden=NO; [NSApp activateIgnoringOtherApps:YES]; [tw.window makeKeyAndOrderFront:nil]; }); }
