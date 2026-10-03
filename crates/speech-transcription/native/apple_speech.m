#import <AVFoundation/AVFoundation.h>
#import <Speech/Speech.h>
#import <Foundation/Foundation.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

// SFSpeechRecognizer is the compatibility implementation for RimV's macOS
// 13 deployment target. SpeechAnalyzer/SpeechTranscriber require macOS 26 and
// a Swift AsyncSequence bridge; this path uses RimV-fed buffers and explicitly
// requires Apple's on-device recognizer instead of allowing server fallback.

static void RimvCopy(NSString *value, char *output, size_t capacity) {
    if (capacity == 0) return;
    const char *utf8 = value.UTF8String ?: "";
    snprintf(output, capacity, "%s", utf8);
}

static NSString *RimvPreferredLocale(NSArray<NSString *> *usableLocales) {
    for (NSString *preference in NSLocale.preferredLanguages) {
        NSString *canonical = [preference stringByReplacingOccurrencesOfString:@"_" withString:@"-"];
        for (NSString *candidate in usableLocales) {
            if ([candidate caseInsensitiveCompare:canonical] == NSOrderedSame) return candidate;
        }
        NSString *language = [[canonical componentsSeparatedByString:@"-"] firstObject];
        for (NSString *candidate in usableLocales) {
            NSString *candidateLanguage = [[candidate componentsSeparatedByString:@"-"] firstObject];
            if ([candidateLanguage caseInsensitiveCompare:language] == NSOrderedSame) return candidate;
        }
    }
    return usableLocales.firstObject;
}

bool rimv_apple_speech_available(const char *locale_id) {
    @autoreleasepool {
        SFSpeechRecognizerAuthorizationStatus authorization = SFSpeechRecognizer.authorizationStatus;
        if (authorization == SFSpeechRecognizerAuthorizationStatusDenied ||
            authorization == SFSpeechRecognizerAuthorizationStatusRestricted) return false;
        NSString *identifier = [NSString stringWithUTF8String:locale_id ?: "en-US"];
        SFSpeechRecognizer *recognizer = [[SFSpeechRecognizer alloc]
            initWithLocale:[[NSLocale alloc] initWithLocaleIdentifier:identifier]];
        return recognizer != nil && recognizer.isAvailable && recognizer.supportsOnDeviceRecognition;
    }
}

int32_t rimv_apple_speech_supported_locales(char *output, size_t output_capacity) {
    @autoreleasepool {
        if (!output || output_capacity == 0) return 0;
        NSMutableArray<NSString *> *supported = [NSMutableArray array];
        NSMutableArray<NSString *> *onDevice = [NSMutableArray array];
        NSMutableArray<NSString *> *usableOnDevice = [NSMutableArray array];
        for (NSLocale *locale in [SFSpeechRecognizer supportedLocales]) {
            NSString *identifier = [locale.localeIdentifier stringByReplacingOccurrencesOfString:@"_" withString:@"-"];
            SFSpeechRecognizer *recognizer = [[SFSpeechRecognizer alloc] initWithLocale:locale];
            if (identifier.length > 0 && recognizer) {
                [supported addObject:identifier];
                if (recognizer.supportsOnDeviceRecognition) {
                    [onDevice addObject:identifier];
                    if (recognizer.isAvailable) [usableOnDevice addObject:identifier];
                }
            }
        }
        [supported sortUsingSelector:@selector(compare:)];
        [onDevice sortUsingSelector:@selector(compare:)];
        [usableOnDevice sortUsingSelector:@selector(compare:)];
        NSString *preferredLocale = RimvPreferredLocale(usableOnDevice);
        NSDictionary *catalog = @{
            @"supported": supported,
            @"on_device": onDevice,
            @"usable_on_device": usableOnDevice,
            @"preferred_locale": preferredLocale ?: NSNull.null,
        };
        NSData *json = [NSJSONSerialization dataWithJSONObject:catalog options:0 error:NULL];
        if (!json || json.length + 1 > output_capacity) return -1;
        memcpy(output, json.bytes, json.length);
        output[json.length] = '\0';
        return (int32_t)usableOnDevice.count;
    }
}

bool rimv_apple_speech_authorized(void) {
    return SFSpeechRecognizer.authorizationStatus == SFSpeechRecognizerAuthorizationStatusAuthorized;
}

int32_t rimv_apple_speech_prepare(const char *locale_id, char *error, size_t error_capacity) {
    @autoreleasepool {
        __block SFSpeechRecognizerAuthorizationStatus authorization = SFSpeechRecognizer.authorizationStatus;
        if (authorization == SFSpeechRecognizerAuthorizationStatusNotDetermined) {
            dispatch_semaphore_t permission = dispatch_semaphore_create(0);
            [SFSpeechRecognizer requestAuthorization:^(SFSpeechRecognizerAuthorizationStatus status) {
                authorization = status;
                dispatch_semaphore_signal(permission);
            }];
            if (dispatch_semaphore_wait(permission, dispatch_time(DISPATCH_TIME_NOW, 30 * NSEC_PER_SEC)) != 0) {
                RimvCopy(@"Timed out waiting for Speech Recognition permission.", error, error_capacity);
                return 2;
            }
        }
        if (authorization != SFSpeechRecognizerAuthorizationStatusAuthorized) {
            RimvCopy(@"Speech Recognition permission is denied.", error, error_capacity);
            return 3;
        }
        NSString *identifier = [NSString stringWithUTF8String:locale_id ?: "en-US"];
        SFSpeechRecognizer *recognizer = [[SFSpeechRecognizer alloc]
            initWithLocale:[[NSLocale alloc] initWithLocaleIdentifier:identifier]];
        if (!recognizer || !recognizer.isAvailable) {
            RimvCopy(@"Apple Speech is unavailable for the selected locale.", error, error_capacity);
            return 4;
        }
        if (!recognizer.supportsOnDeviceRecognition) {
            RimvCopy(@"On-device Apple Speech is unavailable for the selected locale.", error, error_capacity);
            return 5;
        }
        return 0;
    }
}

int32_t rimv_apple_speech_transcribe(const char *locale_id,
                                     const float *samples,
                                     size_t sample_count,
                                     bool wait_for_final,
                                     char *output,
                                     size_t output_capacity,
                                     char *error,
                                     size_t error_capacity) {
    @autoreleasepool {
        if (!samples || sample_count == 0 || !output || output_capacity == 0) {
            RimvCopy(@"No audio samples were provided.", error, error_capacity);
            return 1;
        }
        __block SFSpeechRecognizerAuthorizationStatus authorization = SFSpeechRecognizer.authorizationStatus;
        if (authorization == SFSpeechRecognizerAuthorizationStatusNotDetermined) {
            dispatch_semaphore_t permission = dispatch_semaphore_create(0);
            [SFSpeechRecognizer requestAuthorization:^(SFSpeechRecognizerAuthorizationStatus status) {
                authorization = status;
                dispatch_semaphore_signal(permission);
            }];
            if (dispatch_semaphore_wait(permission, dispatch_time(DISPATCH_TIME_NOW, 30 * NSEC_PER_SEC)) != 0) {
                RimvCopy(@"Timed out waiting for Speech Recognition permission.", error, error_capacity);
                return 2;
            }
        }
        if (authorization != SFSpeechRecognizerAuthorizationStatusAuthorized) {
            RimvCopy(@"Speech Recognition permission is denied.", error, error_capacity);
            return 3;
        }

        NSString *identifier = [NSString stringWithUTF8String:locale_id ?: "en-US"];
        SFSpeechRecognizer *recognizer = [[SFSpeechRecognizer alloc]
            initWithLocale:[[NSLocale alloc] initWithLocaleIdentifier:identifier]];
        if (!recognizer || !recognizer.isAvailable) {
            RimvCopy(@"Apple Speech is unavailable for the selected locale.", error, error_capacity);
            return 4;
        }
        if (!recognizer.supportsOnDeviceRecognition) {
            RimvCopy(@"On-device Apple Speech is unavailable for the selected locale.", error, error_capacity);
            return 5;
        }

        AVAudioFormat *format = [[AVAudioFormat alloc] initWithCommonFormat:AVAudioPCMFormatFloat32
                                                                 sampleRate:16000
                                                                   channels:1
                                                                interleaved:NO];
        AVAudioPCMBuffer *buffer = [[AVAudioPCMBuffer alloc] initWithPCMFormat:format
                                                                 frameCapacity:(AVAudioFrameCount)sample_count];
        if (!buffer) {
            RimvCopy(@"Could not allocate an Apple Speech audio buffer.", error, error_capacity);
            return 6;
        }
        buffer.frameLength = (AVAudioFrameCount)sample_count;
        memcpy(buffer.floatChannelData[0], samples, sample_count * sizeof(float));

        SFSpeechAudioBufferRecognitionRequest *request = [SFSpeechAudioBufferRecognitionRequest new];
        request.shouldReportPartialResults = YES;
        request.requiresOnDeviceRecognition = YES;
        dispatch_semaphore_t completed = dispatch_semaphore_create(0);
        __block BOOL signaled = NO;
        __block NSString *recognized = @"";
        __block NSString *failure = nil;
        SFSpeechRecognitionTask *task = [recognizer recognitionTaskWithRequest:request
            resultHandler:^(SFSpeechRecognitionResult *result, NSError *taskError) {
                @synchronized (completed) {
                    if (taskError) failure = taskError.localizedDescription;
                    if (result.bestTranscription.formattedString.length > 0) {
                        recognized = result.bestTranscription.formattedString;
                    }
                    if (!signaled && ((wait_for_final && result.isFinal) || (!wait_for_final && recognized.length > 0) || taskError)) {
                        signaled = YES;
                        dispatch_semaphore_signal(completed);
                    }
                }
            }];
        [request appendAudioPCMBuffer:buffer];
        [request endAudio];
        const int64_t timeout_seconds = wait_for_final ? 20 : 2;
        const long timed_out = dispatch_semaphore_wait(completed,
            dispatch_time(DISPATCH_TIME_NOW, timeout_seconds * NSEC_PER_SEC));
        [task cancel];
        if (timed_out != 0) {
            RimvCopy(@"Apple Speech did not return a result before the timeout.", error, error_capacity);
            return 7;
        }
        if (failure.length > 0) {
            RimvCopy(failure, error, error_capacity);
            return 8;
        }
        RimvCopy(recognized, output, output_capacity);
        return 0;
    }
}
