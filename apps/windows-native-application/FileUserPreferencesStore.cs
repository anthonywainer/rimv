using System.Text.Json;

namespace RimV.Windows.Application;

public sealed class FileUserPreferencesStore(string path) : IUserPreferencesStore
{
    public async Task<UserPreferences?> LoadAsync(CancellationToken cancellationToken = default)
    {
        if (!File.Exists(path)) return null;
        await using var stream = File.OpenRead(path);
        return await JsonSerializer.DeserializeAsync<UserPreferences>(stream, cancellationToken: cancellationToken);
    }

    public async Task SaveAsync(UserPreferences preferences, CancellationToken cancellationToken = default)
    {
        string? directory = Path.GetDirectoryName(path);
        if (!string.IsNullOrEmpty(directory)) Directory.CreateDirectory(directory);
        string temporary = path + ".tmp";
        await using (var stream = File.Create(temporary))
            await JsonSerializer.SerializeAsync(stream, preferences, cancellationToken: cancellationToken);
        File.Move(temporary, path, overwrite: true);
    }
}
