using System.Diagnostics;
using System.Text;
using System.Text.Json;

namespace RimV.Windows.Application;

/// <summary>Writes newline-delimited, redacted diagnostics with one rotated backup.</summary>
public sealed class FileAppLog(string path) : IAppLog
{
    private const long MaximumBytes = 4 * 1024 * 1024;
    private static readonly object WriteLock = new();
    private readonly string _path = path;

    public void Info(string eventName, string? detail = null) => Write("info", eventName, detail, null);

    public void Error(string eventName, Exception error, string? detail = null) =>
        Write("error", eventName, detail, error);

    private void Write(string level, string eventName, string? detail, Exception? error)
    {
        lock (WriteLock)
        {
            try
            {
                string? directory = Path.GetDirectoryName(_path);
                if (!string.IsNullOrEmpty(directory)) Directory.CreateDirectory(directory);
                if (File.Exists(_path) && new FileInfo(_path).Length >= MaximumBytes)
                {
                    string backup = $"{_path}.1";
                    if (File.Exists(backup)) File.Delete(backup);
                    File.Move(_path, backup);
                }

                var entry = new
                {
                    utc = DateTimeOffset.UtcNow,
                    level,
                    @event = eventName,
                    detail = Redact(detail),
                    error = error is null ? null : Redact(error.ToString()),
                };
                using var stream = new FileStream(_path, FileMode.Append, FileAccess.Write, FileShare.ReadWrite);
                using var writer = new StreamWriter(stream, new UTF8Encoding(encoderShouldEmitUTF8Identifier: false));
                writer.WriteLine(JsonSerializer.Serialize(entry));
            }
            catch (Exception loggingError)
            {
                Trace.WriteLine($"RimV diagnostics write failed: {loggingError.GetType().Name}");
            }
        }
    }

    private static string? Redact(string? value)
    {
        if (value is null) return null;
        string profile = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        if (!string.IsNullOrEmpty(profile)) value = value.Replace(profile, "%USERPROFILE%", StringComparison.OrdinalIgnoreCase);
        string localAppData = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
        if (!string.IsNullOrEmpty(localAppData)) value = value.Replace(localAppData, "%LOCALAPPDATA%", StringComparison.OrdinalIgnoreCase);
        return value;
    }
}
