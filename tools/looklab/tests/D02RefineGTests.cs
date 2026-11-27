using System;

internal static partial class Program
{
    private static void D02CorrectionRowsRevealWithoutTakingFocus()
    {
        string host = Read("app/Chrome/Ui/JumbleHost.cs");
        Check(host.Contains("Corrections.Model?.Mark(id, by); RevealCorrections(); break;", StringComparison.Ordinal),
            "marking from the palette must leave Space available for the next palette line");
        Check(host.Contains("case \"jumble.corrections.check\": Corrections.CheckSet(); RevealCorrections(); break;", StringComparison.Ordinal),
            "checking from the palette must leave Space available for the next palette line");
        Check(host.Contains("private void RevealCorrections() => frame.Model.RevealDock(DockSide.Left, NuCorrectionsPanel.PanelId, frame.Folds(DockSide.Left));",
            StringComparison.Ordinal), "the result must reveal the dock without focusing the correction list");
    }
}
