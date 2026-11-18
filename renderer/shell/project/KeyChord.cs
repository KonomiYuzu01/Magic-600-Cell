using Godot;

namespace Magic600.Shell;

/// <summary>
/// Godot's physical key codes as c600-input's chord form (charter E2; a draft): the W3C UI Events <c>code</c> of the
/// key's position, after its modifiers in the order Ctrl, Shift, Alt, Meta (<c>Shift+KeyJ</c>). Only the codes that
/// c600-input's chord parser knows are named; every other key, the modifier keys themselves among them, has none and
/// never reaches the host.
/// </summary>
public static class KeyChord
{
    /// <summary>The W3C code of a physical key, or null when c600-input does not know it.</summary>
    public static string? Code(Key key)
    {
        var value = (long)key;
        if (value >= (long)Key.A && value <= (long)Key.Z)
        {
            return "Key" + (char)value;
        }
        if (value >= (long)Key.Key0 && value <= (long)Key.Key9)
        {
            return "Digit" + (char)value;
        }
        if (value >= (long)Key.F1 && value <= (long)Key.F12)
        {
            return "F" + (value - (long)Key.F1 + 1);
        }
        return key switch
        {
            Key.Minus => "Minus",
            Key.Equal => "Equal",
            Key.Bracketleft => "BracketLeft",
            Key.Bracketright => "BracketRight",
            Key.Backslash => "Backslash",
            Key.Semicolon => "Semicolon",
            Key.Apostrophe => "Quote",
            Key.Quoteleft => "Backquote",
            Key.Comma => "Comma",
            Key.Period => "Period",
            Key.Slash => "Slash",
            Key.Space => "Space",
            Key.Enter => "Enter",
            Key.Escape => "Escape",
            Key.Tab => "Tab",
            Key.Backspace => "Backspace",
            Key.Delete => "Delete",
            Key.Insert => "Insert",
            Key.Home => "Home",
            Key.End => "End",
            Key.Pageup => "PageUp",
            Key.Pagedown => "PageDown",
            Key.Up => "ArrowUp",
            Key.Down => "ArrowDown",
            Key.Left => "ArrowLeft",
            Key.Right => "ArrowRight",
            Key.Menu => "ContextMenu",
            _ => null,
        };
    }

    /// <summary>The chord of a key event: its modifiers, then its physical key's code.</summary>
    public static string Chord(InputEventKey key, string code) =>
        (key.CtrlPressed ? "Ctrl+" : "") + (key.ShiftPressed ? "Shift+" : "") + (key.AltPressed ? "Alt+" : "")
        + (key.MetaPressed ? "Meta+" : "") + code;
}
