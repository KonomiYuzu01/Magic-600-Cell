using System.Collections.Generic;
using System.Linq;

namespace Magic600.Chrome;

// The boundary classes of chapter 07 (BND.27): a built-in tool's declared class is data for the policy and grants
// nothing by itself. The two solving-side exceptions of LB1.5 are each a class of their own (ADR-D01.8): the setup
// search for a named target (ADR-D01.68) and the repair list at a named cap (ADR-D02).
public enum BoundaryClass { Tracking, Analysis, Checking, Protection, ViewAid, SetupSearchNamedTarget, RepairList }

public static class BoundaryClasses
{
    // The class as chapter 07 spells it.
    public static string Word(BoundaryClass c) => c switch
    {
        BoundaryClass.Tracking => "tracking",
        BoundaryClass.Analysis => "analysis",
        BoundaryClass.Checking => "checking",
        BoundaryClass.Protection => "protection",
        BoundaryClass.ViewAid => "view_aid",
        BoundaryClass.SetupSearchNamedTarget => "setup_search_named_target",
        _ => "repair_list",
    };
}

// A tool's PrincipleRecord (LB6.1) as the chrome shows it: the parts a person reads. Every part is text or a typed
// "none, because ..."; an empty part is missing. A sample record says so in its own words and in Sample.
public sealed record PrincipleRecord(string Specification, string Justification, string Instance, string Decomposition,
    string Reason, string NotMeant, bool Sample)
{
    public const string None = "none, because ";

    public IReadOnlyList<string> Missing() => new (string Part, string Text)[]
    {
        ("specification", Specification), ("justification", Justification), ("instance", Instance),
        ("decomposition", Decomposition), ("reason", Reason), ("not meant", NotMeant),
    }.Where(p => string.IsNullOrWhiteSpace(p.Text)).Select(p => p.Part).ToArray();
}

// A built-in tool ships only with its BoundaryClass and its PrincipleRecord (ADR-D01.68; LB6.3: two separate
// obligations). A declaration that lacks either, or whose record lacks a part, refuses every run and names what is
// missing.
public sealed record ToolDeclaration(string Name, BoundaryClass? Class, PrincipleRecord? Principle)
{
    public string? Refusal()
    {
        if (Class == null && Principle == null) return Name + " ships without its BoundaryClass and its PrincipleRecord";
        if (Class == null) return Name + " ships without its BoundaryClass";
        if (Principle == null) return Name + " ships without its PrincipleRecord";
        IReadOnlyList<string> missing = Principle.Missing();
        return missing.Count == 0 ? null : Name + "'s PrincipleRecord lacks " + string.Join(", ", missing);
    }

    // The declaration line a slot shows under its result.
    public string Line() => "Boundary class: " + (Class is { } c ? BoundaryClasses.Word(c) : "none")
        + " · Principle record: " + (Principle == null ? "none" : Principle.Sample ? "sample (" + SolvingDemo.Mark + ")" : "shipped");
}
