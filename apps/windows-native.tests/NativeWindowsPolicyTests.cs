using RimV.Windows.Application;

namespace RimV.Windows.UnitTests;

public sealed class NativeWindowsPolicyTests
{
    [Theory]
    [InlineData(0x0400u, true, true)]
    [InlineData(0x0401u, true, true)]
    [InlineData(0x007Bu, true, true)]
    [InlineData(0x0202u, true, false)]
    [InlineData(0x0205u, true, false)]
    [InlineData(0x0202u, false, true)]
    [InlineData(0x0205u, false, true)]
    public void TraySelectEventsToggleOnceForTheNegotiatedShellVersion(uint action, bool version4, bool expected) =>
        Assert.Equal(expected, NativeWindowsPolicy.IsTrayToggleEvent(action, version4));

    [Theory]
    [InlineData("microphone", true, true, "microphone")]
    [InlineData("system", true, true, "system")]
    [InlineData("both", true, true, "both")]
    [InlineData("system", true, false, "microphone")]
    [InlineData("microphone", false, true, "system")]
    [InlineData("both", false, true, "system")]
    public void ResolveSourceUsesAvailableCaptureCapabilities(
        string preferred,
        bool microphone,
        bool system,
        string expected)
    {
        Assert.Equal(expected, NativeWindowsPolicy.ResolveSource(preferred, microphone, system));
    }

    [Theory]
    [InlineData("idle", "start_capture")]
    [InlineData("starting", "stop_capture")]
    [InlineData("recording", "stop_capture")]
    [InlineData("stopping", null)]
    [InlineData("error", "start_capture")]
    public void ListeningCommandDoesNotRestartWhileStopping(string status, string? expected) =>
        Assert.Equal(expected, NativeWindowsPolicy.ListeningCommand(status));

    [Theory]
    [InlineData("available", true, false)]
    [InlineData("incomplete", true, false)]
    [InlineData("downloading", false, false)]
    [InlineData("installed", false, true)]
    [InlineData("unsupported", false, false)]
    public void ModelActionsMatchCoreReportedState(
        string state,
        bool canInstall,
        bool canSelect)
    {
        Assert.Equal(canInstall, NativeWindowsPolicy.CanInstallModel(state));
        Assert.Equal(state == "downloading", NativeWindowsPolicy.CanCancelModelInstall(state));
        Assert.Equal(canSelect, NativeWindowsPolicy.CanSelectModel(state, "idle"));
    }

    [Theory]
    [InlineData("completed", true)]
    [InlineData("recording", false)]
    [InlineData("starting", false)]
    public void OnlyCompletedRecordingsCanBeDeleted(string state, bool expected) =>
        Assert.Equal(expected, NativeWindowsPolicy.CanDeleteRecording(state));

    [Theory]
    [InlineData(true, true, false, true)]
    [InlineData(true, true, true, false)]
    [InlineData(false, true, false, false)]
    [InlineData(true, false, false, false)]
    public void CtrlQQuitsOnlyOutsideTextEntry(bool isQ, bool controlDown, bool textInputFocused, bool expected) =>
        Assert.Equal(expected, MenuInteractionPolicy.ShouldQuit(isQ, controlDown, textInputFocused));

    [Theory]
    [InlineData(590, 800, 500, 680, 590, false)]
    [InlineData(900, 800, 500, 680, 680, true)]
    [InlineData(590, 420, 500, 680, 420, true)]
    public void PopupUsesContentHeightAndScrollsOnlyWhenConstrained(
        int desired, int available, int minimum, int maximum, int expectedHeight, bool expectedScroll)
    {
        PopupSizingPolicy.Result result = PopupSizingPolicy.FitContent(desired, available, minimum, maximum);

        Assert.Equal(expectedHeight, result.Height);
        Assert.Equal(expectedScroll, result.RequiresScrolling);
    }

    [Fact]
    public void MenuThemeTokensMaintainReadableTextAndExplicitActionForeground()
    {
        string themePath = Path.Combine(AppContext.BaseDirectory, "Themes", "RimV.xaml");
        System.Xml.Linq.XDocument document = System.Xml.Linq.XDocument.Load(themePath);
        System.Xml.Linq.XNamespace x = "http://schemas.microsoft.com/winfx/2006/xaml";

        foreach (string themeName in new[] { "Light", "Dark" })
        {
            System.Xml.Linq.XElement theme = document.Descendants()
                .Single(element => (string?)element.Attribute(x + "Key") == themeName);
            uint Color(string key) => ParseColor((string)theme.Elements()
                .Single(element => (string?)element.Attribute(x + "Key") == key)
                .Attribute("Color")!);

            Assert.True(MenuContrastPolicy.ContrastRatio(Color("RimVMenuTextBrush"), Color("RimVMenuSurfaceBrush")) >= 4.5);
            Assert.True(MenuContrastPolicy.ContrastRatio(Color("RimVMenuSecondaryTextBrush"), Color("RimVMenuSurfaceBrush")) >= 4.5);
            Assert.True(MenuContrastPolicy.ContrastRatio(Color("RimVMenuDisabledTextBrush"), Color("RimVMenuDisabledBackgroundBrush")) >= 4.5);
            Assert.True(MenuContrastPolicy.ContrastRatio(Color("RimVMenuActionForegroundBrush"), Color("RimVMenuAccentBrush")) >= 4.5);
            Assert.True(MenuContrastPolicy.ContrastRatio(Color("RimVMenuSelectionTextBrush"), Color("RimVMenuSelectedBackgroundBrush")) >= 4.5);
        }

        System.Xml.Linq.XElement listenStyle = document.Descendants()
            .Single(element => (string?)element.Attribute(x + "Key") == "RimVMenuListenButtonStyle");
        Assert.Contains(listenStyle.Descendants(), element =>
            element.Name.LocalName == "Setter"
            && (string?)element.Attribute("Property") == "Foreground"
            && ((string?)element.Attribute("Value"))?.Contains("RimVMenuActionForegroundBrush", StringComparison.Ordinal) == true);
        System.Xml.Linq.XDocument shell = System.Xml.Linq.XDocument.Load(
            Path.Combine(AppContext.BaseDirectory, "Themes", "ShellWindow.xaml"));
        foreach (string elementName in new[] { "ListenIcon", "ListenLabel" })
        {
            Assert.Contains(shell.Descendants(), element =>
                (string?)element.Attribute(x + "Name") == elementName
                && ((string?)element.Attribute("Foreground"))?.Contains("RimVMenuActionForegroundBrush", StringComparison.Ordinal) == true);
        }
    }

    private static uint ParseColor(string value) => Convert.ToUInt32(value.TrimStart('#'), 16);
}
