using System;
using Godot;

namespace Magic600.Chrome;

// The comparison on the preview view's stage (ADR-D01.10): the committed state beside the state after the whole
// sequence, one row of cells for each orbit. Before, the share of an orbit that will move is outlined; after, it
// is filled. The line under the panes is the scrub: one notch for each move, filled up to where the stage is.
// It draws the two ends only; the state between them is the core's to draw.
public sealed partial class PreviewCompare : Control
{
    public const int Cells = 24;
    private readonly PreviewModel model;
    private readonly Func<TokenSet> tokens;

    public PreviewCompare(PreviewModel model, Func<TokenSet> tokens)
    {
        this.model = model; this.tokens = tokens; Name = "PreviewCompare"; MouseFilter = MouseFilterEnum.Ignore;
        SizeFlagsHorizontal = SizeFlags.ExpandFill; SizeFlagsVertical = SizeFlags.ExpandFill; CustomMinimumSize = new Vector2(0, 96);
        model.Changed += () => { if (IsInstanceValid(this)) QueueRedraw(); };
    }

    public override void _Draw()
    {
        if (model.Pending is not { Status: PreviewStatus.Ready } p) return;
        TokenSet t = tokens(); Font font = GetThemeDefaultFont(); int size = Math.Max(9, GetThemeDefaultFontSize() - 3);
        bool before = model.Side != PreviewSide.After, after = model.Side != PreviewSide.Before; int panes = (before ? 1 : 0) + (after ? 1 : 0);
        float pad = 8, gap = 14, width = (Size.X - pad * 2 - gap * (panes - 1)) / panes, x = pad;
        if (before) { Pane(p, t, font, size, x, width, false); x += width + gap; }
        if (after) Pane(p, t, font, size, x, width, true);
        float y = Size.Y - 8, w = Size.X - pad * 2; int n = Math.Max(1, p.Moves.Count);
        DrawLine(new Vector2(pad, y), new Vector2(pad + w, y), t.C("text.off"), 1);
        DrawLine(new Vector2(pad, y), new Vector2(pad + w * model.Scrub / n, y), t.C("state.preview"), 2);
        for (int i = 0; i <= n; i++)
        {
            float tick = pad + w * i / n;
            DrawLine(new Vector2(tick, y - 3), new Vector2(tick, y + 3), t.C(i <= model.Scrub ? "state.preview" : "text.off"), 1);
        }
    }

    private void Pane(Preview p, TokenSet t, Font font, int size, float x, float width, bool after)
    {
        DrawString(font, new Vector2(x, size + 4), after ? "AFTER ALL" : "BEFORE", HorizontalAlignment.Left, width, size, t.C("text.dim"));
        float top = size + 12, room = Size.Y - top - 20, pitch = Math.Min(22, room / Math.Max(1, p.Effects.Count));
        float wide = (width - (Cells - 1)) / Cells, tall = Math.Min(pitch - 4, 12);
        if (wide < 1.5f || tall < 2) return;
        Color rest = t.C("text.off") with { A = .35f }, moved = t.C("state.preview");
        for (int r = 0; r < p.Effects.Count; r++)
        {
            int lit = PreviewModel.Lit(p.Effects[r], Cells);
            for (int c = 0; c < Cells; c++)
            {
                var rect = new Rect2(x + c * (wide + 1), top + r * pitch, wide, tall);
                if (after && c < lit) { DrawRect(rect, moved); continue; }
                DrawRect(rect, rest);
                if (c < lit) DrawRect(rect, moved, false, 1);
            }
        }
    }
}
