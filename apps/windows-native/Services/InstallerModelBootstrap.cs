using RimV.Windows.Application;
using System.Diagnostics;
using System.Text.Json;

namespace RimV.Windows;

/// <summary>
/// Runs a one-shot model install through the same FFI and Rust ModelManager
/// used by the app's Model Manager. The NSIS installer never downloads model
/// URLs or writes model files itself.
/// </summary>
internal static class InstallerModelBootstrap
{
    public static async Task<int> RunAsync(string modelId, string dataDirectory, IAppLog log)
    {
        if (modelId is not ("parakeet-tdt-0.6b-v3-int8" or "whisper-base"))
        {
            log.Error("installer.model_rejected", new ArgumentException("Unsupported installer model id."));
            return 1;
        }
        string logDirectory = Path.Combine(dataDirectory, "logs");
        string statusPath = Path.Combine(logDirectory, $"installer-model-{modelId}.json");
        var timer = Stopwatch.StartNew();
        ISharedCoreClient? client = null;
        string currentModelId = modelId;
        try
        {
            Directory.CreateDirectory(dataDirectory);
            Directory.CreateDirectory(Path.Combine(dataDirectory, "recordings"));
            Directory.CreateDirectory(Path.Combine(dataDirectory, "models"));
            Directory.CreateDirectory(logDirectory);
            client = new CoreClientFactory(log).Create(
                Path.Combine(dataDirectory, "recordings"),
                Path.Combine(dataDirectory, "models"));
            // Local ASR capture requires the shared Silero VAD model. Install
            // it through the same catalog as a prerequisite so a selected
            // engine works immediately after an online setup.
            foreach (string requiredModelId in new[] { modelId, "silero-vad" })
            {
                currentModelId = requiredModelId;
                ModelInstallResult result = await InstallOneAsync(client, requiredModelId, timer);
                if (result.Phase != "complete")
                {
                    await File.WriteAllTextAsync(statusPath, JsonSerializer.Serialize(new
                    {
                        modelId,
                        failedModelId = requiredModelId,
                        phase = result.Phase,
                        error = result.Error,
                    }));
                    log.Info("installer.model_finished", $"model={modelId}; failed_dependency={requiredModelId}; phase={result.Phase}");
                    return 1;
                }
            }

            await File.WriteAllTextAsync(statusPath, JsonSerializer.Serialize(new { modelId, phase = "complete" }));
            log.Info("installer.model_finished", $"model={modelId}; phase=complete");
            return 0;
        }
        catch (Exception error)
        {
            try
            {
                Directory.CreateDirectory(logDirectory);
                await File.WriteAllTextAsync(statusPath, JsonSerializer.Serialize(new
                {
                    modelId,
                    failedModelId = currentModelId,
                    phase = "failed",
                    error = error.Message,
                }));
            }
            catch (Exception statusError) { log.Error("installer.model_status_write_failed", statusError); }
            log.Error("installer.model_failed", error, $"model={modelId}; failed_dependency={currentModelId}");
            return 1;
        }
        finally
        {
            if (client is not null)
            {
                try { client.Shutdown(); }
                catch (Exception shutdownError) { log.Error("installer.model_engine_shutdown_failed", shutdownError); }
                client.Dispose();
            }
        }
    }

    private static async Task<ModelInstallResult> InstallOneAsync(
        ISharedCoreClient client,
        string modelId,
        Stopwatch timer)
    {
        await client.RequestAsync<JsonElement>(new { type = "install_model", model_id = modelId });
        while (timer.Elapsed < TimeSpan.FromHours(6))
        {
            CoreEvent? item = await client.PollEventAsync(CancellationToken.None);
            if (item?.Type != "model_progress") continue;
            JsonElement progress = item.Payload.GetProperty("progress");
            if (!string.Equals(progress.GetProperty("model_id").GetString(), modelId, StringComparison.Ordinal))
                continue;

            string phase = progress.GetProperty("phase").GetString() ?? "";
            if (phase is not ("complete" or "failed" or "cancelled")) continue;
            string? error = progress.TryGetProperty("error", out JsonElement errorElement)
                && errorElement.ValueKind != JsonValueKind.Null
                ? errorElement.GetString()
                : null;
            return new ModelInstallResult(phase, error);
        }

        throw new TimeoutException($"The {modelId} installation exceeded six hours.");
    }

    private sealed record ModelInstallResult(string Phase, string? Error);
}
