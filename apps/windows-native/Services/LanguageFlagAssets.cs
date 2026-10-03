using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using System.Collections.Concurrent;

namespace RimV.Windows;

internal static class LanguageFlagAssets
{
    private static readonly ConcurrentDictionary<string, SvgImageSource> Cache = new(StringComparer.OrdinalIgnoreCase);

    internal static SvgImageSource? ForRegion(string? regionCode)
    {
        if (regionCode is not { Length: 2 }
            || !regionCode.All(char.IsAsciiLetter))
            return null;

        string code = regionCode.ToLowerInvariant();
        string path = Path.Combine(AppContext.BaseDirectory, "Assets", "Flags", $"{code}.svg");
        if (!File.Exists(path)) return null;
        return Cache.GetOrAdd(code, _ => new SvgImageSource(new Uri(path, UriKind.Absolute)));
    }
}
