using RimV.Windows.Application;
using System.Collections.Concurrent;
using System.Diagnostics;
using System.Text;

namespace RimV.Windows;

internal static class WindowsNativeSpeechE2E
{
    private const int SamplesPerChunk = 1_024;
    private static readonly TimeSpan ChunkDuration = TimeSpan.FromMilliseconds(64);

    public static async Task<int> RunAsync()
    {
        if (!OperatingSystem.IsWindowsVersionAtLeast(10, 0, 26100))
            throw new InvalidOperationException("Windows Native Speech E2E requires Windows 11 24H2 or later.");
        string root = FindRepositoryRoot();
        string fixturePath = Path.Combine(root, "resources", "audio", "audio1-16k-mono.wav");
        string referencePath = Path.Combine(root, "resources", "audio", "audio1.txt");
        (short[] samples, int sampleRate, short channels) = ReadPcmWave(fixturePath);
        if (sampleRate != 16_000 || channels != 1)
            throw new InvalidDataException($"Expected 16 kHz mono fixture, received {sampleRate} Hz / {channels} channels.");

        await using var provider = new WindowsNativeSpeechProvider();
        NativeSpeechAvailability availability = provider.CheckAvailability();
        if (!availability.Supported)
            throw new InvalidOperationException($"Native Windows Speech unavailable: {availability.Reason}");
        if (!availability.ModelReady)
            throw new InvalidOperationException("Speech model is not ready. Start Native once in RimV, accept the Windows Update download, then rerun this test.");

        // E2E never downloads a model implicitly. The provider is forced directly;
        // no Parakeet/Whisper selector or fallback is involved.
        await provider.PrepareAsync(_ => Task.FromResult(false), CancellationToken.None);
        var observed = new ConcurrentQueue<(NativeSpeechResult Result, TimeSpan Elapsed)>();
        var clock = Stopwatch.StartNew();
        provider.ResultReceived += result => observed.Enqueue((result, clock.Elapsed));
        await provider.StartSourceAsync("microphone", CancellationToken.None);
        try
        {
            for (int offset = 0; offset < samples.Length; offset += SamplesPerChunk)
            {
                short[] block = samples.AsSpan(offset, Math.Min(SamplesPerChunk, samples.Length - offset)).ToArray();
                provider.PushPcm("microphone", (long)offset * 1_000 / sampleRate, block);
                await Task.Delay(ChunkDuration);
            }
            await Task.Delay(TimeSpan.FromSeconds(8));
        }
        finally
        {
            await provider.StopSourceAsync("microphone");
        }

        var results = observed.ToArray();
        var mapper = new NativeTranscriptMapper();
        var updates = results.Select(item => (Update: mapper.Map(item.Result), item.Elapsed)).ToArray();
        var partials = updates.Where(item => !item.Update.IsFinal && !string.IsNullOrWhiteSpace(item.Update.UnstableText)).ToArray();
        var finals = updates.Where(item => item.Update.IsFinal && !string.IsNullOrWhiteSpace(item.Update.StableText)).ToArray();
        string actual = string.Join(" ", finals.Select(item => item.Update.StableText));
        string expected = File.ReadAllText(referencePath).Trim();
        var accuracy = Score(expected, actual);
        TimeSpan? firstPartial = partials.FirstOrDefault().Elapsed;
        if (partials.Length == 0) firstPartial = null;
        TimeSpan? firstFinal = finals.FirstOrDefault().Elapsed;
        if (finals.Length == 0) firstFinal = null;
        var replacementGroups = updates.Where(item => !item.Update.IsFinal)
            .GroupBy(item => item.Update.UtteranceId)
            .Select(group => new { Id = group.Key, Count = group.Count(), Finalized = finals.Any(final => final.Update.UtteranceId == group.Key) })
            .ToArray();
        int maxPartialsBeforeFinal = replacementGroups.Where(group => group.Finalized).Select(group => group.Count).DefaultIfEmpty().Max();
        bool noDuplicateFinals = finals.Select(item => Normalize(item.Update.StableText)).Distinct(StringComparer.Ordinal).Count() == finals.Length;
        bool duplicateFinalText = !noDuplicateFinals;
        string report = string.Join(Environment.NewLine, new[] {
            "Provider: native_windows (Microsoft.Windows.AI.Speech; forced, no fallback)",
            $"Fixture format: {sampleRate} Hz, {channels} channel, PCM 16-bit",
            $"Partial count: {partials.Length}",
            $"Final count: {finals.Length}",
            $"Time to first partial: {Format(firstPartial)}",
            $"Time to final: {Format(firstFinal)}",
            $"Partial revisions before final: {maxPartialsBeforeFinal}",
            $"Expected: {expected}",
            $"Actual: {actual}",
            $"WER: {accuracy.Wer:F3}; CER: {accuracy.Cer:F3}",
            "Duplicate final text: " + duplicateFinalText,
        ]);
        bool partialPrecedesFinal = firstPartial is { } partialTime
            && firstFinal is { } finalTime
            && partialTime < finalTime;
        bool pass = partials.Length > 0 && finals.Length > 0 && maxPartialsBeforeFinal >= 2
            && partialPrecedesFinal && actual.Length > 0 && noDuplicateFinals
            && Normalize(actual).Contains("travel plans") && Normalize(actual).Contains("san francisco");
        report += Environment.NewLine + (pass ? "Result: PASS" : "Result: FAIL (streaming, replacement, duplicate, or fixture accuracy requirements were not met)");
        Console.WriteLine(report);
        string reportPath = Path.Combine(root, "target", "windows-native-speech-e2e.txt");
        Directory.CreateDirectory(Path.GetDirectoryName(reportPath)!);
        File.WriteAllText(reportPath, report + Environment.NewLine);
        return pass ? 0 : 1;
    }

    private static string FindRepositoryRoot()
    {
        string? configured = Environment.GetEnvironmentVariable("RIMV_REPO_ROOT");
        IEnumerable<string> starts = new[] { configured, Environment.CurrentDirectory, AppContext.BaseDirectory }
            .OfType<string>();
        foreach (string start in starts)
        {
            DirectoryInfo? directory = new(Path.GetFullPath(start));
            while (directory is not null)
            {
                if (File.Exists(Path.Combine(directory.FullName, "resources", "audio", "audio1-16k-mono.wav")))
                    return directory.FullName;
                directory = directory.Parent;
            }
        }
        throw new FileNotFoundException("Set RIMV_REPO_ROOT or run from a RimV checkout containing the existing audio fixture.");
    }

    private static (short[] Samples, int SampleRate, short Channels) ReadPcmWave(string path)
    {
        using var reader = new BinaryReader(File.OpenRead(path));
        if (Encoding.ASCII.GetString(reader.ReadBytes(4)) != "RIFF") throw new InvalidDataException("Fixture is not RIFF/WAVE.");
        _ = reader.ReadInt32();
        if (Encoding.ASCII.GetString(reader.ReadBytes(4)) != "WAVE") throw new InvalidDataException("Fixture is not RIFF/WAVE.");
        short format = 0, channels = 0, bitsPerSample = 0;
        int sampleRate = 0;
        byte[]? data = null;
        while (reader.BaseStream.Position + 8 <= reader.BaseStream.Length)
        {
            string chunk = Encoding.ASCII.GetString(reader.ReadBytes(4));
            int length = reader.ReadInt32();
            if (length < 0 || reader.BaseStream.Position + length > reader.BaseStream.Length)
                throw new InvalidDataException("Fixture contains an invalid WAV chunk.");
            if (chunk == "fmt ")
            {
                format = reader.ReadInt16();
                channels = reader.ReadInt16();
                sampleRate = reader.ReadInt32();
                _ = reader.ReadInt32();
                _ = reader.ReadInt16();
                bitsPerSample = reader.ReadInt16();
                reader.BaseStream.Position += length - 16;
            }
            else if (chunk == "data") data = reader.ReadBytes(length);
            else reader.BaseStream.Position += length;
            if ((length & 1) != 0) reader.BaseStream.Position++;
        }
        if (format != 1 || bitsPerSample != 16 || data is null)
            throw new InvalidDataException($"Expected signed PCM16 WAV (format={format}, bits={bitsPerSample}).");
        short[] samples = new short[data.Length / sizeof(short)];
        Buffer.BlockCopy(data, 0, samples, 0, data.Length);
        return (samples, sampleRate, channels);
    }

    private static (double Wer, double Cer) Score(string expected, string actual)
    {
        string[] referenceWords = Normalize(expected).Split(' ', StringSplitOptions.RemoveEmptyEntries);
        string[] actualWords = Normalize(actual).Split(' ', StringSplitOptions.RemoveEmptyEntries);
        string referenceChars = string.Concat(referenceWords);
        string actualChars = string.Concat(actualWords);
        return (Distance(referenceWords, actualWords) / (double)Math.Max(1, referenceWords.Length),
            Distance(referenceChars.Select(character => character.ToString()).ToArray(),
                actualChars.Select(character => character.ToString()).ToArray()) / (double)Math.Max(1, referenceChars.Length));
    }

    private static int Distance(IReadOnlyList<string> left, IReadOnlyList<string> right)
    {
        int[] prior = Enumerable.Range(0, right.Count + 1).ToArray();
        for (int row = 1; row <= left.Count; row++)
        {
            int[] next = new int[right.Count + 1];
            next[0] = row;
            for (int column = 1; column <= right.Count; column++)
                next[column] = Math.Min(Math.Min(next[column - 1] + 1, prior[column] + 1), prior[column - 1] + (left[row - 1] == right[column - 1] ? 0 : 1));
            prior = next;
        }
        return prior[right.Count];
    }

    private static string Normalize(string text)
    {
        var normalized = new StringBuilder(text.Length);
        bool pendingSpace = false;
        foreach (char character in text.ToLowerInvariant())
        {
            if (char.IsLetterOrDigit(character))
            {
                if (pendingSpace && normalized.Length > 0) normalized.Append(' ');
                normalized.Append(character);
                pendingSpace = false;
            }
            else pendingSpace = true;
        }
        return normalized.ToString();
    }
    private static string Format(TimeSpan? elapsed) => elapsed?.ToString(@"hh\:mm\:ss\.fff") ?? "none";
}
