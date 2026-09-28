namespace RimV.Windows.Application;

public readonly record struct PixelRect(int Left, int Top, int Right, int Bottom)
{
    public int Width => Right - Left;
    public int Height => Bottom - Top;
    public int CenterX => Left + Width / 2;
    public int CenterY => Top + Height / 2;
}

public enum PopupPointerEdge { Bottom, Top, Left, Right }

public readonly record struct TrayPopupPlacement(int X, int Y, int Width, int Height, PopupPointerEdge Edge, int PointerOffset);

/// <summary>Positions a tray flyout in physical screen pixels, including its pointer.</summary>
public static class TrayPopupPositioner
{
    public static TrayPopupPlacement Place(PixelRect icon, PixelRect work, int width, int height, int pointerInset)
    {
        const int margin = 8;
        int centerX = icon.CenterX;
        int centerY = icon.CenterY;
        PopupPointerEdge edge;
        int x;
        int y;

        if (centerY >= work.Bottom)
        {
            edge = PopupPointerEdge.Bottom;
            x = centerX - width / 2;
            y = work.Bottom - height;
        }
        else if (centerY <= work.Top)
        {
            edge = PopupPointerEdge.Top;
            x = centerX - width / 2;
            y = work.Top;
        }
        else if (centerX <= work.Left)
        {
            edge = PopupPointerEdge.Left;
            x = work.Left;
            y = centerY - height / 2;
        }
        else if (centerX >= work.Right)
        {
            edge = PopupPointerEdge.Right;
            x = work.Right - width;
            y = centerY - height / 2;
        }
        else
        {
            // Overflow icons may live inside the work area. Prefer a flyout above them.
            edge = centerY >= work.Top + work.Height / 2 ? PopupPointerEdge.Bottom : PopupPointerEdge.Top;
            x = centerX - width / 2;
            y = edge == PopupPointerEdge.Bottom ? icon.Top - height : icon.Bottom;
        }

        x = Math.Clamp(x, work.Left + margin, Math.Max(work.Left + margin, work.Right - width - margin));
        y = Math.Clamp(y, work.Top + margin, Math.Max(work.Top + margin, work.Bottom - height - margin));
        int axis = edge is PopupPointerEdge.Bottom or PopupPointerEdge.Top ? centerX - x : centerY - y;
        int span = edge is PopupPointerEdge.Bottom or PopupPointerEdge.Top ? width : height;
        int offset = Math.Clamp(axis, pointerInset, Math.Max(pointerInset, span - pointerInset));
        return new TrayPopupPlacement(x, y, width, height, edge, offset);
    }
}
