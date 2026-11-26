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

// The cap map's panel in Solve's bottom dock, which the entry slot opens (ADR-D02.12). B8 draws the map; until then the
// panel says so, and its legend shows what the entry armed.
public sealed partial class CapMapPanel : VBoxContainer
{
    private readonly ChromeSettings settings;
    private readonly Func<TokenSet> tokens;
    private readonly Label legend;

    public CapMapPanel(ChromeSettings settings, Func<TokenSet> tokens, string title)
    {
        this.settings = settings; this.tokens = tokens; Name = "CapMapPanel";
        Label head = ChromeWidgets.Text(title + ": not drawn in this build yet", "DimLabel"); head.Name = "CapMapState"; AddChild(head);
        legend = ChromeWidgets.Text("", "MicroLabel"); legend.Name = "CapMapLegend"; AddChild(legend);
        settings.Changed += _ => { if (IsInstanceValid(this)) Refresh(); };
        Refresh();
    }

    public void Refresh()
    {
        string Word(string key) => settings.Get(key) == EntrySlot.Armed ? "armed" : "off";
        legend.Text = "contact classes · " + Word(EntrySlot.ContactClasses) + "   never hidden · " + Word(EntrySlot.NeverHidden);
        legend.AddThemeColorOverride("font_color", tokens().C("text.dim"));
    }
}
