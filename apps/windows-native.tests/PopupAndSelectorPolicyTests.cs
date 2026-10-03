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
    public void LanguagePresentationKeepsCanonicalRegionAndSearchMetadata()
    {
        LanguagePresentation english = Presentation("en", "en-US", "en-US", "English", "US", "United States", "🇺🇸");
        LanguagePresentation british = Presentation("en-GB", "en-GB", "en-US", "English", "GB", "United Kingdom", "🇬🇧");
        LanguagePresentation spanish = Presentation("es", "es-ES", "es-ES", "Spanish", "ES", "Spain", "🇪🇸");

        LanguageSelectorSections sections = LanguageSelectorPolicy.Build(
            [english, british, spanish], "united kingdom", true, false, "en-GB");

        Assert.True(sections.IsSearching);
        Assert.Equal("en-GB", Assert.Single(sections.Popular).Code);
        Assert.Equal("English", sections.Popular[0].Name);
        Assert.Equal("United Kingdom", sections.Popular[0].RegionName);
        Assert.Equal("🇬🇧", sections.Popular[0].Flag);
    }

    [Fact]
    public void LanguageSectionsPutUniquePopularItemsBeforeAllRemainingAndKeepOneSelection()
    {
        LanguagePresentation[] supported = [
            Presentation("es", "es-ES", "es-ES", "Spanish", "ES", "Spain", "🇪🇸"),
            Presentation("en-GB", "en-GB", "en-US", "English", "GB", "United Kingdom", "🇬🇧"),
            Presentation("en", "en-US", "en-US", "English", "US", "United States", "🇺🇸"),
            Presentation("fr", "fr-FR", "fr-FR", "French", "FR", "France", "🇫🇷"),
            Presentation("ja", "ja-JP", "ja-JP", "Japanese", "JP", "Japan", "🇯🇵"),
            Presentation("en", "en-US", "en-US", "English", "US", "United States", "🇺🇸"),
        ];

        LanguageSelectorSections compact = LanguageSelectorPolicy.Build(supported, null, true, false, "en-GB");
        LanguageSelectorSections expanded = LanguageSelectorPolicy.Build(supported, null, true, true, "en-GB");
        string[] allIds = expanded.Popular.Concat(expanded.Remaining).Select(option => option.Code).ToArray();

        Assert.Equal(new[] { "auto", "en-GB", "es", "fr", "ja" }, compact.Popular.Select(option => option.Code));
        Assert.Equal(new[] { "auto", "en-GB", "es", "fr", "ja", "en" }, allIds);
        Assert.Equal(allIds.Length, allIds.Distinct(StringComparer.OrdinalIgnoreCase).Count());
        Assert.Equal("en-GB", expanded.Popular[1].Code);
        Assert.Equal("en", expanded.Remaining.Single().Code);
        Assert.Equal("United Kingdom", expanded.Popular[1].RegionName);
        Assert.Equal(1, allIds.Count(code => code.Equals("en-GB", StringComparison.OrdinalIgnoreCase)));
    }

    [Fact]
    public void ProviderSwitchRebuildsLanguageRowsFromTheNewProviderCapabilities()
    {
        LanguageSelectorSections parakeet = LanguageSelectorPolicy.Build([
            Presentation("en", "en-US", "en-US", "English", "US", "United States", "🇺🇸"),
            Presentation("es", "es-ES", "es-ES", "Spanish", "ES", "Spain", "🇪🇸"),
        ], null, true, true, "en");
        LanguageSelectorSections native = LanguageSelectorPolicy.Build([
            Presentation("en-US", "en-US", "en-US", "English", "US", "United States", "🇺🇸"),
            Presentation("en-GB", "en-GB", "en-US", "English", "GB", "United Kingdom", "🇬🇧"),
            Presentation("es-ES", "es-ES", "es-ES", "Spanish", "ES", "Spain", "🇪🇸"),
        ], null, false, true, "en-GB");
        LanguageSelectorSections whisper = LanguageSelectorPolicy.Build([
            Presentation("fr", "fr-FR", "fr-FR", "French", "FR", "France", "🇫🇷"),
            Presentation("de", "de-DE", "de-DE", "German", "DE", "Germany", "🇩🇪"),
            Presentation("en", "en-US", "en-US", "English", "US", "United States", "🇺🇸"),
        ], null, true, true, "auto");

        Assert.Equal(new[] { "auto", "en", "es" }, parakeet.Popular.Select(option => option.Code));
        Assert.DoesNotContain(native.Popular.Concat(native.Remaining), option => option.Code == "en");
        Assert.Contains(native.Popular.Concat(native.Remaining), option => option.Code == "en-GB" && option.Flag == "🇬🇧");
        Assert.Equal(new[] { "auto", "en", "fr", "de" }, whisper.Popular.Select(option => option.Code));
        Assert.Equal(1, whisper.Popular.Count(option => option.Code == "auto"));
    }

    private static LanguagePresentation Presentation(string id, string locale, string canonicalLocale,
        string languageName, string? regionCode, string? regionName, string flag) => new()
    {
        Id = id,
        Locale = locale,
        CanonicalLocale = canonicalLocale,
        LanguageName = languageName,
        RegionCode = regionCode,
        RegionName = regionName,
        Flag = flag,
        SearchTerms = [id, locale, languageName, regionName ?? ""],
    };

    [Fact]
    public void CompactModelSelectorOnlyShowsCoreReadyModelsAndRefreshesAfterInstallation()
    {
        ModelRecord installed = TestModel("installed", "ready");
        ModelRecord downloading = TestModel("downloading", "new");
        ModelRecord incomplete = TestModel("incomplete", "partial");
        ModelRecord vad = new() { State = "installed", Descriptor = new ModelDescriptor { Id = "silero-vad", Backend = "vad" } };
        ModelRecord[] catalog = [installed, downloading, incomplete, vad];

        Assert.Equal("ready", Assert.Single(ModelSelectorPolicy.InstalledReady(catalog)).Descriptor.Id);

        downloading.State = "installed";
        Assert.Equal(["ready", "new"], ModelSelectorPolicy.InstalledReady(catalog).Select(model => model.Descriptor.Id));
        Assert.Empty(ModelSelectorPolicy.InstalledReady([TestModel("available", "missing")]));
    }

    [Fact]
    public void ModelManagerDisablesRemovalForSelectedOrBusyModelsAndReportsProgress()
    {
        ModelRecord selected = TestModel("installed", "selected");
        selected = new ModelRecord { State = selected.State, Selected = true, Descriptor = selected.Descriptor };
        ModelRecord available = TestModel("available", "available");
        ModelRecord installed = TestModel("installed", "installed");

        Assert.False(NativeWindowsPolicy.CanRemoveModel(selected, "idle"));
        Assert.False(NativeWindowsPolicy.CanRemoveModel(installed, "recording"));
        Assert.True(NativeWindowsPolicy.CanRemoveModel(installed, "idle"));
        Assert.False(NativeWindowsPolicy.CanRemoveModel(available, "idle"));
        Assert.Equal(42.5, NativeWindowsPolicy.DownloadProgressPercent(new ModelProgress
        {
            DownloadedBytes = 425,
            TotalBytes = 1000,
        }));
        Assert.Equal(0, NativeWindowsPolicy.DownloadProgressPercent(new ModelProgress()));
    }

    private static ModelRecord TestModel(string state, string id) => new()
    {
        State = state,
        Descriptor = new ModelDescriptor { Id = id, Backend = "parakeet", DisplayName = id },
    };

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
            else if (type == "list_input_devices") response = new List<AudioInputDevice>();
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
