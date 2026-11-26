using System;
using System.Collections.Generic;
using Godot;

namespace Magic600.Chrome;

// Local's Jumble content (ADR-D02.6, .12): the empty sentence while no cap is named, then the four sections, one open
// at a time, each closed one a one-line header with its count. The cut slice (B20) and the raw view (B21) draw later;
// until then Local shows this. Every section is empty until its build step fills it.
public sealed partial class JumbleSurface : VBoxContainer
{
    public LocalAccordion Accordion { get; }
    private readonly Func<TokenSet> tokens;
    private readonly IReadOnlyDictionary<string, string> names;
    private readonly string emptyBody;
    private readonly Label sentence;
    private readonly VBoxContainer sections = new() { Name = "LocalSections" };

    public JumbleSurface(LocalAccordion accordion, Func<TokenSet> tokens, IReadOnlyDictionary<string, string> names, string emptySentence, string emptyBody)
    {
        Accordion = accordion; this.tokens = tokens; this.names = names; this.emptyBody = emptyBody;
        Name = "JumbleLocal"; SizeFlagsHorizontal = SizeFlags.ExpandFill; SizeFlagsVertical = SizeFlags.ExpandFill;
        sentence = ChromeWidgets.Text(emptySentence, "DimLabel"); sentence.Name = "LocalEmpty";
        sentence.AutowrapMode = TextServer.AutowrapMode.WordSmart; sentence.HorizontalAlignment = HorizontalAlignment.Center;
        AddChild(sentence); AddChild(sections);
        accordion.Changed += () => { if (IsInstanceValid(this)) Refresh(); };
        Refresh();
    }

    public void Refresh()
    {
        TokenSet t = tokens();
        AddThemeConstantOverride("separation", t.Px("space.2"));
        foreach (Node child in sections.GetChildren()) { sections.RemoveChild(child); child.QueueFree(); }
        foreach (string section in Accordion.Sections)
        {
            bool open = Accordion.Open == section;
            Button header = ChromeWidgets.Tool((open ? "▾ " : "▸ ") + Accordion.Header(section, names), open ? "Close this section" : "Open this section; the open one closes", true);
            header.Name = "LocalSection_" + section; header.Alignment = HorizontalAlignment.Left; header.SetPressedNoSignal(open);
            string id = section; header.Pressed += () => Accordion.Toggle(id);
            sections.AddChild(header);
            if (open)
            {
                Label body = ChromeWidgets.Text(emptyBody, "DimLabel"); body.Name = "LocalBody_" + section;
                body.AddThemeColorOverride("font_color", t.C("text.dim")); sections.AddChild(body);
            }
        }
    }
}

// The cap map's panel in Solve's bottom dock, which the entry slot opens (ADR-D02.12, .13): its head, the map (B8),
// the legend of the classes with their counts, and the legend of what the entry armed.
public sealed partial class CapMapPanel : VBoxContainer
{
    public CapMapControl Map { get; }
    private readonly ChromeSettings settings;
    private readonly Func<TokenSet> tokens;
    private readonly string title;
    private readonly Label head, legend;
    private readonly HFlowContainer classes = new() { Name = "CapMapClasses" };

    public CapMapPanel(ChromeSettings settings, Func<TokenSet> tokens, string title)
    {
        this.settings = settings; this.tokens = tokens; this.title = title; Name = "CapMapPanel";
        head = ChromeWidgets.Text(title, "DimLabel"); head.Name = "CapMapState"; AddChild(head);
        Map = new CapMapControl(tokens); AddChild(Map);
        AddChild(classes);
        legend = ChromeWidgets.Text("", "MicroLabel"); legend.Name = "CapMapLegend"; AddChild(legend);
        settings.Changed += _ => { if (IsInstanceValid(this)) Refresh(); };
        Refresh();
    }

    // The bound state's map, or every cap pending while none is bound; no layout, no sheet.
    public void Show(CapLayout? layout, CapMapModel map)
    {
        Map.Show(layout, map);
        Refresh();
    }

    public void Refresh()
    {
        TokenSet t = tokens();
        head.Text = Map.Layout == null ? title + ": no cap layout in this build" : title + " · 20 rings of 30 caps";
        string Word(string key) => settings.Get(key) == EntrySlot.Armed ? "armed" : "off";
        legend.Text = "contact classes · " + Word(EntrySlot.ContactClasses) + "   never hidden · " + Word(EntrySlot.NeverHidden);
        legend.AddThemeColorOverride("font_color", t.C("text.dim"));
        foreach (Node child in classes.GetChildren()) { classes.RemoveChild(child); child.QueueFree(); }
        if (Map.Layout == null) return;
        classes.AddThemeConstantOverride("h_separation", t.Px("space.3"));
        IReadOnlyDictionary<CapClass, int> counts = Map.Map.Counts();
        foreach (CapClass k in new[] { CapClass.Blocked, CapClass.Closed, CapClass.AtFloor, CapClass.AboveFloor, CapClass.Pending })
        {
            string word = k == CapClass.AtFloor ? "at floor (N_c = F_c > 0)" : CapMapModel.Word(k);
            Entry(new CapSwatch(tokens, k), word + " · " + counts[k], "CapMapClass_" + k);
        }
        Entry(new CapSwatch(tokens, null), "columns wrap", "CapMapWrap");
        Map.Restyle();

        void Entry(Control swatch, string text, string name)
        {
            var row = new HBoxContainer { Name = name }; row.AddThemeConstantOverride("separation", t.Px("space.1"));
            row.AddChild(swatch);
            Label label = ChromeWidgets.Text(text, "MicroLabel"); label.AddThemeColorOverride("font_color", t.C("text.dim")); row.AddChild(label);
            classes.AddChild(row);
        }
    }
}
