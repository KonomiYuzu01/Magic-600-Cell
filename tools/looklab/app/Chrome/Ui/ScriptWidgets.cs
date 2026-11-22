using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// The small drawn shapes that tell things apart without colour: severities in the gutter, script outputs.
public static class MarkShapes
{
    public static string Of(Severity severity) => severity == Severity.Error ? "diamond" : severity == Severity.Warning ? "triangle" : "dot";

    public static void Draw(CanvasItem on, string shape, Vector2 c, float r, Color colour)
    {
        switch (shape)
        {
            case "square": on.DrawRect(new Rect2(c - new Vector2(r, r) * .8f, new Vector2(r, r) * 1.6f), colour); break;
            case "triangle": on.DrawColoredPolygon(new[] { c + new Vector2(0, -r), c + new Vector2(r, r * .8f), c + new Vector2(-r, r * .8f) }, colour); break;
            case "diamond": on.DrawColoredPolygon(new[] { c + new Vector2(0, -r), c + new Vector2(r, 0), c + new Vector2(0, r), c + new Vector2(-r, 0) }, colour); break;
            default: on.DrawCircle(c, r * .8f, colour); break;
        }
    }
}

public sealed partial class MarkGlyph : Control
{
    private readonly string shape;
    private readonly Func<Color> colour;

    public MarkGlyph(string shape, Func<Color> colour, int size)
    {
        this.shape = shape; this.colour = colour; CustomMinimumSize = new Vector2(size, size); MouseFilter = MouseFilterEnum.Ignore; SizeFlagsVertical = SizeFlags.ShrinkCenter;
    }

    public override void _Draw() => MarkShapes.Draw(this, shape, Size / 2, Math.Min(Size.X, Size.Y) * .4f, colour());
}

// A chart of the host (ADR-D01.30): bars, a line or a heat strip, in one token colour. Display only.
public sealed partial class ChartView : Control
{
    public ChartWidget Chart { get; }
    private readonly Func<TokenSet> tokens;

    public ChartView(ChartWidget chart, Func<TokenSet> tokens)
    {
        if (!ChartWidget.Forms.Contains(chart.Form)) throw J.Error(chart.Form, "unknown chart form");
        Chart = chart; this.tokens = tokens; SizeFlagsHorizontal = SizeFlags.ExpandFill; Name = "Chart_" + chart.Form;
        CustomMinimumSize = new Vector2(0, tokens().Px("control.height") * (chart.Form == "heat" ? 1 : 2.5f));
        TooltipText = string.Join("\n", chart.Points.Select(p => $"{p.Label}: {p.Value.ToString("0.##", CultureInfo.InvariantCulture)}"));
    }

    public override void _Draw()
    {
        TokenSet t = tokens(); int n = Chart.Points.Count;
        if (n == 0) return;
        double top = Math.Max(1e-9, Chart.Points.Max(p => p.Value)); Color colour = t.C(Chart.Colour);
        float gap = t.Px("space.1"), thin = Math.Max(1, t.Px("border.width")), w = (Size.X - gap * (n - 1)) / n, h = Size.Y - thin;
        Font font = GetThemeFont("font", "Label"); int text = Math.Max(8, GetThemeFontSize("font_size", "Label") - 3);
        if (Chart.Form == "heat")
        {
            for (int i = 0; i < n; i++) DrawRect(new Rect2(i * (w + gap), 0, w, Size.Y), new Color(colour, (float)(.12 + .88 * Chart.Points[i].Value / top)));
            return;
        }
        DrawLine(new Vector2(0, h), new Vector2(Size.X, h), t.C("border.strong"), thin);
        var points = new Vector2[n];
        for (int i = 0; i < n; i++)
        {
            float tall = (float)(Chart.Points[i].Value / top) * (h - text - gap);
            points[i] = new Vector2(i * (w + gap) + w / 2, h - tall);
            if (Chart.Form == "bars") DrawRect(new Rect2(i * (w + gap), h - tall, w, tall), colour);
            else DrawCircle(points[i], thin * 2.5f, colour);
            if (w > text * 2.5f) DrawString(font, new Vector2(i * (w + gap), h - tall - gap), Chart.Points[i].Label, HorizontalAlignment.Center, w, text, t.C("text.dim"));
        }
        if (Chart.Form == "line" && n > 1) DrawPolyline(points, colour, thin * 1.5f, true);
    }
}

// The host's widgets for a script panel: typed parameter controls and read-only displays. A widget reports its
// new value to the host; the program never gets a control that commits.
public static class ScriptWidgets
{
    private static string Number(double value, double step) => value.ToString(step >= 1 ? "0" : step >= .1 ? "0.0" : "0.00", CultureInfo.InvariantCulture);

    private static HBoxContainer Row(PanelWidget widget)
    {
        var row = new HBoxContainer { Name = "Widget_" + widget.Id }; row.SetMeta("widget", widget.Id);
        Label caption = ChromeWidgets.Text(widget.Label, "DimLabel", true); caption.SizeFlagsStretchRatio = .7f; row.AddChild(caption);
        return row;
    }

    public static Control Build(PanelWidget widget, Func<TokenSet> tokens, Action<string, string> changed)
    {
        TokenSet t = tokens();
        switch (widget)
        {
            case NumberWidget n:
            {
                // The ladder: one step and ten steps, down and up.
                HBoxContainer row = Row(n);
                void Step(string text, int by, string tip)
                {
                    Button b = ChromeWidgets.Tool(text, tip, false, "FieldValue");
                    b.Pressed += () => changed(n.Id, Math.Clamp(n.Value + by * n.Step, n.Min, n.Max).ToString(CultureInfo.InvariantCulture)); row.AddChild(b);
                }
                Step("«", -10, "Ten steps down"); Step("‹", -1, "One step down");
                Label value = ChromeWidgets.Text(Number(n.Value, n.Step), "CodeLabel"); value.HorizontalAlignment = HorizontalAlignment.Center;
                value.CustomMinimumSize = new Vector2(t.Px("control.height") * 2, 0); row.AddChild(value);
                Step("›", 1, "One step up"); Step("»", 10, "Ten steps up");
                return row;
            }
            case ChoiceWidget c:
            {
                HBoxContainer row = Row(c);
                var pick = new OptionButton { SizeFlagsHorizontal = Control.SizeFlags.ExpandFill, FocusMode = Control.FocusModeEnum.None };
                foreach (string option in c.Options) pick.AddItem(option);
                pick.Selected = Math.Clamp(c.Chosen, 0, Math.Max(0, c.Options.Count - 1)); pick.ItemSelected += i => changed(c.Id, i.ToString(CultureInfo.InvariantCulture)); row.AddChild(pick);
                return row;
            }
            case SwitchWidget s:
            {
                HBoxContainer row = Row(s);
                var toggle = new CheckButton { ButtonPressed = s.On, FocusMode = Control.FocusModeEnum.None }; toggle.Toggled += on => changed(s.Id, on ? "on" : "off"); row.AddChild(toggle);
                return row;
            }
            case ReferenceWidget r:
            {
                HBoxContainer row = Row(r);
                Button pick = SessionWidgets.Tool("ref.pick", $"Take the selected {r.Kind}", t, () => changed(r.Id, "pick"), r.Value); pick.ThemeTypeVariation = "FieldValue"; row.AddChild(pick);
                return row;
            }
            case TokenWidget k:
            {
                HBoxContainer row = Row(k);
                foreach (string token in k.Tokens)
                {
                    string name = token; Button swatch = ChromeWidgets.Tool("", name, true, "FieldValue"); swatch.SetPressedNoSignal(name == k.Chosen); swatch.SetMeta("token", name);
                    swatch.CustomMinimumSize = new Vector2(t.Px("control.height"), t.Px("control.height"));
                    var mark = new MarkGlyph("square", () => tokens().C(name), t.Px("icon.small")); swatch.AddChild(mark); mark.SetAnchorsAndOffsetsPreset(Control.LayoutPreset.FullRect);
                    swatch.Pressed += () => changed(k.Id, name); row.AddChild(swatch);
                }
                return row;
            }
            case TableWidget table:
            {
                var grid = new GridContainer { Columns = Math.Max(1, table.Columns.Count), Name = "Widget_" + table.Id }; grid.SetMeta("widget", table.Id);
                foreach (string column in table.Columns) grid.AddChild(ChromeWidgets.Text(column, "MicroLabel", true));
                foreach (IReadOnlyList<string> line in table.Rows)
                    for (int i = 0; i < table.Columns.Count; i++) grid.AddChild(ChromeWidgets.Text(i < line.Count ? line[i] : "", i == 0 ? "DimLabel" : "CodeLabel", true));
                return grid;
            }
            case TreeWidget tree:
            {
                var column = new VBoxContainer { Name = "Widget_" + tree.Id }; column.SetMeta("widget", tree.Id); column.AddThemeConstantOverride("separation", 0);
                foreach ((int depth, string text, string value) in tree.Nodes)
                {
                    var row = new HBoxContainer(); column.AddChild(row);
                    row.AddChild(new Control { CustomMinimumSize = new Vector2(t.Px("icon.small") * depth, 0) });
                    Label fold = ChromeWidgets.Text("", "MicroLabel"); ChromeIcons.Apply(fold, depth == 0 ? "fold.open" : "tree.leaf", t.Px("icon.small")); row.AddChild(fold);
                    row.AddChild(ChromeWidgets.Text(text, depth == 0 ? "" : "DimLabel", true)); row.AddChild(ChromeWidgets.Text(value, "CodeLabel"));
                }
                return column;
            }
            case ChartWidget chart:
            {
                var column = new VBoxContainer { Name = "Widget_" + chart.Id }; column.SetMeta("widget", chart.Id);
                if (chart.Label.Length > 0) column.AddChild(ChromeWidgets.Text(chart.Label, "MicroLabel", true));
                column.AddChild(new ChartView(chart, tokens));
                return column;
            }
            case CanvasWidget canvas:
            {
                var surface = new CanvasSurface(tokens) { Name = "Widget_" + canvas.Id, CustomMinimumSize = new Vector2(0, t.Px("dock.width") * .7f) }; surface.SetMeta("widget", canvas.Id);
                surface.Show(canvas.Marks);
                return surface;
            }
            default: throw J.Error(widget.Id, "unknown widget");
        }
    }
}

// The consent sheet of a package by someone else (ADR-D01.32): what it is, whose it is, its digest and what it
// asks for, each with a glyph. It is a sheet inside the window, never a dialog of the system.
public sealed partial class ConsentSheet : ChromeOverlay
{
    public PackageManifest? Package { get; private set; }
    public Button Install { get; private set; } = null!;
    public event Action<PackageManifest, bool>? Decided;

    public ConsentSheet(Func<TokenSet> tokens) : base(tokens, .36f, .18f) { Name = "ConsentSheet"; }

    public void Ask(PackageManifest package)
    {
        Package = package; TokenSet t = Tokens();
        foreach (Node child in Panel.GetChildren()) { Panel.RemoveChild(child); child.QueueFree(); }
        var column = new VBoxContainer(); Panel.AddChild(column);
        Label Glyph(string icon, string colour)
        {
            Label mark = ChromeWidgets.Text("", ""); ChromeIcons.Apply(mark, icon, t.Px("icon.medium")); mark.AddThemeColorOverride("font_color", t.C(colour));
            return mark;
        }
        var head = new HBoxContainer(); column.AddChild(head);
        head.AddChild(Glyph("package", "script.frame")); head.AddChild(ChromeWidgets.Text(package.Title, "TitleLabel", true));
        column.AddChild(ChromeWidgets.Text("by " + package.Author, "DimLabel"));
        var digest = new HBoxContainer(); column.AddChild(digest);
        digest.AddChild(Glyph(package.Verified ? "task.ok" : "severity.error", package.Verified ? "severity.ok" : "severity.error"));
        Label hash = ChromeWidgets.Text(package.Digest.Length > 19 ? package.Digest.Substring(0, 19) + "…" : package.Digest, "CodeLabel"); hash.TooltipText = package.Digest; hash.MouseFilter = MouseFilterEnum.Pass; digest.AddChild(hash);
        digest.AddChild(ChromeWidgets.Text(package.Verified ? "digest matches" : "digest differs", "MicroLabel"));
        column.AddChild(new HSeparator());
        foreach (string capability in package.Capabilities)
        {
            var row = new HBoxContainer(); row.SetMeta("capability", capability); column.AddChild(row);
            row.AddChild(Glyph("cap." + capability, "text")); row.AddChild(ChromeWidgets.Text(PackageManifest.Known[capability], "", true));
        }
        var never = new HBoxContainer(); column.AddChild(never);
        never.AddChild(Glyph("lock", "state.protected")); never.AddChild(ChromeWidgets.Text("never commits", "DimLabel"));
        column.AddChild(new HSeparator());
        var buttons = new HBoxContainer { Alignment = BoxContainer.AlignmentMode.End }; column.AddChild(buttons);
        Button decline = SessionWidgets.Tool("pane.close", "Leave it uninstalled", t, () => Decide(false), "Decline"); decline.Name = "ConsentDecline";
        Install = SessionWidgets.Tool("package.install", package.Verified ? "Install with these capabilities" : "Its digest differs: it cannot be installed", t, () => Decide(true), "Install"); Install.Name = "ConsentInstall";
        Install.Disabled = !package.Verified;
        buttons.AddChild(decline); buttons.AddChild(Install);
        Open();
    }

    public void Decide(bool install)
    {
        if (Package is not { } package) return;
        Close(); Safe.Raise(Decided, package, install && package.Verified);
    }
}

// The one-line command bar over the selection (ADR-D01.21): a line about what is selected. Enter runs it and
// never commits; Up and Down step the lines typed before.
public sealed partial class CommandBar : ChromeOverlay
{
    public LineEdit Input { get; } = new() { Name = "CommandBarInput", SizeFlagsHorizontal = SizeFlags.ExpandFill };
    public LineHistory History { get; } = new();
    public event Action<string>? Entered;
    private readonly HBoxContainer row = new();
    private readonly HBoxContainer target = new();

    public CommandBar(Func<TokenSet> tokens) : base(tokens, .44f, .12f)
    {
        Name = "CommandBar"; Panel.AddChild(row); row.AddChild(target); row.AddChild(Input);
        Input.TextSubmitted += text => { if (text.Trim().Length > 0) { History.Add(text.Trim()); Close(); Safe.Raise(Entered, text.Trim()); } };
        Input.GuiInput += e =>
        {
            if (e is not InputEventKey { Pressed: true } key || key.Keycode is not (Godot.Key.Up or Godot.Key.Down)) return;
            Input.Text = History.Step(key.Keycode == Godot.Key.Up ? -1 : 1); Input.CaretColumn = Input.Text.Length; Input.AcceptEvent();
        };
    }

    public void Ask(string about)
    {
        TokenSet t = Tokens();
        foreach (Node child in target.GetChildren()) { target.RemoveChild(child); child.QueueFree(); }
        Label mark = ChromeWidgets.Text("", ""); ChromeIcons.Apply(mark, "command.bar", t.Px("icon.small")); target.AddChild(mark);
        target.AddChild(SessionWidgets.Chip(about, t.C("state.selected"), "The line is about the selection"));
        Label never = ChromeWidgets.Text("", ""); ChromeIcons.Apply(never, "lock", t.Px("icon.small")); never.AddThemeColorOverride("font_color", t.C("state.protected"));
        never.TooltipText = "Enter runs the line. It never commits."; never.MouseFilter = MouseFilterEnum.Pass; target.AddChild(never);
        Input.Text = ""; Open(); Input.GrabFocus();
    }
}

// Something that can be dragged into the editor as a typed reference: a history row, an inspector fact, a cell.
public static class ScriptDrag
{
    public static void Source(Control control, Func<string> reference) =>
        control.SetDragForwarding(Callable.From<Vector2, Variant>(_ => Variant.From(reference())), new Callable(), new Callable());
}
