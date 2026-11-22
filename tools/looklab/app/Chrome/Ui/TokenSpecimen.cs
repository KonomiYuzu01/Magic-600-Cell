using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// The design tokens shown as they resolve now: the three type roles with the face actually in use, the ramps of
// the theme family and every colour role with its meaning and, for state and encoding roles, the cue that is
// not colour (ADR-D01.47 to ADR-D01.51). A specimen for tuning; it reads the token set and changes nothing.
public sealed partial class TokenSpecimen : ScrollContainer
{
    public int Swatches { get; private set; }
    public IReadOnlyDictionary<string, string> Faces { get; private set; } = new Dictionary<string, string>();
    private readonly VBoxContainer column = new() { SizeFlagsHorizontal = SizeFlags.ExpandFill };
    private static readonly (string Role, string Title, string[] Variations, string Sample)[] Type =
    {
        ("ui", "UI", new[] { "HeadingLabel", "TitleLabel", "", "DimLabel", "MicroLabel" }, "Orbit 3 of 12  ·  1,200 generators"),
        ("content", "Content", new[] { "ContentLabel" }, "A twist moves every sticker of one cell layer; the orbit it belongs to never changes."),
        ("code", "Code", new[] { "CodeLabel" }, "macro m = [R, U] * 3  // 0123456789"),
    };

    public TokenSpecimen()
    {
        Name = "TokenSpecimen"; HorizontalScrollMode = ScrollMode.Disabled;
        var margin = new MarginContainer { SizeFlagsHorizontal = SizeFlags.ExpandFill }; AddChild(margin);
        foreach (string side in new[] { "margin_left", "margin_right", "margin_top", "margin_bottom" }) margin.AddThemeConstantOverride(side, 8);
        margin.AddChild(column);
    }

    // The first face of the role's chain that this machine has; the last one is the fallback of the chain.
    public static string FaceInUse(TokenSet tokens, string role)
    {
        string[] installed = OS.GetSystemFonts();
        IReadOnlyList<string> faces = tokens.System.Type[role].Faces;
        return faces.FirstOrDefault(f => installed.Contains(f, StringComparer.OrdinalIgnoreCase)) ?? faces[^1];
    }

    private void Head(string text) { column.AddChild(new HSeparator()); column.AddChild(ChromeWidgets.Text(text.ToUpperInvariant(), "MicroLabel")); }

    private ColorRect Swatch(TokenSet tokens, Rgb colour, string tooltip)
    {
        int size = tokens.Px("icon.medium"); Swatches++;
        return new ColorRect { Color = new Color((float)colour.R, (float)colour.G, (float)colour.B), CustomMinimumSize = new Vector2(size, size), TooltipText = tooltip,
            SizeFlagsVertical = SizeFlags.ShrinkCenter };
    }

    public void Show(TokenSet tokens)
    {
        foreach (Node child in column.GetChildren()) { column.RemoveChild(child); child.QueueFree(); }
        Swatches = 0;
        var faces = new SortedDictionary<string, string>(StringComparer.Ordinal);
        column.AddChild(ChromeWidgets.Text($"{tokens.Theme.Name}  ·  {tokens.Density}  ·  {tokens.DisplayScale:0.##}x" + (tokens.HighContrast ? "  ·  high contrast" : ""), "AccentLabel"));
        Head("Type roles");
        foreach (var role in Type)
        {
            string wanted = tokens.System.Type[role.Role].Faces[0], used = FaceInUse(tokens, role.Role); faces[role.Role] = used;
            Label name = ChromeWidgets.Text(used == wanted ? $"{role.Title}: {wanted}" : $"{role.Title}: {wanted} is not installed, showing {used}", "DimLabel", true);
            if (used != wanted) name.AddThemeColorOverride("font_color", tokens.C("severity.warning"));
            column.AddChild(name);
            foreach (string variation in role.Variations)
            {
                Label sample = ChromeWidgets.Text(role.Sample, variation); sample.AutowrapMode = TextServer.AutowrapMode.WordSmart;
                sample.CustomMinimumSize = new Vector2(120, 0); column.AddChild(sample);
            }
        }
        Faces = faces;
        Head("Ramps, steps 0 to 11");
        foreach (string ramp in tokens.Theme.Ramps.Keys)
        {
            var row = new HBoxContainer(); row.AddThemeConstantOverride("separation", 1); column.AddChild(row);
            Label name = ChromeWidgets.Text(ramp, "MicroLabel"); name.CustomMinimumSize = new Vector2(52, 0); row.AddChild(name);
            for (int step = 0; step < ThemeFamily.Steps; step++) row.AddChild(Swatch(tokens, tokens.Theme.Step(ramp, step, tokens.Knobs).ToSrgb(), $"{ramp}.{step}"));
        }
        foreach ((string group, string title) in new[] { ("chrome", "Chrome roles"), ("state", "State roles: colour and cue"), ("encoding", "Relationship strip: colour and place"), ("syntax", "Syntax roles") })
        {
            Head(title);
            foreach (ColourRole role in tokens.System.Roles.Values.Where(r => r.Group == group))
            {
                ResolvedColour colour = tokens.Colours[role.Id];
                var row = new HBoxContainer { TooltipText = role.Meaning }; column.AddChild(row);
                row.AddChild(Swatch(tokens, colour.Value, colour.Value.Hex));
                Label name = ChromeWidgets.Text(role.Id + (colour.Overridden ? " (override)" : ""), group == "syntax" ? "CodeLabel" : "");
                if (group != "chrome") name.AddThemeColorOverride("font_color", tokens.C(role.Id));
                name.CustomMinimumSize = new Vector2(104, 0); row.AddChild(name);
                row.AddChild(ChromeWidgets.Text(string.IsNullOrEmpty(role.Cue) ? role.Meaning : role.Cue!, "MicroLabel", true));
            }
        }
    }
}
