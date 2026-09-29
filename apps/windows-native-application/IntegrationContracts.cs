namespace RimV.Windows.Application;

public interface ISharedCoreClient : IDisposable
{
    Task<T> RequestAsync<T>(object request, CancellationToken cancellationToken = default);
    Task<CoreEvent?> PollEventAsync(CancellationToken cancellationToken);
    void Shutdown();
}

public interface ISharedCoreClientFactory
{
    ISharedCoreClient Create(string recordingsDirectory, string modelsDirectory);
}

public interface IUiDispatcher
{
    bool HasThreadAccess { get; }
    bool TryEnqueue(Action action);
}

public interface IUserPreferencesStore
{
    Task<UserPreferences?> LoadAsync(CancellationToken cancellationToken = default);
    Task SaveAsync(UserPreferences preferences, CancellationToken cancellationToken = default);
}

/// <summary>Receives redacted diagnostic events. Never pass transcript or audio content.</summary>
public interface IAppLog
{
    void Info(string eventName, string? detail = null);
    void Error(string eventName, Exception error, string? detail = null);
}

public sealed class NullAppLog : IAppLog
{
    public static NullAppLog Instance { get; } = new();
    private NullAppLog() { }
    public void Info(string eventName, string? detail = null) { }
    public void Error(string eventName, Exception error, string? detail = null) { }
}

public sealed record UserPreferences(string? SelectedModelId, string? Language, string Source, string Theme)
{
    public string? MicrophoneDeviceId { get; init; }
    public static UserPreferences Default { get; } = new(null, null, "microphone", "system");
}
