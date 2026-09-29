using Microsoft.UI.Input;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;

namespace RimV.Windows;

/// <summary>Source tile using WinUI's hand pointer while retaining ToggleButton semantics.</summary>
public sealed class HandCursorToggleButton : ToggleButton
{
    public HandCursorToggleButton()
    {
        ProtectedCursor = InputSystemCursor.Create(InputSystemCursorShape.Hand);
        PointerEntered += (_, _) => SetPointerState(pressed: false);
        PointerExited += (_, _) => RestoreBaseState();
        PointerPressed += (_, _) => SetPointerState(pressed: true);
        PointerReleased += (_, _) => RestoreBaseState();
        GotFocus += (_, _) => BorderBrush = Brush("RimVMenuAccentBrush");
        LostFocus += (_, _) => RestoreBaseState();
        IsEnabledChanged += (_, _) => RestoreBaseState();
    }

    private void SetPointerState(bool pressed)
    {
        if (!IsEnabled) return;
        Background = Brush(pressed ? "RimVMenuPressedBackgroundBrush" : "RimVMenuHoverBackgroundBrush");
        BorderBrush = Brush("RimVMenuAccentBrush");
        Foreground = pressed || IsChecked == true
            ? Brush("RimVMenuActionForegroundBrush")
            : Brush("RimVMenuTextBrush");
        SetContentForeground(Foreground);
    }

    private void RestoreBaseState()
    {
        Background = Brush(IsEnabled && IsChecked == true
            ? "RimVMenuSelectedBackgroundBrush"
            : IsEnabled ? "RimVMenuSurfaceBrush" : "RimVMenuDisabledBackgroundBrush");
        BorderBrush = Brush(IsChecked == true ? "RimVMenuAccentBrush" : "RimVMenuBorderBrush");
        Foreground = Brush(!IsEnabled
            ? "RimVMenuDisabledTextBrush"
            : IsChecked == true ? "RimVMenuSelectionTextBrush" : "RimVMenuTextBrush");
        SetContentForeground(Foreground);
    }

    private void SetContentForeground(Microsoft.UI.Xaml.Media.Brush foreground)
    {
        if (Content is not StackPanel panel) return;
        foreach (var child in panel.Children)
        {
            if (child is TextBlock text) text.Foreground = foreground;
            else if (child is FontIcon icon) icon.Foreground = foreground;
        }
    }

    private static Microsoft.UI.Xaml.Media.Brush Brush(string resource) =>
        (Microsoft.UI.Xaml.Media.Brush)Microsoft.UI.Xaml.Application.Current.Resources[resource];
}
