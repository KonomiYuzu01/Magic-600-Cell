using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// ADR-D02.28: the ν band drawn under the rail's cells, one column per cell in the rail's order. A 0/1 mark is a
// diamond (filled for 1), outside the ring family; a frame group's mark is a smaller diamond below it; O17 and O28
// carry a five-position dial; the nine always-zero coordinates are fixed dots. Off the lattice the band shows
// numeral.undefined as a whole.
public sealed partial class NuBandStrip : Control
{
    private readonly Func<TokenSet> tokens;
    public NuBand? Band { get; private set; }

    public NuBandStrip(Func<TokenSet> tokens)
    {
        this.tokens = tokens; Name = "NuBand"; MouseFilter = MouseFilterEnum.Ignore; SizeFlagsHorizontal = SizeFlags.ExpandFill;
        CustomMinimumSize = new Vector2(0, 34); Visible = false;
    }

    public void SetBand(NuBand? band)
    {
        Band = band;
        Visible = band is { Shown: true } or { Undefined: true };
        QueueRedraw();
    }

    public override void _Draw()
    {
        if (Band is not { } band) return;
        TokenSet t = tokens(); float step = Size.X / OrbitAtlas.Count, h = Size.Y;
        Font font = GetThemeFont("font", "MicroLabel"); int fontSize = Math.Max(8, GetThemeFontSize("font_size", "MicroLabel") - 1);
        if (band.Undefined)
        {
            Color u = t.C(Numeral.Undefined.Role);
            for (float x = 0; x < Size.X - 1; x += 6) DrawLine(new Vector2(x, h - 2), new Vector2(Math.Min(x + 6, Size.X), 2), u with { A = u.A * .35f }, 1);
            DrawString(font, new Vector2(0, h / 2 + fontSize / 2f), "ν " + Numeral.Undefined.Word, HorizontalAlignment.Center, Size.X, fontSize, u);
            return;
        }
        Color mark = t.C("rail.cell.id"), dim = t.C("text.dim");
        for (int i = 0; i < band.Cells.Count; i++)
        {
            NuCell c = band.Cells[i]; float mid = i * step + step / 2;
            Diamond(new Vector2(mid, 6), 3.5f, c.BitFixed ? null : c.Bit, mark, dim);
            if (c.HasFrame) Diamond(new Vector2(mid, 15), 2.6f, c.FrameFixed ? null : c.Frame, mark, dim);
            if (c.HasDial) Dial(new Vector2(mid, 26), 5.5f, c.Dial ?? 0, mark, dim);
        }
    }

    // A ν mark: a diamond filled for 1 and outlined for 0; a fixed coordinate is a fixed dot.
    private void Diamond(Vector2 c, float r, int? value, Color on, Color dim)
    {
        if (value == null) { DrawCircle(c, 1.1f, dim); return; }
        var p = new[] { c + new Vector2(0, -r), c + new Vector2(r, 0), c + new Vector2(0, r), c + new Vector2(-r, 0) };
        if (value != 0) DrawColoredPolygon(p, on);
        else DrawPolyline(p.Append(p[0]).ToArray(), dim, 1, true);
    }

    // A five-position dial: five fixed ticks and a hand to the value's position; never a bar.
    private void Dial(Vector2 c, float r, int value, Color on, Color dim)
    {
        for (int k = 0; k < NuConstants.DialPositions; k++)
        {
            float a = k * Mathf.Tau / NuConstants.DialPositions;
            Vector2 at = c + new Vector2(Mathf.Sin(a), -Mathf.Cos(a)) * r;
            DrawCircle(at, k == value ? 1.5f : .9f, k == value ? on : dim);
        }
        float v = value * Mathf.Tau / NuConstants.DialPositions;
        DrawLine(c, c + new Vector2(Mathf.Sin(v), -Mathf.Cos(v)) * (r - 1.5f), on, 1.2f, true);
    }
}

// ADR-D02.39: the ν-corrections rows (Library › Built-in › ν-corrections), in the left dock. Up and Down move the
// selection; Space marks the selected row into the set (+1), Shift+Space marks it -1, Delete takes it out; Shift+Enter
// previews it alone; Ctrl+Enter checks the set; Home and End go to the first and last row. The rows are in id order and none is ranked.
public sealed partial class NuCorrectionsPanel : VBoxContainer, IOwnKeys
{
    public const string PanelId = "jumble.corrections";
    public NuCorrections? Model { get; private set; }
    private Func<IReadOnlyList<int>?> start = () => null;
    private readonly Label path = ChromeWidgets.Text(NuCorrections.Path, "MicroLabel", true), preview = ChromeWidgets.Text("", "MicroLabel", true),
        check = ChromeWidgets.Text("", "", true), keys = ChromeWidgets.Text("Space marks · Shift+Space −1 · Shift+Enter previews · Ctrl+Enter checks the set", "MicroLabel", true);
    private readonly ItemList rows = new() { Name = "CorrectionRows", SelectMode = ItemList.SelectModeEnum.Single, FocusMode = FocusModeEnum.None, SizeFlagsVertical = SizeFlags.ExpandFill };

    public NuCorrectionsPanel()
    {
        Name = "NuCorrectionsPanel"; FocusMode = FocusModeEnum.All;
        path.Name = "CorrectionsPath"; preview.Name = "CorrectionPreview"; check.Name = "CorrectionCheck";
        rows.CustomMinimumSize = new Vector2(0, 220);
        rows.ItemClicked += (index, _, _) => { Model?.Select((int)index); GrabFocus(); };
        AddChild(path); AddChild(rows); AddChild(keys); AddChild(preview); AddChild(check);
        TreeEntered += () => { if (Model != null) Model.Changed += Refresh; Refresh(); };
        TreeExiting += () => { if (Model != null) Model.Changed -= Refresh; };
    }

    public void Bind(NuCorrections? model, Func<IReadOnlyList<int>?> stateNu)
    {
        if (Model != null) Model.Changed -= Refresh;
        Model = model; start = stateNu;
        if (Model != null && IsInsideTree()) Model.Changed += Refresh;
        Refresh();
    }

    public NuCorrectionCheck? CheckSet() => Model?.Check(start());

    public override void _GuiInput(InputEvent e)
    {
        if (Model is not { } m || e is not InputEventKey { Pressed: true } key) return;
        switch (key.Keycode)
        {
            case Key.Down: m.Move(1); break;
            case Key.Up: m.Move(-1); break;
            case Key.Home: m.Select(0); break;
            case Key.End: m.Select(m.Rows.Count - 1); break;
            case Key.Space when !key.Echo: m.Mark(m.Current.Id, key.ShiftPressed ? -1 : 1); break;
            case Key.Delete or Key.Backspace when !key.Echo: m.Unmark(m.Current.Id); break;
            case Key.Enter or Key.KpEnter when !key.Echo && key.CtrlPressed: CheckSet(); break;
            case Key.Enter or Key.KpEnter when !key.Echo && key.ShiftPressed: m.Preview(); break;
            default: return;
        }
        AcceptEvent();
    }

    // The panel as text, for the shell's dump.
    public IReadOnlyDictionary<string, string> Lines() => new Dictionary<string, string>
    {
        ["path"] = path.Text, ["rows"] = rows.ItemCount.ToString(System.Globalization.CultureInfo.InvariantCulture),
        ["selected"] = Model == null ? "" : Model.RowLine(Model.Current), ["preview"] = preview.Text, ["check"] = check.Text,
        ["set"] = Model == null ? "" : string.Join(" ", Model.Set.Select(p => p.Key + ":" + p.Value)),
    };

    private void Refresh()
    {
        if (!IsInstanceValid(this)) return;
        rows.Clear();
        if (Model is not { } m) { preview.Text = check.Text = ""; return; }
        foreach (NuCorrection r in m.Rows) rows.AddItem(m.RowLine(r));
        rows.Select(m.Selected); rows.EnsureCurrentIsVisible();
        preview.Text = m.PreviewLine(start()) ?? "";
        check.Text = m.Checked?.Line(m.Constants) ?? "";
    }
}
