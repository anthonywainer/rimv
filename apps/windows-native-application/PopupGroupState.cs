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

public sealed record LanguageOption(string Code, string Name, string Flag);

public static class LanguageSelectorPolicy
{
    private static readonly string[] PopularCodes = ["en", "es", "fr", "de", "pt", "it", "nl", "pl", "ru", "uk"];

    public static IReadOnlyList<string> OrderSupported(IEnumerable<string> supported) =>
        supported.Distinct(StringComparer.OrdinalIgnoreCase)
            .OrderBy(PopularIndex)
            .ThenBy(code => code, StringComparer.OrdinalIgnoreCase)
            .ToArray();

    private static int PopularIndex(string code)
    {
        string language = code.Split('-', 2)[0].ToLowerInvariant();
        int index = Array.IndexOf(PopularCodes, language);
        return index >= 0 ? index : int.MaxValue;
    }

    public static IEnumerable<LanguageOption> Search(IEnumerable<string> supported, string? query)
    {
        string normalized = query?.Trim() ?? "";
        foreach (string code in OrderSupported(supported))
        {
            string name = GetName(code);
            if (normalized.Length == 0 || code.Contains(normalized, StringComparison.CurrentCultureIgnoreCase)
                || name.Contains(normalized, StringComparison.CurrentCultureIgnoreCase))
                yield return new LanguageOption(code, name, GetFlag(code));
        }
    }

    public static string GetName(string code)
    {
        string language = code.Split('-', 2)[0];
        try { return System.Globalization.CultureInfo.GetCultureInfo(language).EnglishName; }
        catch (System.Globalization.CultureNotFoundException) { return code; }
    }

    public static string GetFlag(string code)
    {
        string language = code.Split('-', 2)[0].ToLowerInvariant();
        return language switch
        {
        "en" => "🇺🇸", "es" => "🇪🇸", "fr" => "🇫🇷", "de" => "🇩🇪", "pt" => "🇵🇹",
        "it" => "🇮🇹", "nl" => "🇳🇱", "pl" => "🇵🇱", "ru" => "🇷🇺", "uk" => "🇺🇦",
        "ja" => "🇯🇵", "zh" => "🇨🇳", "ko" => "🇰🇷", "ar" => "🇸🇦", "hi" => "🇮🇳",
        "tr" => "🇹🇷", "sv" => "🇸🇪", "da" => "🇩🇰", "no" => "🇳🇴", "fi" => "🇫🇮",
        "he" => "🇮🇱", "cs" => "🇨🇿", "el" => "🇬🇷", "hu" => "🇭🇺", "ro" => "🇷🇴",
        "th" => "🇹🇭", "vi" => "🇻🇳", "id" => "🇮🇩", "ms" => "🇲🇾",
            _ => "🌐",
        };
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
