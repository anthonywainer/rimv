using RimV.Windows.Application;

namespace RimV.Windows.UnitTests;

public sealed class PopupAndSelectorPolicyTests
{
    [Fact]
    public void EscapeClosesSelectorBeforeTrayMenu()
    {
        var state = new PopupGroupState();
        state.ShowMenu();
        state.OpenSelector(PopupSelectorKind.Language);

        Assert.False(state.Escape());
        Assert.True(state.IsMenuVisible);
        Assert.Equal(PopupSelectorKind.None, state.ActiveSelector);

        Assert.True(state.Escape());
        Assert.False(state.IsMenuVisible);
    }

    [Fact]
    public void SelectorSwitchAndTrayToggleOperateOnOnePopupGroup()
    {
        var state = new PopupGroupState();
        state.ShowMenu();
        state.OpenSelector(PopupSelectorKind.Language);
        state.OpenSelector(PopupSelectorKind.Model);

        Assert.True(state.IsMenuVisible);
        Assert.Equal(PopupSelectorKind.Model, state.ActiveSelector);
        Assert.False(state.ToggleMenu());
        Assert.False(state.IsMenuVisible);
        Assert.Equal(PopupSelectorKind.None, state.ActiveSelector);
        Assert.True(state.ToggleMenu());
    }

    [Fact]
    public void ActivationDismissesOnlyWhenFocusLeavesTheGroupAndTrayIcon()
    {
        var state = new PopupGroupState();
        state.ShowMenu();
        state.OpenSelector(PopupSelectorKind.Language);

        Assert.False(state.ShouldDismissForActivation(targetInPopupGroup: true, cursorOverTrayIcon: false));
        Assert.False(state.ShouldDismissForActivation(targetInPopupGroup: false, cursorOverTrayIcon: true));
        Assert.True(state.ShouldDismissForActivation(targetInPopupGroup: false, cursorOverTrayIcon: false));
    }

    [Theory]
    [InlineData(0, 100, PopupPointerEdge.Left)]
    [InlineData(1900, 100, PopupPointerEdge.Right)]
    public void SelectorPopupFlipsAndStaysInsideTheMonitor(int anchorX, int anchorY, PopupPointerEdge expectedEdge)
    {
        var work = new PixelRect(0, 0, 1920, 1080);
        var anchor = new PixelRect(anchorX, anchorY, anchorX + 40, anchorY + 40);

        SelectorPopupPlacement placement = SelectorPopupPositioner.Place(anchor, work, 350, 440, 4, 30);

        Assert.Equal(expectedEdge, placement.Edge);
        Assert.InRange(placement.X, work.Left + 8, work.Right - placement.Width - 8);
        Assert.InRange(placement.Y, work.Top + 8, work.Bottom - placement.Height - 8);
        Assert.InRange(placement.PointerOffset, 30, placement.Height - 30);
    }

    [Fact]
    public void LanguagesAreFilteredAndSortedFromSharedModelCapabilities()
    {
        string[] supported = ["ja", "es", "en", "fr"];

        Assert.Equal(["en", "es", "fr", "ja"], LanguageSelectorPolicy.OrderSupported(supported));
        Assert.Equal("Spanish", Assert.Single(LanguageSelectorPolicy.Search(supported, "span")).Name);
        Assert.Equal("🇯🇵", LanguageSelectorPolicy.GetFlag("ja"));
    }

    [Fact]
    public void RecordingsFilterByQueryAndStatusWithActiveSessionsFirst()
    {
        RecordingSummary[] source =
        [
            new() { SessionId = "a", Title = "Older", State = "completed", StartedAtUnixMs = 1 },
            new() { SessionId = "b", Title = "Meeting", State = "recording", StartedAtUnixMs = 2 },
            new() { SessionId = "c", Title = "Meeting notes", State = "completed", StartedAtUnixMs = 3 },
        ];

        RecordingSummary[] all = RecordingSelectorPolicy.Filter(source, "meeting", RecordingFilter.All,
            item => item.Title, item => item.SessionId, item => item.State).ToArray();
        RecordingSummary[] completed = RecordingSelectorPolicy.Filter(source, "", RecordingFilter.Completed,
            item => item.Title, item => item.SessionId, item => item.State).ToArray();

        Assert.Equal(["b", "c"], all.Select(item => item.SessionId));
        Assert.Equal(["a", "c"], completed.Select(item => item.SessionId));
    }

    [Fact]
    public async Task ModelAndLanguageSelectionUseSharedCoreCommandsAndCapabilities()
    {
        using var temporary = new TemporaryDirectory();
        var factory = new CoordinatorTestCoreFactory
        {
            Models =
            [
                new ModelRecord
                {
                    State = "installed",
                    Descriptor = new ModelDescriptor
                    {
                        Id = "parakeet",
                        DisplayName = "Parakeet",
                        Languages = ["en", "es"],
                        Capabilities = new ModelCapabilities { SupportsLanguageDetection = true },
                    },
                },
            ],
        };
        var coordinator = new AppCoordinator(temporary.Path, factory,
            new CoordinatorTestPreferences(), new CoordinatorTestDispatcher());
        await coordinator.InitializeAsync();

        await coordinator.SelectModelAsync("parakeet");
        await coordinator.SelectLanguageAsync("es");

        Assert.Equal("parakeet", coordinator.SelectedModelId);
        Assert.Equal("es", coordinator.SelectedLanguage);
        Assert.Contains(factory.Client.Requests, request => request.TryGetProperty("type", out var type) && type.GetString() == "select_model");
        Assert.Contains(factory.Client.Requests, request => request.TryGetProperty("type", out var type) && type.GetString() == "set_language");
        Assert.True(coordinator.Models.Single().Descriptor.Capabilities.SupportsLanguageDetection);
        await coordinator.ShutdownAsync();
    }

    [Fact]
    public async Task DismissingPopupStateDoesNotStopAnActiveSharedCoreSession()
    {
        using var temporary = new TemporaryDirectory();
        var factory = new CoordinatorTestCoreFactory();
        var coordinator = new AppCoordinator(temporary.Path, factory,
            new CoordinatorTestPreferences(), new CoordinatorTestDispatcher());
        await coordinator.InitializeAsync();
        await coordinator.StartOrStopAsync();
        int requestCount = factory.Client.Requests.Count;

        var popups = new PopupGroupState();
        popups.ShowMenu();
        popups.OpenSelector(PopupSelectorKind.Recordings);
        popups.HideAll();

        Assert.Equal("recording", coordinator.Snapshot.Status);
        Assert.Equal(requestCount, factory.Client.Requests.Count);
        await coordinator.ShutdownAsync();
    }

    private sealed class CoordinatorTestCoreFactory : ISharedCoreClientFactory
    {
        public CoordinatorTestCoreClient Client { get; } = new();
        public IReadOnlyList<ModelRecord> Models { get; init; } = [];
        public ISharedCoreClient Create(string recordingsDirectory, string modelsDirectory)
        {
            Client.Models = Models;
            return Client;
        }
    }

    private sealed class CoordinatorTestCoreClient : ISharedCoreClient
    {
        private readonly System.Threading.Channels.Channel<CoreEvent> _events = System.Threading.Channels.Channel.CreateUnbounded<CoreEvent>();
        private static readonly System.Text.Json.JsonSerializerOptions Options = new() { PropertyNamingPolicy = System.Text.Json.JsonNamingPolicy.SnakeCaseLower };
        public CoreSnapshot Snapshot { get; private set; } = new()
        {
            Capabilities = new EngineCapabilities { MicrophoneCapture = true },
        };
        public IReadOnlyList<ModelRecord> Models { get; set; } = [];
        public List<System.Text.Json.JsonElement> Requests { get; } = [];

        public Task<T> RequestAsync<T>(object request, CancellationToken cancellationToken = default)
        {
            using System.Text.Json.JsonDocument document = System.Text.Json.JsonDocument.Parse(System.Text.Json.JsonSerializer.Serialize(request, Options));
            System.Text.Json.JsonElement input = document.RootElement.Clone();
            Requests.Add(input);
            string? type = input.TryGetProperty("type", out System.Text.Json.JsonElement typeValue) ? typeValue.GetString() : null;
            object response;
            if (type == "get_state") response = Snapshot;
            else if (type == "get_models") response = Models.ToList();
            else if (type == "get_recordings") response = new List<RecordingSummary>();
            else if (type == "send")
            {
                string? command = input.GetProperty("command").GetProperty("type").GetString();
                if (command == "start_capture") Snapshot = new CoreSnapshot { Status = "recording", Capabilities = Snapshot.Capabilities };
                response = Snapshot;
            }
            else response = System.Text.Json.JsonDocument.Parse("null").RootElement.Clone();
            return Task.FromResult((T)response);
        }

        public async Task<CoreEvent?> PollEventAsync(CancellationToken cancellationToken) =>
            await _events.Reader.ReadAsync(cancellationToken);
        public void Shutdown() { }
        public void Dispose() { }
    }

    private sealed class CoordinatorTestPreferences : IUserPreferencesStore
    {
        public Task<UserPreferences?> LoadAsync(CancellationToken cancellationToken = default) => Task.FromResult<UserPreferences?>(UserPreferences.Default);
        public Task SaveAsync(UserPreferences preferences, CancellationToken cancellationToken = default) => Task.CompletedTask;
    }

    private sealed class CoordinatorTestDispatcher : IUiDispatcher
    {
        public bool HasThreadAccess => true;
        public bool TryEnqueue(Action action) { action(); return true; }
    }

    private sealed class TemporaryDirectory : IDisposable
    {
        public string Path { get; } = System.IO.Path.Combine(System.IO.Path.GetTempPath(), $"rimv-popup-tests-{Guid.NewGuid():N}");
        public TemporaryDirectory() => Directory.CreateDirectory(Path);
        public void Dispose() => Directory.Delete(Path, recursive: true);
    }
}
