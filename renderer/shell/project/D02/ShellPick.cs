using System;
using System.Collections.Generic;
using System.Globalization;
using System.Text.Json;
using Godot;

namespace Magic600.Shell.D02;

/// <summary>
/// The views' shared pick context behind M600_D02=1 (ADR-D02 build step B17; ADR-D01.41 and .67): one selection and
/// one hover for every view, set by the core's ID pass through the host (host version 14, m6h_pick_request and
/// m6h_pick_poll). A click selects the piece of the sticker under the pointer, in Global or Local; a click on no
/// sticker clears the selection. Moving the pointer over a view hovers, at most one pick in flight and one every
/// <see cref="HoverEvery"/>. A pick whose frame drew a revision the head has since left is dropped (the host answers it
/// stale), so the selection never names a piece by an older state. Selection and hover are linked across the views: both
/// views read this one context. Nothing here commits or changes the session.
/// </summary>
public static class ShellPick
{
    /// <summary>How often a moving pointer asks for a hover pick, at most.</summary>
    public static readonly TimeSpan HoverEvery = TimeSpan.FromMilliseconds(80);

    /// <summary>A picked sticker: its piece, the sticker's home slot, the slot it occupies (none off the lattice), the
    /// view it was picked in and the revision the picked frame drew.</summary>
    public sealed record Picked(uint Piece, uint Sticker, uint? Slot, bool Lattice, ulong View, ulong Revision)
    {
        /// <summary>The cell the sticker occupies (ADR-D02: a cap is named by its cell), or none off the lattice.</summary>
        public int? Cell => Slot is uint slot ? (int)(slot / 433) : null;
    }

    /// <summary>The selection, or none.</summary>
    public static Picked? Selection { get; private set; }

    /// <summary>The hovered sticker, or none.</summary>
    public static Picked? Hover { get; private set; }

    /// <summary>Raised when the selection or the hover changes.</summary>
    public static event Action? Changed;

    /// <summary>ADR-D02 B17: raised when a click sets a different inspector selection.</summary>
    public static event Action? SelectionChanged;

    /// <summary>Clicks asked, answered with a hit, answered with none, and dropped as stale (for the run's dump).</summary>
    public static (int Asked, int Hits, int Misses, int Stale) Counts => (_asked, _hits, _misses, _stale);

    private static readonly List<(ulong Ticket, (uint X, uint Y)? HoverAt, Func<Picked?, bool>? Take)> Pending = new();
    private static (uint X, uint Y, Func<Picked?, bool>? Take)? _click;
    private static (uint X, uint Y)? _hover;
    private static DateTime _hoverAt = DateTime.MinValue;
    private static int _asked, _hits, _misses, _stale;

    /// <summary>A click at a surface pixel: the next frame asks the ID pass there.</summary>
    public static void Click(uint x, uint y) => _click = (x, y, null);

    /// <summary>The pointer at a surface pixel, or off every view (null clears the hover).</summary>
    public static void Point((uint X, uint Y)? at)
    {
        if (at is null && Hover is not null)
        {
            Hover = null;
            _pickedHoverAt = null;
            Changed?.Invoke();
        }
        _hover = at;
    }

    /// <summary>Clears the selection (a key or the chrome may ask for it).</summary>
    public static void Clear()
    {
        if (Selection is not null)
        {
            Selection = null;
            SelectionChanged?.Invoke();
            Changed?.Invoke();
        }
    }

    /// <summary>
    /// Once per frame, after the frame was submitted: asks the waiting click (and a hover, when due and none is in
    /// flight) on the newest frame, then polls every live pick. Answers whether the selection or hover changed.
    /// </summary>
    public static bool Step(HostLibrary library, IntPtr host)
    {
        var changed = false;
        if (_click is { } click)
        {
            var code = library.PickRequest(host, click.X, click.Y, out var ticket);
            if (code == HostLibrary.Ok)
            {
                Pending.Add((ticket, null, click.Take));
                _asked++;
            }
            // ADR-D02 B17: keep the click until a current frame's ID resources are ready.
            if (code != HostLibrary.Unavailable)
            {
                _click = null;
            }
        }
        var now = DateTime.UtcNow;
        if (_hover is { } hover && now - _hoverAt >= HoverEvery && !Pending.Exists(p => p.HoverAt != null)
            && library.PickRequest(host, hover.X, hover.Y, out var hoverTicket) == HostLibrary.Ok)
        {
            Pending.Add((hoverTicket, hover, null));
            _hoverAt = now;
        }
        for (var i = Pending.Count - 1; i >= 0; i--)
        {
            var (t, hoverAt, take) = Pending[i];
            bool isClick = hoverAt == null;
            if (library.PickPoll(host, t, out var pick) != HostLibrary.Ok)
            {
                Pending.RemoveAt(i);
                continue;
            }
            if (pick.State == HostLibrary.PickPending)
            {
                continue;
            }
            Pending.RemoveAt(i);
            if (pick.State == HostLibrary.PickStale)
            {
                if (isClick)
                {
                    _stale++;
                }
                continue;
            }
            var picked = pick.Hit == 1
                ? new Picked(pick.Piece, pick.Sticker, pick.Slot == HostLibrary.None ? null : pick.Slot, pick.Lattice == 1,
                    pick.View, pick.EngineRevision)
                : null;
            if (isClick)
            {
                if (picked is null)
                {
                    _misses++;
                }
                else
                {
                    _hits++;
                }
                if (take?.Invoke(picked) != true && Selection != picked)
                {
                    Selection = picked;
                    SelectionChanged?.Invoke();
                    changed = true;
                }
            }
            else if (hoverAt == _hover)
            {
                _pickedHoverAt = hoverAt;
                if (Hover != picked)
                {
                    Hover = picked;
                    changed = true;
                }
            }
        }
        if (changed)
        {
            Changed?.Invoke();
        }
        return changed;
    }

    private static (uint X, uint Y)? _pickedHoverAt;

    /// <summary>The hover ID at the pointer's current pixel, once its pick has returned.</summary>
    public static Picked? HoverAtPointer => _pickedHoverAt == _hover ? Hover : null;

    /// <summary>A click whose confirmed hit can take the direct-twist route before selection.</summary>
    public static void GripClick(uint x, uint y, Func<Picked?, bool> take) => _click = (x, y, take);

    /// <summary>
    /// A D02 script's <c>click</c> step, <c>{"click": [fx, fy]}</c>: a left click at fractions fx, fy of the Global
    /// opening's width and height, through the shell's input. Answers the step's name for the run's dump.
    /// </summary>
    public static string ScriptClick(Rect2 opening, JsonElement click)
    {
        var fx = click[0].GetSingle();
        var fy = click[1].GetSingle();
        var at = opening.Position + new Vector2(opening.Size.X * fx, opening.Size.Y * fy);
        Input.ParseInputEvent(new InputEventMouseButton { ButtonIndex = MouseButton.Left, Pressed = true, Position = at, GlobalPosition = at, ButtonMask = MouseButtonMask.Left });
        Input.ParseInputEvent(new InputEventMouseButton { ButtonIndex = MouseButton.Left, Pressed = false, Position = at, GlobalPosition = at });
        return "click " + fx.ToString(CultureInfo.InvariantCulture) + " " + fy.ToString(CultureInfo.InvariantCulture);
    }

    /// <summary>Writes the pick context into a D02 run's screen dump (the d02_run.py pick check reads it).</summary>
    public static void Dump(Utf8JsonWriter json)
    {
        json.WriteStartObject("pick");
        Write(json, "selection", Selection);
        Write(json, "hover", Hover);
        json.WriteNumber("asked", _asked);
        json.WriteNumber("hits", _hits);
        json.WriteNumber("misses", _misses);
        json.WriteNumber("stale", _stale);
        json.WriteEndObject();
    }

    private static void Write(Utf8JsonWriter json, string name, Picked? picked)
    {
        if (picked is null)
        {
            json.WriteNull(name);
            return;
        }
        json.WriteStartObject(name);
        json.WriteNumber("piece", picked.Piece);
        json.WriteNumber("sticker", picked.Sticker);
        if (picked.Slot is uint slot)
        {
            json.WriteNumber("slot", slot);
        }
        else
        {
            json.WriteNull("slot");
        }
        json.WriteBoolean("lattice", picked.Lattice);
        json.WriteNumber("view", picked.View);
        json.WriteNumber("revision", picked.Revision);
        json.WriteEndObject();
    }
}
