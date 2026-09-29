using System.Xml.Linq;

namespace RimV.Windows.UnitTests;

public sealed class TranscriptionPresentationTests
{
    [Fact]
    public void TranscriptRowsHaveNoSelectionCheckboxesAndRenderLiveHypothesesSeparately()
    {
        XDocument document = LoadViewer();
        XNamespace x = "http://schemas.microsoft.com/winfx/2006/xaml";

        XElement transcriptList = document.Descendants()
            .Single(element => (string?)element.Attribute(x + "Name") == "TranscriptList");
        Assert.Equal("None", (string?)transcriptList.Attribute("SelectionMode"));

        XElement[] runs = transcriptList.Descendants()
            .Where(element => element.Name.LocalName == "Run")
            .ToArray();
        Assert.Contains(runs, element => (string?)element.Attribute("Text") == "{x:Bind StableText}");
        Assert.Contains(runs, element =>
            (string?)element.Attribute("Text") == "{x:Bind UnstableText}"
            && ((string?)element.Attribute("Foreground"))?.Contains("RimVPartialTranscriptBrush", StringComparison.Ordinal) == true
            && (string?)element.Attribute("FontStyle") == "Italic");
    }

    [Fact]
    public void SavedAudioUsesResponsiveNativeTransportControls()
    {
        XDocument document = LoadViewer();
        XNamespace x = "http://schemas.microsoft.com/winfx/2006/xaml";
        XElement player = document.Descendants()
            .Single(element => (string?)element.Attribute(x + "Name") == "AudioPlayer");

        Assert.Equal("True", (string?)player.Attribute("AreTransportControlsEnabled"));
        Assert.Equal("Stretch", (string?)player.Attribute("HorizontalAlignment"));
        Assert.NotNull(player.Attribute("MinHeight"));
    }

    private static XDocument LoadViewer() => XDocument.Load(
        Path.Combine(AppContext.BaseDirectory, "Themes", "TranscriptionWindow.xaml"));
}
