using System;

namespace Magic600.Chrome;

public enum OverlayInk : byte { None, Cut, Outline, Preview }

// The D02 slots opt into this packing in styled.hlsl. The ABI still carries two palette nibbles, the fill's dim
// bit, and the outline's error bit. Outline tint and height tint are exclusive, so each mode has spare entries.
public static class JumbleStyle
{
    public const byte GlobalInactive = 0x80, LocalInactive = 0x08;

    public static OverlayInk Ink(bool line, bool cut, bool preview, bool markedPreview) =>
        line || (preview && markedPreview) ? OverlayInk.Outline
        : preview ? OverlayInk.Preview : cut ? OverlayInk.Cut : OverlayInk.None;

    public static bool Reference(OverlayInk ink) => ink is OverlayInk.Outline or OverlayInk.Preview;

    public static bool OutlineOnly(OverlayInk ink, InactiveMode mode, bool outside) =>
        Reference(ink) && mode == InactiveMode.Hidden && outside;

    public static InactiveMode FocusMode(FocusLayers? layers, Func<int, bool>? member) =>
        layers is { Dimming: true } && member != null ? layers.Inactive : InactiveMode.Normal;

    public static bool BoundFor(ulong? bound, ulong revision) => bound == revision;

    public static (byte Outline, byte Fill) Encode(byte outline, byte height, bool heightMode, OverlayInk ink,
                                                   bool globalOutside, bool localOutside)
    {
        int fill;
        if (heightMode)
        {
            // Height mode leaves just four ordinary outline entries: none, selection, hover and the motion flash.
            int stroke = (outline & 15) == JumbleOverlay.Entry ? 3 : outline & 15;
            outline = (byte)((outline & 0x10) | stroke | ((byte)ink << 2));
            fill = height == 0 ? 0 : height - JumbleTint.FirstHeight + 1;
        }
        else fill = (byte)ink;
        if (globalOutside) fill |= GlobalInactive;
        if (localOutside) fill |= LocalInactive;
        return (outline, (byte)fill);
    }

    public static (byte Outline, byte Height, OverlayInk Ink) Decode(byte outline, byte fill, bool heightMode)
    {
        if (!heightMode) return (outline, 0, (OverlayInk)(fill & 3));
        int stroke = outline & 3, height = fill & 7;
        return ((byte)((outline & 0x10) | (stroke == 3 ? JumbleOverlay.Entry : stroke)),
                height == 0 ? (byte)0 : (byte)(height + JumbleTint.FirstHeight - 1),
                (OverlayInk)((outline >> 2) & 3));
    }
}
