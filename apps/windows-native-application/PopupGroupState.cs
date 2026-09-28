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

public sealed record LanguageOption(string Code, string Name, string? FlagRegion);

public static class LanguageSelectorPolicy
{
    private static readonly string[] PopularCodes = ["en", "es", "fr", "de", "pt", "it", "nl", "pl", "ru", "uk"];
    private static readonly HashSet<string> FlagRegions = new(StringComparer.OrdinalIgnoreCase)
    {
        "bg", "hr", "cz", "dk", "nl", "us", "gb", "mx", "br", "ca", "au", "ee", "fi", "fr", "de",
        "gr", "hu", "it", "lv", "lt", "mt", "pl", "pt", "ro", "sk", "si", "es", "se", "ru", "ua",
        "jp", "cn", "tw", "kr", "sa", "in", "tr", "no", "il", "th", "vn", "id", "my",
    };

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
                yield return new LanguageOption(code, name, GetFlagRegion(code));
        }
    }

    public static string GetName(string code)
    {
        string language = code.Split('-', 2)[0];
        try { return System.Globalization.CultureInfo.GetCultureInfo(language).EnglishName; }
        catch (System.Globalization.CultureNotFoundException) { return code; }
    }

    /// <summary>Maps a language tag to its display flag region; language and country codes are distinct.</summary>
    public static string? GetFlagRegion(string code)
    {
        string[] subtags = code.Split('-', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
        string language = subtags.FirstOrDefault()?.ToLowerInvariant() ?? "";
        if (language == "zh" && subtags.Skip(1).Any(tag => tag.Equals("Hant", StringComparison.OrdinalIgnoreCase)))
            return "tw";
        string? explicitRegion = subtags.Skip(1).FirstOrDefault(tag => tag.Length == 2 && tag.All(char.IsAsciiLetter));
        if (explicitRegion is not null && FlagRegions.Contains(explicitRegion)) return explicitRegion.ToLowerInvariant();

        return language switch
        {
            "bg" => "bg", "hr" => "hr", "cs" => "cz", "da" => "dk", "nl" => "nl",
            "en" => "us", "et" => "ee", "fi" => "fi", "fr" => "fr", "de" => "de",
            "el" => "gr", "hu" => "hu", "it" => "it", "lv" => "lv", "lt" => "lt",
            "mt" => "mt", "pl" => "pl", "pt" => "pt", "ro" => "ro", "sk" => "sk",
            "sl" => "si", "es" => "es", "sv" => "se", "ru" => "ru", "uk" => "ua",
            "ja" => "jp", "zh" => "cn", "ko" => "kr", "ar" => "sa", "hi" => "in",
            "tr" => "tr", "no" => "no", "he" => "il", "th" => "th", "vi" => "vn",
            "id" => "id", "ms" => "my", _ => null,
        };
    }
}

public static class ModelSelectorPolicy
{
    /// <summary>Only the shared core's Ready state is selectable in the compact selector.</summary>
    public static IReadOnlyList<ModelRecord> InstalledReady(IEnumerable<ModelRecord> models) =>
        models.Where(model => model.State == "installed").ToArray();
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
