using System;
using System.Buffers.Binary;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Security.Cryptography;
using System.Text.Json;
using Godot;
using Magic600.Chrome;
using Quaternion = System.Numerics.Quaternion;
using Vector2 = Godot.Vector2;
using Vector3 = System.Numerics.Vector3;

namespace Magic600.Shell.D02;

/// <summary>D02's grip follower: the host owns the exact key bindings; the shared chrome model draws their marks.</summary>
public static partial class ShellGrip
{
    public static GripFrame Model { get; } = new();
    public static bool Held { get; private set; }
    public static int Gesture { get; private set; }
    public static byte[]? Moving { get; private set; }

    private static HostLibrary? _library;
    private static IntPtr _host;
    private static CorePlaces? _places;
    private static Func<ulong> _adopted = () => 0;
    private static Func<bool> _canClick = () => false;
    private static Func<IEnumerable<GripPane>> _panes = () => Array.Empty<GripPane>();
    private static GripLayer? _layer;
    private static byte[]? _rings;

    internal static void Connect(HostLibrary library, IntPtr host, CorePlaces places, Func<ulong> adopted,
        Func<bool> canClick, Func<IEnumerable<GripPane>> panes)
    {
        (_library, _host, _places, _adopted, _canClick, _panes) = (library, host, places, adopted, canClick, panes);
        if (library.GripReport(host) == HostLibrary.Ok)
        {
            using var answer = JsonDocument.Parse(library.Answer(host));
            Model.Set(answer.RootElement);
        }
    }

    public static void Disconnect()
    {
        FocusLost();
        _library = null;
        _host = IntPtr.Zero;
    }

    public static void Attach(JumbleHost host, ChromeFrame frame)
    {
        host.Grip = Model;
        host.GripRow = Request;
        var layer = _layer = new GripLayer(frame) { Name = "GripMarks", MouseFilter = Control.MouseFilterEnum.Ignore };
        frame.Overlays.AddChild(layer);
        layer.SetAnchorsAndOffsetsPreset(Control.LayoutPreset.FullRect);
        frame.Overlays.MoveChild(layer, 0);
        Model.Changed += CancelGesture;
        ShellPick.SelectionChanged += CancelGesture;
        frame.TreeExiting += () =>
        {
            Model.Changed -= CancelGesture;
            ShellPick.SelectionChanged -= CancelGesture;
            _layer = null;
        };
    }

    private static string? Request(string row, string? argument, bool execute)
    {
        if (_library is not { } library || !Model.Ready) return "Grip needs a live Jumble session";
        int cap, frame;
        ulong? pickedRevision = null;
        if (row == GripFrame.NextRow)
        {
            if (argument != null) return "Next grip frame takes no argument";
            if (Model.NextFrame is not int next || next == Model.Frame) return "No other frame is available";
            (cap, frame) = (Model.Cap, next);
        }
        else
        {
            if (!ParseSelection(argument, out var selected, out frame)) return "Use a cap and optional frame";
            if (selected is not int)
            {
                if (ShellPick.Selection is not { } selection || selection.Revision != _adopted())
                    return "Pick a sticker at the current revision";
                if (selection.Cell is not int cell) return "The picked piece has no cell";
                selected = cell;
                pickedRevision = selection.Revision;
            }
            cap = selected!.Value;
        }
        if (!execute) return null;
        if (pickedRevision is ulong revision)
        {
            if (library.GripReport(_host) != HostLibrary.Ok) return library.LastError(_host);
            using var report = JsonDocument.Parse(library.Answer(_host));
            if (report.RootElement.GetProperty("revision").GetUInt64() != revision)
                return "Pick a sticker at the current revision";
        }
        if (library.GripSet(_host, (uint)cap, (uint)frame) != HostLibrary.Ok) return library.LastError(_host);
        using var answer = JsonDocument.Parse(library.Answer(_host));
        Model.Set(answer.RootElement);
        return null;
    }

    private static bool ParseSelection(string? argument, out int? cap, out int frame)
    {
        cap = null;
        frame = 0;
        if (string.IsNullOrWhiteSpace(argument)) return true;
        if (argument.TrimStart().StartsWith('{'))
        {
            try
            {
                using var json = JsonDocument.Parse(argument);
                foreach (var field in json.RootElement.EnumerateObject())
                {
                    if (field.Value.ValueKind != JsonValueKind.Number || !field.Value.TryGetInt32(out var value)) return false;
                    if (field.Name == "cap" && value is >= 0 and < CorePlaces.Cells) cap = value;
                    else if (field.Name == "frame" && value is >= 0 and < 12) frame = value;
                    else return false;
                }
                return true;
            }
            catch (JsonException) { return false; }
        }
        var parts = argument.Split(' ', StringSplitOptions.RemoveEmptyEntries);
        if (parts.Length > 2 || !int.TryParse(parts[0], NumberStyles.None, CultureInfo.InvariantCulture, out int id) ||
            id is < 0 or >= CorePlaces.Cells) return false;
        cap = id;
        return parts.Length == 1 || (int.TryParse(parts[1], NumberStyles.None, CultureInfo.InvariantCulture, out frame) && frame is >= 0 and < 12);
    }

    public static void Follow(InputEventKey key)
    {
        if (_library == null || key.PhysicalKeycode != Key.Ctrl || key.Echo) return;
        Held = key.Pressed;
        if (!Held) CancelGesture();
    }

    public static void FocusLost()
    {
        Held = false;
        CancelGesture();
    }

    public static void CancelGesture()
    {
        Gesture++;
        SetMoving(null);
    }

    private static (int Cap, ulong Item, ulong Revision)? _read;
    private static HostLibrary.TwistRow? _row;
    private static ulong _rowRevision;

    private static byte[]? ReadMoving(ulong revision)
    {
        if (_library is not { } library || Model.Key("KeyJ") is not { } key) return null;
        var request = (Model.Cap, key.Item, revision);
        if (_read != request)
        {
            library.TwistTransform(_host, (uint)Model.Cap, key.Item, out _row, out _rowRevision);
            _read = request;
        }
        return ShellTurn.InsideFor(_row, revision);
    }

    private static bool HitsGrip(ShellPick.Picked pick, byte[]? moving) =>
        pick.Cell == Model.Cap || (moving != null && pick.Piece < moving.Length && moving[pick.Piece] != 0);

    public static bool ClickHits(ShellPick.Picked? pick, ulong revision)
    {
        if (!Held || !_canClick() || pick is null || pick.Revision != revision || !Model.Ready) return false;
        // This is a new ID-pass result for the click. Recheck the head even when a hover cached its row.
        _read = null;
        var moving = ReadMoving(revision);
        return _rowRevision == revision && HitsGrip(pick, moving);
    }

    public static void StepGhost(ulong revision)
    {
        byte[]? active = null;
        if (Held && _canClick() && ShellPick.HoverAtPointer is { } pick && pick.Revision == revision &&
            ReadMoving(revision) is { } moving && HitsGrip(pick, moving)) active = moving;
        SetMoving(active);
    }

    private static void SetMoving(byte[]? moving)
    {
        if (Moving == moving) return;
        Moving = moving;
        ShellStyle.Invalidate();
    }

    private static bool GhostOn => Held && Moving != null && _rowRevision == _adopted() && _canClick();

    internal readonly record struct GripPane(string Name, Rect2 Rect, CoreView View);

    private static Vector2? Project(GripPane pane, IReadOnlyList<double>? point)
    {
        if (_places?.PlaceCellPoint(Model.Cap, point, pane.View) is not { } at) return null;
        var pixel = pane.Rect.Position + pane.Rect.Size * new Vector2((float)at.X, (float)at.Y);
        return pane.Rect.HasPoint(pixel) ? pixel : null;
    }

    internal static Vector2? Centre(string paneName)
    {
        foreach (var pane in _panes())
            if (pane.Name == paneName) return Project(pane, null);
        return null;
    }

    // The core's product.cpp Hopf chart for a fixed position of its own cell. The unposed cap is already its
    // chart cell, so it needs no nearest-cell search. Ring and half-step come from the same row the core took.
    internal static Func<int, double[], (double X, double Y, double Z)> Chart(uint kind, CorePlaces places, double[] spin) =>
        (cell, point) =>
        {
            var row = _rings ??= ShellHopf.Layout(ShellHopf.RingsPath(), ShellHopf.Rings);
            float Read(int offset) => BinaryPrimitives.ReadSingleLittleEndian(row.AsSpan(offset, 4));
            Quaternion ReadQ(int offset) => new(Read(offset + 4), Read(offset + 8), Read(offset + 12), Read(offset));
            Vector3 Imag(Quaternion q) => new(q.X, q.Y, q.Z);
            Vector3 Turn(Quaternion o, Quaternion q) => Imag(o * q * Quaternion.Conjugate(o));
            var j = ReadQ(8);
            var k = ReadQ(24);
            var jk = j * k;
            var ring = row[360 + cell];
            var half = row[960 + cell];
            var pole = places.CellCentre(cell);
            Quaternion Q(IReadOnlyList<double> v) => new((float)v[1], (float)v[2], (float)v[3], (float)v[0]);
            var c = Q(pole);
            var a = (18 - 72 * half) * Math.PI / 180;
            var e = c * new Quaternion(Imag(j) * (float)Math.Sin(a), (float)Math.Cos(a));
            var w = Q(point);
            var x = Quaternion.Dot(c * j, w);
            var y = Quaternion.Dot(e * k, w);
            var z = Quaternion.Dot(e * jk, w);
            if (kind == ShellHopf.Sheet)
                return (.6 * (x + .68 * (half / 2.0 - 14.75)), .6 * (y - .57 * (ring - 9.5)), .6 * z);
            var origin = ReadQ(40 + 16 * ring);
            var origin0 = ReadQ(40);
            var basis = Turn(origin, j);
            var n = Turn(origin, k);
            var t = Turn(origin, jk);
            var angle = half * Math.PI / 30;
            float cos = (float)Math.Cos(angle), sin = (float)Math.Sin(angle);
            var position = .28f * ((8.063f - z) * basis + (2.62f + y) * (cos * n + sin * t) + x * (-sin * n + cos * t));
            var placed = new Vector3(Vector3.Dot(Turn(origin0, k), position), Vector3.Dot(Turn(origin0, jk), position),
                Vector3.Dot(Turn(origin0, j), position));
            return (spin[0] * placed.X + spin[1] * placed.Y + spin[2] * placed.Z,
                spin[4] * placed.X + spin[5] * placed.Y + spin[6] * placed.Z,
                spin[8] * placed.X + spin[9] * placed.Y + spin[10] * placed.Z);
        };

    public static void Dump(Utf8JsonWriter json)
    {
        if (_library is not { } library || library.GripReport(_host) != HostLibrary.Ok) return;
        using var answer = JsonDocument.Parse(library.Answer(_host));
        var report = answer.RootElement;
        json.WriteStartObject("grip");
        Model.Dump(json);
        json.WritePropertyName("host");
        report.WriteTo(json);
        foreach (string name in new[] { "committed", "refused", "state_digest" })
        {
            json.WritePropertyName(name);
            report.GetProperty(name).WriteTo(json);
        }
        json.WriteStartArray("drawn");
        _layer?.Dump(json);
        json.WriteEndArray();
        json.WriteStartObject("ghost");
        var moving = GhostOn ? Moving : null;
        json.WriteBoolean("on", moving != null);
        json.WriteString("modifier", GripFrame.Modifier);
        json.WriteNumber("cap", Model.Cap);
        if (Model.Key("KeyJ") is { } key) json.WriteNumber("item", key.Item);
        json.WriteNumber("moving", moving?.Count(b => b != 0) ?? 0);
        json.WriteString("sha256", moving == null ? null : Convert.ToHexString(SHA256.HashData(moving)).ToLowerInvariant());
        json.WriteNumber("palette", ShellStyle.Hovered);
        json.WriteNumber("outlined", moving != null && ShellStyle.SentOutline is { } outline
            ? Enumerable.Range(0, moving.Length).Count(i => moving[i] != 0 && (outline[i] & 15) == ShellStyle.Hovered) : 0);
        json.WriteEndObject();
        json.WriteEndObject();
    }

    private sealed partial class GripLayer : Control
    {
        private readonly ChromeFrame frame;
        private sealed record Placed(GripMark Mark, Vector2 At, Vector2 Label);
        private readonly List<(GripPane Pane, IReadOnlyList<Placed> Marks, Vector2? Direction)> drawn = new();

        public GripLayer(ChromeFrame frame) => this.frame = frame;

        public override void _Process(double delta)
        {
            if (Model.Ready) QueueRedraw();
        }

        public override void _Draw()
        {
            drawn.Clear();
            if (!Model.Ready || _library == null) return;
            foreach (var pane in _panes())
            {
                var marks = PlaceMarks(pane);
                Vector2? direction = null;
                if (GhostOn && Model.Key("KeyJ") is { } key && Project(pane, null) is { } centre)
                {
                    GripMarks.Direction(this, centre - GlobalPosition, key, frame.Tokens);
                    direction = centre;
                }
                foreach (var mark in marks)
                {
                    var at = mark.At - GlobalPosition;
                    var label = mark.Label - GlobalPosition;
                    if (at != label)
                    {
                        var colour = frame.Tokens.C("text.primary");
                        DrawLine(at, label, colour, 1, true);
                        DrawCircle(at, 2, colour);
                    }
                    GripMarks.Draw(this, label, mark.Mark, frame.Tokens);
                }
                drawn.Add((pane, marks, direction));
            }
        }

        private IReadOnlyList<Placed> PlaceMarks(GripPane pane)
        {
            var marks = new List<Placed>();
            foreach (var mark in Model.Marks)
                if (Project(pane, mark.Point) is { } at) marks.Add(new(mark, at, at));
            var spacing = 2 * GripMarks.Radius(frame.Tokens) + 2;
            if (!marks.Any(a => marks.Any(b => a != b && a.At.DistanceTo(b.At) < spacing))) return marks;
            // A distant cap may fit in a few pixels. Keep its locations pinned and put overlapping labels on leaders.
            if (pane.Rect.Size.X < 4 * spacing || pane.Rect.Size.Y < 4 * spacing) return marks;
            var centre = marks.Select(m => m.At).Aggregate((a, b) => a + b) / marks.Count;
            var inset = new Vector2(2 * spacing, 2 * spacing);
            centre = centre.Clamp(pane.Rect.Position + inset, pane.Rect.End - inset);
            return marks.OrderBy(m => m.At.Y).Chunk(2).SelectMany((row, i) =>
                row.OrderBy(m => m.At.X).Select((m, j) => m with
                { Label = centre + new Vector2(j == 0 ? -spacing : spacing, i == 0 ? -spacing : spacing) })).ToArray();
        }

        public void Dump(Utf8JsonWriter json)
        {
            foreach (var (pane, marks, direction) in drawn)
            {
                json.WriteStartObject();
                json.WriteString("pane", pane.Name);
                json.WriteStartArray("rect");
                json.WriteNumberValue(pane.Rect.Position.X);
                json.WriteNumberValue(pane.Rect.Position.Y);
                json.WriteNumberValue(pane.Rect.Size.X);
                json.WriteNumberValue(pane.Rect.Size.Y);
                json.WriteEndArray();
                json.WriteStartArray("marks");
                foreach (var (mark, at, label) in marks)
                {
                    json.WriteStartObject();
                    json.WriteNumber("index", mark.Index);
                    json.WriteNumber("corner", mark.Corner);
                    json.WriteNumber("x", at.X);
                    json.WriteNumber("y", at.Y);
                    json.WriteNumber("label_x", label.X);
                    json.WriteNumber("label_y", label.Y);
                    json.WriteEndObject();
                }
                json.WriteEndArray();
                if (direction is { } centre)
                {
                    json.WriteStartArray("direction");
                    json.WriteNumberValue(centre.X);
                    json.WriteNumberValue(centre.Y);
                    json.WriteEndArray();
                }
                json.WriteEndObject();
            }
        }
    }
}
