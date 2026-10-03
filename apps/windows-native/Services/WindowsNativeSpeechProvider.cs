#pragma warning disable CA1416
#pragma warning disable SYSLIB0001
using Microsoft.Windows.AI;
using Microsoft.Windows.AI.Speech;
using RimV.Windows.Application;
using System.Collections.Concurrent;
using System.Runtime.InteropServices;
using System.Text;
using Windows.Foundation;
using Windows.Foundation.Metadata;
using Windows.System.UserProfile;

namespace RimV.Windows;

/// <summary>
/// Windows App SDK experimental speech adapter. Windows AI types stay inside
/// this host boundary; the shared Rust core receives only normalized text.
/// Input is PCM from RimV capture and never opens an audio device itself.
/// </summary>
internal sealed class WindowsNativeSpeechProvider : INativeSpeechBridge
{
    private const string ModelTypeName = "Microsoft.Windows.AI.Speech.SpeechRecognitionModel";
    private const string ProviderTypeName = "Microsoft.Windows.AI.Speech.SpeechAudioProvider";
    private const string StreamingTypeName = "Microsoft.Windows.AI.Speech.StreamingRecognition";
    private readonly ConcurrentDictionary<string, SourceStream> _streams = new(StringComparer.Ordinal);
    private SpeechRecognitionModel? _model;
    private bool _disposed;

    public event Action<NativeSpeechResult>? ResultReceived;

    public NativeSpeechAvailability CheckAvailability()
    {
        if (!OperatingSystem.IsWindowsVersionAtLeast(10, 0, 26100))
            return new(false, false, "Windows AI Speech requires Windows 11 24H2 or later.");
        if (!HasPackageIdentity())
            return new(false, false, "Windows AI Speech requires package identity.");
        if (!ApiInformation.IsTypePresent(ModelTypeName)
            || !ApiInformation.IsTypePresent(ProviderTypeName)
            || !ApiInformation.IsTypePresent(StreamingTypeName))
            return new(false, false, "Windows AI Speech is not available in this Windows App SDK runtime.");

        try
        {
            if (!GlobalizationPreferences.Languages.Any(language => language.StartsWith("en", StringComparison.OrdinalIgnoreCase)))
                return new(false, false, "Windows Native Speech currently requires an English system language.");
            AIFeatureReadyState state = SpeechRecognitionModel.GetReadyState();
            bool unsupported = state is AIFeatureReadyState.NotSupportedOnCurrentSystem
                or AIFeatureReadyState.NotCompatibleWithSystemHardware
                or AIFeatureReadyState.OSUpdateNeeded
                or AIFeatureReadyState.CapabilityMissing;
            return unsupported
                ? new(false, false, $"Windows AI Speech is unavailable: {state}.")
                : new(true, state == AIFeatureReadyState.Ready, state.ToString());
        }
        catch (Exception error)
        {
            return new(false, false, $"Windows AI Speech readiness check failed: 0x{error.HResult:X8}");
        }
    }

    /// <summary>
    /// Model acquisition is never initiated without explicit caller consent.
    /// On CPU systems EnsureReadyAsync may download the optional Windows model.
    /// </summary>
    public async Task PrepareAsync(Func<CancellationToken, Task<bool>> confirmModelDownload, CancellationToken cancellationToken)
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
        NativeSpeechAvailability availability = CheckAvailability();
        if (!availability.Supported)
            throw new InvalidOperationException(availability.Reason);

        AIFeatureReadyState state = SpeechRecognitionModel.GetReadyState();
        if (state != AIFeatureReadyState.Ready)
        {
            if (!await confirmModelDownload(cancellationToken).ConfigureAwait(false))
                throw new OperationCanceledException("Windows speech model download was declined.", cancellationToken);
            cancellationToken.ThrowIfCancellationRequested();
            await SpeechRecognitionModel.EnsureReadyAsync();
        }

        var result = await SpeechRecognitionModel.TryCreateAsync();
        _model = result.SpeechModel ?? throw new InvalidOperationException(
            $"Windows speech model could not be created: {result.ExtendedError}");
    }

    /// <summary>Starts one provider stream per RimV source. It does not capture audio.</summary>
    public async Task StartSourceAsync(string source, CancellationToken cancellationToken)
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
        SpeechRecognitionModel model = _model
            ?? throw new InvalidOperationException("Windows speech model has not been prepared.");
        cancellationToken.ThrowIfCancellationRequested();
        if (_streams.ContainsKey(source)) return;

        var provider = new SpeechAudioProvider();
        var context = new SourceContext();
        StreamingRecognition? recognition = null;
        TypedEventHandler<StreamingRecognition, StreamingRecognizingEventArgs> recognizing = (_, args) =>
        {
            string text = args.Text?.Trim() ?? "";
            if (text.Length > 0)
                ResultReceived?.Invoke(new NativeSpeechResult(source, text, false, context.LatestInputStartMs, 0));
        };
        TypedEventHandler<StreamingRecognition, StreamingRecognizedEventArgs> recognized = (_, args) =>
        {
            string text = args.Text?.Trim() ?? "";
            if (text.Length > 0)
            ResultReceived?.Invoke(new NativeSpeechResult(source, text, true,
                    (long)args.Offset.TotalMilliseconds, (long)args.Duration.TotalMilliseconds));
        };

        try
        {
            recognition = new StreamingRecognition(AudioConfiguration.ForProvider(provider), model);
            recognition.Recognizing += recognizing;
            recognition.Recognized += recognized;
            await recognition.StartContinuousRecognitionAsync();
            var stream = new SourceStream(provider, recognition, recognizing, recognized, context);
            if (!_streams.TryAdd(source, stream))
                await stream.DisposeAsync().ConfigureAwait(false);
        }
        catch
        {
            if (recognition is not null)
            {
                recognition.Recognizing -= recognizing;
                recognition.Recognized -= recognized;
                recognition.Dispose();
            }
            provider.Dispose();
            throw;
        }
    }

    /// <summary>Pushes signed 16-bit mono PCM, preserving each sample's bits.</summary>
    public void PushPcm(string source, long startMs, ReadOnlySpan<short> samples)
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
        if (samples.IsEmpty || !_streams.TryGetValue(source, out SourceStream? stream)) return;
        stream.Context.SetLatestInputStart(startMs);
        ushort[] payload = new ushort[samples.Length];
        for (int index = 0; index < samples.Length; index++)
            payload[index] = unchecked((ushort)samples[index]);
        stream.Provider.PushData(payload);
    }

    public async Task StopSourceAsync(string source)
    {
        if (_streams.TryRemove(source, out SourceStream? stream))
            await stream.DisposeAsync().ConfigureAwait(false);
    }

    public async ValueTask DisposeAsync()
    {
        if (_disposed) return;
        _disposed = true;
        foreach (string source in _streams.Keys)
            await StopSourceAsync(source).ConfigureAwait(false);
        _model?.Dispose();
        _model = null;
    }

    private static bool HasPackageIdentity()
    {
        var packageName = new StringBuilder(512);
        uint length = (uint)packageName.Capacity;
        return GetCurrentPackageFullName(ref length, packageName) == 0;
    }

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetCurrentPackageFullName(ref uint packageFullNameLength, StringBuilder packageFullName);

    private sealed class SourceStream(
        SpeechAudioProvider provider,
        StreamingRecognition recognition,
        TypedEventHandler<StreamingRecognition, StreamingRecognizingEventArgs> recognizing,
        TypedEventHandler<StreamingRecognition, StreamingRecognizedEventArgs> recognized,
        SourceContext context) : IAsyncDisposable
    {
        internal SpeechAudioProvider Provider { get; } = provider;
        internal SourceContext Context { get; } = context;

        public ValueTask DisposeAsync()
        {
            recognition.Recognizing -= recognizing;
            recognition.Recognized -= recognized;
            try { recognition.StopContinuousRecognition(); }
            finally
            {
                recognition.Dispose();
                provider.Dispose();
            }
            return ValueTask.CompletedTask;
        }
    }

    private sealed class SourceContext
    {
        private long _latestInputStartMs;
        internal long LatestInputStartMs => Interlocked.Read(ref _latestInputStartMs);
        internal void SetLatestInputStart(long value) => Interlocked.Exchange(ref _latestInputStartMs, value);
    }
}
