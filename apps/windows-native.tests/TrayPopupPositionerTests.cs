using RimV.Windows.Application;

namespace RimV.Windows.UnitTests;

public sealed class TrayPopupPositionerTests
{
    [Theory]
    [InlineData(950, 1040, PopupPointerEdge.Bottom)]
    [InlineData(950, 0, PopupPointerEdge.Top)]
    [InlineData(0, 500, PopupPointerEdge.Left)]
    [InlineData(1900, 500, PopupPointerEdge.Right)]
    public void PopupStaysInsideWorkAreaAndPointerTracksTrayIcon(int iconX, int iconY, PopupPointerEdge expectedEdge)
    {
        var work = new PixelRect(40, 40, 1860, 1020);
        var icon = new PixelRect(iconX, iconY, iconX + 24, iconY + 24);

        TrayPopupPlacement placement = TrayPopupPositioner.Place(icon, work, 420, 620, 32);

        Assert.Equal(expectedEdge, placement.Edge);
        Assert.InRange(placement.X, work.Left, work.Right - placement.Width);
        Assert.InRange(placement.Y, work.Top, work.Bottom - placement.Height);
        int actualAxis = expectedEdge is PopupPointerEdge.Bottom or PopupPointerEdge.Top
            ? placement.X + placement.PointerOffset
            : placement.Y + placement.PointerOffset;
        int expectedAxis = expectedEdge is PopupPointerEdge.Bottom or PopupPointerEdge.Top ? icon.CenterX : icon.CenterY;
        Assert.Equal(expectedAxis, actualAxis);
    }

    [Fact]
    public void OverflowIconUsesItsActualPositionRatherThanTheTaskbarCorner()
    {
        var work = new PixelRect(-1920, 0, 0, 1040);
        var icon = new PixelRect(-420, 900, -388, 932);

        TrayPopupPlacement placement = TrayPopupPositioner.Place(icon, work, 420, 620, 32);

        Assert.Equal(PopupPointerEdge.Bottom, placement.Edge);
        Assert.Equal(icon.CenterX, placement.X + placement.PointerOffset);
        Assert.InRange(placement.X, work.Left, work.Right - placement.Width);
    }
}
