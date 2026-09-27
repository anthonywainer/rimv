using Microsoft.UI.Dispatching;
using RimV.Windows.Application;

namespace RimV.Windows;

internal sealed class WinUiDispatcher(DispatcherQueue queue) : IUiDispatcher
{
    public bool HasThreadAccess => queue.HasThreadAccess;
    public bool TryEnqueue(Action action) => queue.TryEnqueue(() => action());
}
