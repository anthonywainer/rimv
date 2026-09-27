using System.Text.Json;
using System.Threading.Channels;
using RimV.Windows.Application;

namespace RimV.Windows.UnitTests;

public sealed class AppCoordinatorTests
{
    [Fact]
    public async Task FilePreferencesStorePersistsOnlyNativePresentationSelections()
    {
        using var temporary = new TemporaryDirectory();
        string file = System.IO.Path.Combine(temporary.Path, "preferences.json");
        var store = new FileUserPreferencesStore(file);
        var expected = new UserPreferences("parakeet", "en", "both", "dark");

        await store.SaveAsync(expected);

        Assert.Equal(expected, await store.LoadAsync());
    }

    [Fact]
    public async Task ListeningCommandsUseCoreStateAndShutdownJoinsEventPolling()
    {
        using var temporary = new TemporaryDirectory();
        var factory = new FakeCoreFactory();
        var coordinator = CreateCoordinator(temporary.Path, factory, UserPreferences.Default);
        await coordinator.InitializeAsync();

        await coordinator.StartOrStopAsync();
        Assert.Equal("recording", coordinator.Snapshot.Status);
        Assert.Contains(factory.Client.Requests, request => Command(request) == "start_capture");

        await coordinator.StartOrStopAsync();
        Assert.Equal("idle", coordinator.Snapshot.Status);
        Assert.Contains(factory.Client.Requests, request => Command(request) == "stop_capture");

        await coordinator.ShutdownAsync();
        Assert.True(factory.Client.ShutdownCalled);
    }

    [Fact]
    public async Task StartupDropsSavedLanguageNotSupportedByInstalledModel()
    {
        using var temporary = new TemporaryDirectory();
        var factory = new FakeCoreFactory
        {
            Models =
            [
                new ModelRecord
                {
                    State = "installed",
                    Descriptor = new ModelDescriptor
                    {
                        Id = "model-en",
                        Backend = "parakeet",
                        DisplayName = "English model",
                        Languages = ["en"],
                    },
                },
            ],
        };
        var preferences = new MemoryPreferences(new UserPreferences("model-en", "es", "microphone", "system"));
        var coordinator = CreateCoordinator(temporary.Path, factory, preferences.Value, preferences);
        await coordinator.InitializeAsync();

        JsonElement selected = factory.Client.Requests.Single(request => Type(request) == "select_model");
        Assert.Equal(JsonValueKind.Null, selected.GetProperty("language").ValueKind);
        Assert.Null(coordinator.SelectedLanguage);

        await coordinator.ShutdownAsync();
    }

    [Fact]
    public async Task SystemAudioRequestIsRejectedWhenCoreReportsNoLoopbackCapability()
    {
        using var temporary = new TemporaryDirectory();
        var factory = new FakeCoreFactory
        {
            Snapshot = new CoreSnapshot
            {
                Capabilities = new EngineCapabilities { MicrophoneCapture = true, SystemAudioCapture = false },
            },
        };
        var coordinator = CreateCoordinator(temporary.Path, factory, UserPreferences.Default);
        await coordinator.InitializeAsync();
        int priorCommands = factory.Client.Requests.Count(request => Type(request) == "send");

        await coordinator.SetSourceAsync("system");

        Assert.Equal(priorCommands, factory.Client.Requests.Count(request => Type(request) == "send"));
        Assert.Contains("isn't available", coordinator.ErrorMessage, StringComparison.Ordinal);
        await coordinator.ShutdownAsync();
    }

    [Fact]
    public async Task LiveTranscriptEventsReachPresentationSubscribersExactlyOnce()
    {
        using var temporary = new TemporaryDirectory();
        var factory = new FakeCoreFactory();
        var coordinator = CreateCoordinator(temporary.Path, factory, UserPreferences.Default);
        await coordinator.InitializeAsync();
        var observed = new TaskCompletionSource<CoreEvent>(TaskCreationOptions.RunContinuationsAsynchronously);
        coordinator.CoreEventReceived += item => observed.TrySetResult(item);
        using JsonDocument payload = JsonDocument.Parse("{}");
        await factory.Client.PublishAsync(new CoreEvent("transcript_update", payload.RootElement.Clone()));

        CoreEvent received = await observed.Task.WaitAsync(TimeSpan.FromSeconds(2));

        Assert.Equal("transcript_update", received.Type);
        Assert.Equal(1, coordinator.TranscriptRevision);
        await coordinator.ShutdownAsync();
    }

    private static AppCoordinator CreateCoordinator(
        string dataDirectory,
        FakeCoreFactory factory,
        UserPreferences preferences,
        IUserPreferencesStore? store = null) =>
        new(dataDirectory, factory, store ?? new MemoryPreferences(preferences), new ImmediateDispatcher());

    private static string? Type(JsonElement request) => request.TryGetProperty("type", out JsonElement type) ? type.GetString() : null;
    private static string? Command(JsonElement request) =>
        request.TryGetProperty("command", out JsonElement command) && command.TryGetProperty("type", out JsonElement type)
            ? type.GetString()
            : null;

    private sealed class FakeCoreFactory : ISharedCoreClientFactory
    {
        public CoreSnapshot Snapshot { get; init; } = new()
        {
            Capabilities = new EngineCapabilities { MicrophoneCapture = true, SystemAudioCapture = true },
        };
        public IReadOnlyList<ModelRecord> Models { get; init; } = [];
        public FakeCoreClient Client { get; } = new();

        public ISharedCoreClient Create(string recordingsDirectory, string modelsDirectory)
        {
            Client.Snapshot = Snapshot;
            Client.Models = Models;
            return Client;
        }
    }

    private sealed class FakeCoreClient : ISharedCoreClient
    {
        private static readonly JsonSerializerOptions SerializerOptions = new() { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };
        private readonly Channel<CoreEvent> _events = Channel.CreateUnbounded<CoreEvent>();
        public CoreSnapshot Snapshot { get; set; } = new();
        public IReadOnlyList<ModelRecord> Models { get; set; } = [];
        public List<JsonElement> Requests { get; } = [];
        public bool ShutdownCalled { get; private set; }

        public Task<T> RequestAsync<T>(object request, CancellationToken cancellationToken = default)
        {
            using JsonDocument document = JsonDocument.Parse(JsonSerializer.Serialize(request, SerializerOptions));
            JsonElement input = document.RootElement.Clone();
            Requests.Add(input);
            string? operation = Type(input);
            object response = operation switch
            {
                "get_state" => Snapshot,
                "get_models" => Models.ToList(),
                "get_recordings" => new List<RecordingSummary>(),
                "send" => ApplyCommand(input),
                _ => JsonDocument.Parse("null").RootElement.Clone(),
            };
            return Task.FromResult((T)response);
        }

        private CoreSnapshot ApplyCommand(JsonElement input)
        {
            string? command = Command(input);
            if (command == "start_capture") Snapshot = WithStatus("recording");
            else if (command == "stop_capture") Snapshot = WithStatus("idle");
            return Snapshot;
        }

        private CoreSnapshot WithStatus(string status) => new() { Status = status, Capabilities = Snapshot.Capabilities };

        public async Task<CoreEvent?> PollEventAsync(CancellationToken cancellationToken)
        {
            return await _events.Reader.ReadAsync(cancellationToken);
        }

        public ValueTask PublishAsync(CoreEvent item) => _events.Writer.WriteAsync(item);

        public void Shutdown() => ShutdownCalled = true;
        public void Dispose() { }
    }

    private sealed class MemoryPreferences(UserPreferences? value) : IUserPreferencesStore
    {
        public UserPreferences? Value { get; private set; } = value;
        public Task<UserPreferences?> LoadAsync(CancellationToken cancellationToken = default) => Task.FromResult(Value);
        public Task SaveAsync(UserPreferences preferences, CancellationToken cancellationToken = default)
        {
            Value = preferences;
            return Task.CompletedTask;
        }
    }

    private sealed class ImmediateDispatcher : IUiDispatcher
    {
        public bool HasThreadAccess => true;
        public bool TryEnqueue(Action action) { action(); return true; }
    }

    private sealed class TemporaryDirectory : IDisposable
    {
        public string Path { get; } = System.IO.Path.Combine(System.IO.Path.GetTempPath(), $"rimv-app-tests-{Guid.NewGuid():N}");
        public TemporaryDirectory() => Directory.CreateDirectory(Path);
        public void Dispose() => Directory.Delete(Path, recursive: true);
    }
}
