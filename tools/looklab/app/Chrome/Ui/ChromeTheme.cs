using System;
using System.Linq;
using Godot;

namespace Magic600.Chrome;

// The Godot Theme generated at run time from one resolved token set (ADR-D01.33). Product code and Look Lab
// name tokens only; no colour, size or face is written anywhere else.
public static class ChromeTheme
{
    public static Color C(this TokenSet t, string token, float alpha = 1)
    {
        Rgb c = t.Colour(token);
        return new Color((float)c.R, (float)c.G, (float)c.B, alpha);
    }

    public static int Px(this TokenSet t, string token) => (int)t.Size(token);

    public static Font Face(TokenSet t, string role, bool bold = false)
    {
        TypeRole type = t.System.Type[role];
        if (ChromeFonts.Face(type, bold) is { } installed)
            return type.Tabular ? new FontVariation { BaseFont = installed, OpentypeFeatures = new Godot.Collections.Dictionary { { TextServerManager.GetPrimaryInterface().NameToTag("tnum"), 1 } } } : installed;
        var system = new SystemFont { FontNames = type.Faces.ToArray(), FontWeight = bold ? 600 : 400, AllowSystemFallback = true,
            Antialiasing = TextServer.FontAntialiasing.Gray, Hinting = TextServer.Hinting.Light, SubpixelPositioning = TextServer.SubpixelPositioning.Auto };
        if (!type.Tabular) return system;
        // Number fields use tabular figures (ADR-D01.47).
        var features = new Godot.Collections.Dictionary { { TextServerManager.GetPrimaryInterface().NameToTag("tnum"), 1 } };
        return new FontVariation { BaseFont = system, OpentypeFeatures = features };
    }

    public static StyleBoxFlat Box(TokenSet t, string background, string? border = null, string radius = "radius.control", int pad = -1, int borderWidth = -1)
    {
        var box = new StyleBoxFlat { BgColor = t.C(background), AntiAliasing = false };
        int width = border == null ? 0 : borderWidth >= 0 ? borderWidth : t.Px("border.width");
        box.SetBorderWidthAll(width);
        if (border != null) box.BorderColor = t.C(border);
        box.SetCornerRadiusAll(t.Px(radius));
        int p = pad >= 0 ? pad : t.Px("space.2");
        box.ContentMarginLeft = p + t.Px("space.1"); box.ContentMarginRight = p + t.Px("space.1"); box.ContentMarginTop = p; box.ContentMarginBottom = p;
        return box;
    }

    private static StyleBoxFlat Edge(TokenSet t, string background, string border, Side side)
    {
        var box = new StyleBoxFlat { BgColor = t.C(background), BorderColor = t.C(border), AntiAliasing = false };
        box.SetBorderWidth(side, t.Px("border.width"));
        box.SetContentMarginAll(0);
        return box;
    }

    public static Theme Build(TokenSet t)
    {
        ChromeIcons.EmptySize = t.Px("icon.large") * 2;
        var theme = new Theme { DefaultFont = Face(t, "ui"), DefaultFontSize = (int)t.TypeSize("ui", "body") };
        int s1 = t.Px("space.1"), s2 = t.Px("space.2"), s3 = t.Px("space.3"), control = t.Px("control.height");
        Color text = t.C("text"), dim = t.C("text.dim"), off = t.C("text.off");

        // Borders before shadows, no nested cards: a panel is a flat surface with one line.
        theme.SetStylebox("panel", "Panel", Box(t, "panel.background", null, "radius.panel", 0));
        theme.SetStylebox("panel", "PanelContainer", Box(t, "panel.background", null, "radius.panel", 0));
        foreach ((string name, Side side) in new[] { ("BarTop", Side.Bottom), ("BarBottom", Side.Top), ("DockLeft", Side.Right), ("DockRight", Side.Left), ("DockBottom", Side.Top) })
        {
            theme.SetTypeVariation(name, "PanelContainer");
            theme.SetStylebox("panel", name, Edge(t, "panel.background", "panel.border", side));
        }
        theme.SetTypeVariation("FrameGround", "PanelContainer"); theme.SetStylebox("panel", "FrameGround", Box(t, "window.background", null, "radius.panel", 0));
        theme.SetTypeVariation("ViewFrame", "PanelContainer"); theme.SetStylebox("panel", "ViewFrame", Box(t, "window.background", "panel.border", "radius.panel", 0));
        theme.SetTypeVariation("ViewFrameFocus", "PanelContainer"); theme.SetStylebox("panel", "ViewFrameFocus", Box(t, "window.background", "control.focus", "radius.panel", 0));
        theme.SetTypeVariation("Chip", "PanelContainer"); theme.SetStylebox("panel", "Chip", Box(t, "panel.background", "panel.border", "radius.control", s1));
        theme.SetTypeVariation("Row", "PanelContainer"); theme.SetStylebox("panel", "Row", Box(t, "popup.background", null, "radius.control", s1));
        var rowOn = Box(t, "control.pressed", null, "radius.control", s1); rowOn.BorderColor = t.C("accent.base"); rowOn.SetBorderWidth(Side.Left, t.Px("focus.width"));
        theme.SetTypeVariation("RowSelected", "PanelContainer"); theme.SetStylebox("panel", "RowSelected", rowOn);
        theme.SetTypeVariation("OverlayBox", "PanelContainer"); theme.SetStylebox("panel", "OverlayBox", Box(t, "popup.background", "popup.border", "radius.control", s3));
        // Every script-drawn area carries this frame (ADR-D01.4 item 4).
        theme.SetTypeVariation("ScriptFrame", "PanelContainer"); theme.SetStylebox("panel", "ScriptFrame", Box(t, "panel.background", "script.frame", "radius.panel", s2, t.Px("focus.width")));

        foreach (string type in new[] { "Button", "OptionButton", "MenuButton", "CheckButton", "CheckBox" })
        {
            bool flat = type is "CheckButton" or "CheckBox";
            theme.SetStylebox("normal", type, flat ? new StyleBoxEmpty() : Box(t, "control.background", "control.border"));
            theme.SetStylebox("hover", type, flat ? new StyleBoxEmpty() : Box(t, "control.hover", "control.border"));
            theme.SetStylebox("pressed", type, flat ? new StyleBoxEmpty() : Box(t, "control.pressed", "accent.base"));
            theme.SetStylebox("hover_pressed", type, flat ? new StyleBoxEmpty() : Box(t, "control.pressed", "accent.base"));
            theme.SetStylebox("disabled", type, flat ? new StyleBoxEmpty() : Box(t, "panel.background", "control.border"));
            var focus = Box(t, "panel.background", "control.focus", "radius.control", -1, t.Px("focus.width")); focus.DrawCenter = false;
            theme.SetStylebox("focus", type, focus);
            theme.SetColor("font_color", type, text); theme.SetColor("font_hover_color", type, text); theme.SetColor("font_pressed_color", type, text);
            theme.SetColor("font_hover_pressed_color", type, text); theme.SetColor("font_focus_color", type, text); theme.SetColor("font_disabled_color", type, off);
            theme.SetConstant("h_separation", type, s2);
        }
        // A tool button: no box at rest, the accent line when it is the active choice.
        foreach (string name in new[] { "ToolButton", "TabButton" })
        {
            theme.SetTypeVariation(name, "Button");
            var rest = new StyleBoxFlat { DrawCenter = false }; rest.SetContentMarginAll(s2); rest.ContentMarginLeft = s3; rest.ContentMarginRight = s3;
            var hover = Box(t, "control.hover", null); hover.ContentMarginLeft = s3; hover.ContentMarginRight = s3;
            var on = new StyleBoxFlat { BgColor = t.C("control.background"), BorderColor = t.C("accent.base"), AntiAliasing = false };
            on.SetContentMarginAll(s2); on.ContentMarginLeft = s3; on.ContentMarginRight = s3;
            on.SetBorderWidth(name == "TabButton" ? Side.Bottom : Side.Left, t.Px("focus.width"));
            theme.SetStylebox("normal", name, rest); theme.SetStylebox("hover", name, hover); theme.SetStylebox("pressed", name, on);
            theme.SetStylebox("hover_pressed", name, on); theme.SetStylebox("disabled", name, rest);
            theme.SetColor("font_color", name, dim); theme.SetColor("font_pressed_color", name, text); theme.SetColor("font_hover_color", name, text);
        }

        // Inspector fields: a caption that resets on a click, and a value that is dragged or typed.
        foreach (string name in new[] { "FieldCaption", "FieldFact", "FieldValue" })
        {
            theme.SetTypeVariation(name, "Button");
            StyleBoxFlat rest = name == "FieldValue" ? Box(t, "field.background", null, "radius.control", s1) : new StyleBoxFlat { DrawCenter = false };
            if (name != "FieldValue") rest.SetContentMarginAll(s1);
            StyleBoxFlat over = name == "FieldFact" ? rest : Box(t, "control.hover", null, "radius.control", s1);
            theme.SetStylebox("normal", name, rest); theme.SetStylebox("hover", name, over); theme.SetStylebox("pressed", name, over); theme.SetStylebox("disabled", name, rest);
            theme.SetColor("font_color", name, name == "FieldCaption" ? dim : text); theme.SetColor("font_hover_color", name, text); theme.SetColor("font_pressed_color", name, text);
        }

        // An entry of a radial menu: a raised box that takes the accent border under the pointer.
        theme.SetTypeVariation("RadialButton", "Button");
        theme.SetStylebox("normal", "RadialButton", Box(t, "popup.background", "panel.border", "radius.control", s2));
        theme.SetStylebox("hover", "RadialButton", Box(t, "control.hover", "accent.base", "radius.control", s2));
        theme.SetStylebox("pressed", "RadialButton", Box(t, "control.pressed", "accent.base", "radius.control", s2));

        foreach (string type in new[] { "LineEdit", "TextEdit", "CodeEdit" })
        {
            theme.SetStylebox("normal", type, Box(t, "field.background", "control.border"));
            theme.SetStylebox("focus", type, Box(t, "field.background", "control.focus"));
            theme.SetStylebox("read_only", type, Box(t, "panel.background", "control.border"));
            theme.SetColor("font_color", type, text); theme.SetColor("font_placeholder_color", type, off); theme.SetColor("caret_color", type, t.C("accent.strong"));
            theme.SetColor("selection_color", type, t.C("selection.background", .45f)); theme.SetColor("font_uneditable_color", type, dim);
        }
        foreach (string type in new[] { "TextEdit", "CodeEdit" })
        {
            theme.SetFont("font", type, Face(t, "code")); theme.SetFontSize("font_size", type, (int)t.TypeSize("code", "body"));
            theme.SetColor("background_color", type, t.C("field.background")); theme.SetColor("current_line_color", type, t.C("control.background"));
            theme.SetColor("line_number_color", type, off);
        }
        theme.SetConstant("minimum_character_width", "LineEdit", 4);

        theme.SetColor("font_color", "Label", text);
        foreach ((string name, string colour, string role, string step, bool bold) in new[] {
            ("DimLabel", "text.dim", "ui", "small", false), ("MicroLabel", "text.dim", "ui", "micro", false), ("TitleLabel", "text", "ui", "title", true),
            ("HeadingLabel", "text", "ui", "heading", true), ("ContentLabel", "text", "content", "body", false), ("CodeLabel", "text", "code", "body", false),
            ("AccentLabel", "accent.strong", "ui", "small", true) })
        {
            theme.SetTypeVariation(name, "Label"); theme.SetColor("font_color", name, t.C(colour));
            theme.SetFont("font", name, Face(t, role, bold)); theme.SetFontSize("font_size", name, (int)t.TypeSize(role, step));
        }

        var slider = new StyleBoxFlat { BgColor = t.C("field.background"), BorderColor = t.C("control.border"), AntiAliasing = false };
        slider.SetBorderWidthAll(t.Px("border.width")); slider.ContentMarginTop = t.Px("slider.height") / 2f; slider.ContentMarginBottom = t.Px("slider.height") / 2f;
        var fill = new StyleBoxFlat { BgColor = t.C("slider.fill"), AntiAliasing = false };
        foreach (string type in new[] { "HSlider", "VSlider" })
        {
            theme.SetStylebox("slider", type, slider); theme.SetStylebox("grabber_area", type, fill); theme.SetStylebox("grabber_area_highlight", type, fill);
        }
        theme.SetStylebox("background", "ProgressBar", slider); theme.SetStylebox("fill", "ProgressBar", fill);

        var grabber = new StyleBoxFlat { BgColor = t.C("scrollbar.grabber"), AntiAliasing = false };
        grabber.SetContentMarginAll(t.Px("scrollbar.width") / 2f);
        foreach (string type in new[] { "HScrollBar", "VScrollBar" })
        {
            theme.SetStylebox("scroll", type, new StyleBoxEmpty()); theme.SetStylebox("grabber", type, grabber);
            theme.SetStylebox("grabber_highlight", type, grabber); theme.SetStylebox("grabber_pressed", type, fill);
        }

        var line = new StyleBoxLine { Color = t.C("separator"), Thickness = t.Px("border.width") };
        theme.SetStylebox("separator", "HSeparator", line); theme.SetConstant("separation", "HSeparator", s2);
        theme.SetStylebox("separator", "VSeparator", new StyleBoxLine { Color = t.C("separator"), Thickness = t.Px("border.width"), Vertical = true });
        theme.SetConstant("separation", "VSeparator", s2);
        foreach (string type in new[] { "HSplitContainer", "VSplitContainer" })
        {
            theme.SetConstant("separation", type, t.Px("border.width")); theme.SetConstant("minimum_grab_thickness", type, s3);
            theme.SetConstant("autohide", type, 1);
            theme.SetStylebox("split_bar_background", type, new StyleBoxFlat { BgColor = t.C("panel.border") });
        }
        theme.SetConstant("separation", "VBoxContainer", s2); theme.SetConstant("separation", "HBoxContainer", s2);
        foreach (string side in new[] { "margin_left", "margin_right", "margin_top", "margin_bottom" }) theme.SetConstant(side, "MarginContainer", s3);

        foreach (string type in new[] { "PopupMenu", "PopupPanel", "TooltipPanel" })
            theme.SetStylebox("panel", type, Box(t, type == "TooltipPanel" ? "tooltip.background" : "popup.background", "popup.border", "radius.control", s2));
        theme.SetStylebox("hover", "PopupMenu", Box(t, "selection.background", null, "radius.control", 0));
        theme.SetColor("font_color", "PopupMenu", text); theme.SetColor("font_hover_color", "PopupMenu", t.C("selection.text"));
        theme.SetColor("font_color", "TooltipLabel", text);
        foreach (string type in new[] { "Tree", "ItemList" })
        {
            theme.SetStylebox("panel", type, Box(t, "panel.background", null, "radius.panel", 0));
            theme.SetStylebox("selected", type, Box(t, "selection.background", null, "radius.control", 0));
            theme.SetStylebox("selected_focus", type, Box(t, "selection.background", null, "radius.control", 0));
            theme.SetStylebox("cursor", type, new StyleBoxEmpty()); theme.SetStylebox("cursor_unfocused", type, new StyleBoxEmpty());
            theme.SetStylebox("hovered", type, Box(t, "control.hover", null, "radius.control", 0));
            theme.SetColor("font_color", type, text); theme.SetColor("font_selected_color", type, t.C("selection.text"));
            theme.SetColor("guide_color", type, t.C("separator", .5f));
            theme.SetConstant("v_separation", type, Math.Max(0, t.Px("row.height") - (int)t.TypeSize("ui", "body") - s2));
        }
        theme.SetConstant("minimum_height", "ControlRow", control);
        return theme;
    }
}
