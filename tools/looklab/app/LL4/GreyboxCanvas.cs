using System;
using System.Collections.Generic;
using System.Linq;
using Godot;
using LookLab.Core;

namespace LookLab.App.LL4;

internal sealed partial class GreyboxCanvas : Control
{
    private readonly CommandCatalogue catalogue;
    private readonly Dictionary<string, Button> commands = new(StringComparer.Ordinal);
    private readonly Dictionary<string, PanelContainer> regions = new(StringComparer.Ordinal);
    private LayoutSpec? layout;
    public string Context { get; private set; } = "";
    public string? HighlightedCommand { get; private set; }
    public IReadOnlyDictionary<string, Button> CommandControls => commands;
    public IReadOnlyDictionary<string, PanelContainer> RegionControls => regions;
    internal bool CatalogueFilterEnabled { get; set; } = true;

    public GreyboxCanvas(CommandCatalogue catalogue)
    {
        this.catalogue = catalogue;
        ClipContents = true;
        Resized += Arrange;
    }

    public void ShowContext(LayoutSpec spec, string context)
    {
        foreach (Node child in GetChildren()) { RemoveChild(child); child.QueueFree(); }
        commands.Clear(); regions.Clear(); HighlightedCommand = null;
        layout = spec; Context = context;
        var columns = new Dictionary<string, VBoxContainer>(StringComparer.Ordinal);
        foreach (LayoutRegion region in spec.Regions.Values.Where(r => r.Contexts.Contains(context))
            .OrderBy(r => r.Placement == "overlay" ? 1 : 0))
        {
            var box = new PanelContainer { ClipContents = true };
            var style = new StyleBoxFlat { BgColor = region.Kind == "stage" ? new Color(.19f, .19f, .19f) : new Color(.30f, .30f, .30f),
                BorderColor = new Color(.6f, .6f, .6f) };
            style.SetBorderWidthAll(1); box.AddThemeStyleboxOverride("panel", style);
            AddChild(box); regions.Add(region.Id, box);
            var scroll = new ScrollContainer { HorizontalScrollMode = ScrollContainer.ScrollMode.Auto };
            box.AddChild(scroll);
            var column = new VBoxContainer { SizeFlagsHorizontal = SizeFlags.ExpandFill };
            scroll.AddChild(column); columns.Add(region.Id, column);
            column.AddChild(new Label { Text = region.Id + " (" + region.Placement + ")", TooltipText = region.Kind,
                HorizontalAlignment = region.Kind == "stage" ? HorizontalAlignment.Center : HorizontalAlignment.Left });
        }
        var candidates = catalogue.Commands.Values.Where(c => !CatalogueFilterEnabled || c.Contexts.Contains(context));
        foreach (CommandDefinition command in candidates)
        {
            if (!spec.Contexts[context].Placed.TryGetValue(command.Id, out string? region))
            {
                if (CatalogueFilterEnabled) continue; // A layout's hidden commands stay hidden.
                region = spec.Regions.Values.First(r => r.Kind == "stage" && r.Contexts.Contains(context)).Id;
            }
            var button = new Button { Text = command.Id, TooltipText = command.Id + " in " + context,
                Alignment = spec.Regions[region].Kind == "stage" ? HorizontalAlignment.Center : HorizontalAlignment.Left };
            button.SetMeta("ll4_command", command.Id); button.SetMeta("ll4_region", region);
            columns[region].AddChild(button); commands.Add(command.Id, button);
            button.Pressed += () => Highlight(command.Id);
        }
        Arrange();
    }

    public void Highlight(string? command)
    {
        HighlightedCommand = command;
        foreach (var entry in commands)
            entry.Value.Modulate = entry.Key == command ? new Color(1, .85f, .4f) : Colors.White;
        if (command != null && commands.TryGetValue(command, out Button? target))
        {
            var scroll = (ScrollContainer)target.GetParent().GetParent();
            scroll.EnsureControlVisible(target);
        }
    }

    private void Arrange()
    {
        if (layout == null) return;
        foreach (var entry in regions)
        {
            RegionRect rect = layout.Regions[entry.Key].Rect;
            entry.Value.Position = new Vector2((float)rect.X * Size.X, (float)rect.Y * Size.Y);
            // Containers' content minima must not expand the fractional region.
            entry.Value.Size = new Vector2((float)rect.Width * Size.X, (float)rect.Height * Size.Y);
        }
    }
}
