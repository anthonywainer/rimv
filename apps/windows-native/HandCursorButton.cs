using Microsoft.UI.Input;
using Microsoft.UI.Xaml.Controls;

namespace RimV.Windows;

/// <summary>Button variant used by tray and selector surfaces with a hand pointer.</summary>
public sealed class HandCursorButton : Button
{
    public HandCursorButton() =>
        ProtectedCursor = InputSystemCursor.Create(InputSystemCursorShape.Hand);
}
