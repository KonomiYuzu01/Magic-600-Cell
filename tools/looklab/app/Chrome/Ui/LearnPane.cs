using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

public sealed partial class LearnPane : HBoxContainer
{
    private readonly LearnModel model;
    private readonly Dictionary<string, Button> terms = new(StringComparer.Ordinal);
    private readonly Label title = new() { Name = "LearnTitle", ThemeTypeVariation = "TitleLabel", AutowrapMode = TextServer.AutowrapMode.WordSmart };
    private readonly Label levelName = new() { ThemeTypeVariation = "MicroLabel", AutowrapMode = TextServer.AutowrapMode.WordSmart };
    private readonly Label definition = new() { ThemeTypeVariation = "ContentLabel", AutowrapMode = TextServer.AutowrapMode.WordSmart };
    private readonly VBoxContainer links = new();
    private readonly ScrollContainer reading;
    private readonly HelpFigurePlate figure;

    public LearnPane(LearnModel model, Func<TokenSet> tokens)
    {
        Name = "LearnPane"; this.model = model;
        var directory = Scroll(1); AddChild(directory);
        var list = new VBoxContainer { SizeFlagsHorizontal = SizeFlags.ExpandFill }; directory.AddChild(list);
        foreach (HelpLevel level in model.Levels)
        {
            Label heading = ChromeWidgets.Text(level.Name, "HeadingLabel");
            heading.AutowrapMode = TextServer.AutowrapMode.WordSmart; list.AddChild(heading);
            foreach (HelpTopic term in level.Terms)
            {
                Button button = TermButton(term.Id, term.Title, true);
                list.AddChild(button); terms.Add(term.Id, button);
            }
        }
        AddChild(new VSeparator());
        reading = Scroll(2); AddChild(reading);
        var content = new VBoxContainer { SizeFlagsHorizontal = SizeFlags.ExpandFill }; reading.AddChild(content);
        content.AddChild(title); content.AddChild(levelName); content.AddChild(definition);
        figure = new HelpFigurePlate(tokens); content.AddChild(figure); content.AddChild(links);
        Refresh();
    }

    private static ScrollContainer Scroll(float ratio) => new()
    {
        HorizontalScrollMode = ScrollContainer.ScrollMode.Disabled,
        SizeFlagsHorizontal = SizeFlags.ExpandFill, SizeFlagsStretchRatio = ratio,
    };

    private Button TermButton(string id, string text, bool toggle = false)
    {
        Button button = ChromeWidgets.Tool(text, text, toggle);
        button.Alignment = HorizontalAlignment.Left; button.ClipText = true;
        button.Pressed += () => Select(id);
        return button;
    }

    public bool Select(string id)
    {
        if (!model.Select(id)) return false;
        Refresh(); reading.ScrollVertical = 0;
        return true;
    }

    private void Refresh()
    {
        HelpReading? current = model.Selected;
        foreach (var (id, button) in terms) button.SetPressedNoSignal(id == current?.Topic.Id);
        title.Text = current?.Topic.Title ?? LearnModel.ChooseTerm;
        levelName.Text = model.LevelName; levelName.Visible = levelName.Text.Length > 0;
        definition.Text = current?.Body ?? ""; definition.Visible = definition.Text.Length > 0;
        HelpFigure? plate = model.Figure;
        figure.Show(plate, plate?.Caption ?? current?.FigureLine ?? "");
        foreach (Node child in links.GetChildren()) { links.RemoveChild(child); child.QueueFree(); }
        if (current != null)
            foreach (string id in current.Topic.LinkedTermIds.Distinct()) links.AddChild(TermButton(id, terms[id].Text));
    }
}
