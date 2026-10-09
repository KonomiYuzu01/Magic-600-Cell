using System.Collections.Generic;
using System.Linq;
using Godot;
using LookLab.Core;

namespace LookLab.App.LL5;

internal sealed partial class CostMeter : VBoxContainer
{
    private readonly LookState state;
    private readonly Label values = new() { AutowrapMode = TextServer.AutowrapMode.WordSmart };
    private CostTable table;
    public IReadOnlyList<CostRow> Rows { get; private set; } = System.Array.Empty<CostRow>();
    public string Text => values.Text;
    public string Summary => CostDisplay.Summary(Rows);
    internal bool HandleNull { get; set; } = true;

    public CostMeter(LookState state, CostTable table)
    {
        this.state = state; this.table = table;
        AddChild(new Label { Text = "Full-detail cost table" }); AddChild(values); Refresh();
    }

    public void SetTable(CostTable value) { table = value; Refresh(); }

    public void Refresh()
    {
        Rows = CostDisplay.Rows(state.Schema, state.Preset.Params, table, HandleNull);
        values.Text = string.Join("\n", Rows.Select(row => row.Text));
    }
}
