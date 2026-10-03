namespace RimV.Windows.Application;

public enum PopupSelectorKind { None, Language, Model, Recordings }

/// <summary>Pure state transitions for the tray menu and its one active selector.</summary>
public sealed class PopupGroupState
{
    public bool IsMenuVisible { get; private set; }
    public PopupSelectorKind ActiveSelector { get; private set; }

    public void ShowMenu() => IsMenuVisible = true;

    public void HideAll()
    {
        IsMenuVisible = false;
        ActiveSelector = PopupSelectorKind.None;
    }

    public bool ToggleMenu()
    {
        if (IsMenuVisible)
        {
            HideAll();
            return false;
        }

        IsMenuVisible = true;
        return true;
    }

    public void OpenSelector(PopupSelectorKind selector)
    {
        if (selector == PopupSelectorKind.None) throw new ArgumentOutOfRangeException(nameof(selector));
        IsMenuVisible = true;
        ActiveSelector = selector;
    }

    public void CloseSelector() => ActiveSelector = PopupSelectorKind.None;

    public bool ShouldDismissForActivation(bool targetInPopupGroup, bool cursorOverTrayIcon) =>
        IsMenuVisible && !targetInPopupGroup && !cursorOverTrayIcon;

    /// <returns>True when Escape dismissed the menu group after no selector was open.</returns>
    public bool Escape()
    {
        if (ActiveSelector != PopupSelectorKind.None)
        {
            CloseSelector();
            return false;
        }

        HideAll();
        return true;
    }
}

public readonly record struct SelectorPopupPlacement(int X, int Y, int Width, int Height, PopupPointerEdge Edge, int PointerOffset);

public static class SelectorPopupPositioner
{
    public static SelectorPopupPlacement Place(PixelRect anchor, PixelRect workArea, int width, int height, int gap, int pointerInset)
    {
        const int margin = 8;
        bool placeRight = workArea.Right - anchor.Right >= width + gap + margin
            || anchor.Left - workArea.Left < width + gap + margin;
        PopupPointerEdge edge = placeRight ? PopupPointerEdge.Left : PopupPointerEdge.Right;
        int x = placeRight ? anchor.Right + gap : anchor.Left - gap - width;
        x = Math.Clamp(x, workArea.Left + margin, Math.Max(workArea.Left + margin, workArea.Right - width - margin));
        int y = anchor.CenterY - height / 2;
        y = Math.Clamp(y, workArea.Top + margin, Math.Max(workArea.Top + margin, workArea.Bottom - height - margin));
        int pointerOffset = Math.Clamp(anchor.CenterY - y, pointerInset, Math.Max(pointerInset, height - pointerInset));
        return new SelectorPopupPlacement(x, y, width, height, edge, pointerOffset);
    }
}

public sealed record LanguageOption(
    string Code, string Name, string Locale, string CanonicalLocale,
    string? RegionName, string? RegionCode, string Flag, IReadOnlyList<string> SearchTerms);

public sealed record LanguageSelectorSections(
    IReadOnlyList<LanguageOption> Popular, IReadOnlyList<LanguageOption> Remaining, bool IsSearching)
{
    public bool HasRemaining => Remaining.Count > 0;
}

public static class LanguageSelectorPolicy
{
    private static readonly string[] PopularLanguages = ["en", "es", "fr", "de", "it", "pt", "ja", "zh", "ko"];

    public static LanguageSelectorSections Build(IEnumerable<LanguagePresentation> supported,
        string? query, bool supportsAutoDetect, bool showAll, string? selectedCode)
    {
        string normalized = query?.Trim() ?? "";
        bool searching = normalized.Length > 0;
        List<LanguageOption> all = supported
            .Where(item => !string.IsNullOrWhiteSpace(item.Id) && !item.Id.Equals("auto", StringComparison.OrdinalIgnoreCase))
            .GroupBy(item => item.Id, StringComparer.OrdinalIgnoreCase)
            .Select(group => group.First())
            .Select(ToOption)
            .OrderBy(item => item.Name, StringComparer.OrdinalIgnoreCase)
            .ThenBy(item => item.RegionName, StringComparer.OrdinalIgnoreCase)
            .ThenBy(item => item.Code, StringComparer.OrdinalIgnoreCase)
            .ToList();

        if (searching)
        {
            all = all.Where(item => item.Code.Contains(normalized, StringComparison.OrdinalIgnoreCase)
                || item.Name.Contains(normalized, StringComparison.OrdinalIgnoreCase)
                || (item.RegionName?.Contains(normalized, StringComparison.OrdinalIgnoreCase) ?? false)
                || item.SearchTerms.Any(term => term.Contains(normalized, StringComparison.OrdinalIgnoreCase))).ToList();
            if (supportsAutoDetect && "auto detect".Contains(normalized, StringComparison.OrdinalIgnoreCase))
                return new([AutoDetect], all, true);
            return new(all, [], true);
        }

        List<LanguageOption> popular = [];
        if (supportsAutoDetect) popular.Add(AutoDetect);
        List<LanguageOption> remaining = [.. all];
        foreach (string language in PopularLanguages)
        {
            LanguageOption[] candidates = all.Where(item => BaseLanguage(item.Code) == language).ToArray();
            if (candidates.Length == 0) continue;
            LanguageOption preferred = candidates.FirstOrDefault(item =>
                    item.Code.Equals(selectedCode, StringComparison.OrdinalIgnoreCase))
                ?? candidates.FirstOrDefault(item => item.Locale.Equals(item.CanonicalLocale, StringComparison.OrdinalIgnoreCase))
                ?? candidates[0];
            popular.Add(preferred);
            remaining.RemoveAll(item => item.Code.Equals(preferred.Code, StringComparison.OrdinalIgnoreCase));
        }
        return new(popular, remaining, false);
    }

    private static LanguageOption ToOption(LanguagePresentation item) => new(item.Id, item.LanguageName,
        item.Locale, item.CanonicalLocale, item.RegionName, item.RegionCode, item.Flag, item.SearchTerms);
    private static string BaseLanguage(string code) => code.Split('-', 2)[0].ToLowerInvariant();
    private static LanguageOption AutoDetect { get; } = new("auto", "Auto Detect", "", "", null, null, "✨", ["auto", "auto detect"]);
}

public static class ModelSelectorPolicy
{
    /// <summary>Only the shared core's Ready state is selectable in the compact selector.</summary>
    public static IReadOnlyList<ModelRecord> InstalledReady(IEnumerable<ModelRecord> models) =>
        models.Where(model => model.State == "installed" && model.Descriptor.Backend != "vad").ToArray();
}

public static class PopupSizingPolicy
{
    public readonly record struct Result(int Height, bool RequiresScrolling);

    public static Result FitContent(int desiredHeight, int availableHeight, int minimumHeight, int maximumHeight)
    {
        int maximum = Math.Max(1, Math.Min(availableHeight, maximumHeight));
        int minimum = Math.Clamp(minimumHeight, 1, maximum);
        int height = Math.Clamp(Math.Max(1, desiredHeight), minimum, maximum);
        return new Result(height, desiredHeight > height);
    }
}

public static class MenuInteractionPolicy
{
    public static bool ShouldQuit(bool isQ, bool controlDown, bool textInputFocused) =>
        isQ && controlDown && !textInputFocused;
}

public static class MenuContrastPolicy
{
    public static double ContrastRatio(uint foregroundRgb, uint backgroundRgb)
    {
        static double Linear(byte component)
        {
            double value = component / 255d;
            return value <= 0.04045 ? value / 12.92 : Math.Pow((value + 0.055) / 1.055, 2.4);
        }

        static double Luminance(uint color) =>
            0.2126 * Linear((byte)(color >> 16)) +
            0.7152 * Linear((byte)(color >> 8)) +
            0.0722 * Linear((byte)color);

        double first = Luminance(foregroundRgb);
        double second = Luminance(backgroundRgb);
        double lighter = Math.Max(first, second);
        double darker = Math.Min(first, second);
        return (lighter + 0.05) / (darker + 0.05);
    }
}

public enum RecordingFilter { All, Recording, Completed }

public static class RecordingSelectorPolicy
{
    public static IEnumerable<T> Filter<T>(IEnumerable<T> source, string? query, RecordingFilter filter,
        Func<T, string> title, Func<T, string> id, Func<T, string> state)
    {
        string normalized = query?.Trim() ?? "";
        return source.Where(item => filter switch
            {
                RecordingFilter.Recording => state(item) == "recording",
                RecordingFilter.Completed => state(item) == "completed",
                _ => true,
            })
            .Where(item => normalized.Length == 0
                || title(item).Contains(normalized, StringComparison.CurrentCultureIgnoreCase)
                || id(item).Contains(normalized, StringComparison.OrdinalIgnoreCase))
            .OrderBy(item => state(item) == "recording" ? 0 : 1);
    }
}
