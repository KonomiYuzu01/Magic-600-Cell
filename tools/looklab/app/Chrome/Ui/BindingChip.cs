using Godot;

namespace Magic600.Chrome;

// The binding chip on a free view's head (ADR-D01.42): it names the view's source in a word or two, with the sentence
// as its tooltip. The screen text reads its words under "binding:<pane>", apart from the pane's own words.
public sealed partial class BindingChip : PanelContainer
{
    public string PaneId { get; }
    public ViewBinding Binding { get; }

    public BindingChip(string paneId, ViewBinding binding)
    {
        PaneId = paneId; Binding = binding;
        Name = "BindingChip"; ThemeTypeVariation = "Chip"; TooltipText = binding.Tip;
        MouseFilter = MouseFilterEnum.Stop; SizeFlagsVertical = SizeFlags.ShrinkCenter;
        AddChild(ChromeWidgets.Text(binding.Words, "MicroLabel"));
    }
}
