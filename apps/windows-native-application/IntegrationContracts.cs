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

public sealed record UserPreferences(string? SelectedModelId, string? Language, string Source, string Theme)
{
    public static UserPreferences Default { get; } = new(null, null, "microphone", "system");
}
