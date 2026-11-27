using System;
using System.Linq;
using Magic600.Chrome;

internal static partial class Program
{
    // ADR-D01.65: the rail carries the locks, and the protection kinds (whole orbit, positions, identities, a block) are
    // distinct marks with their own words. A whole-orbit lock covers the others; no lock is ever called "partial" or "safe".
    private static void D01RailLocksNameTheirKinds()
    {
        Equal(Protections.Lock(ProtectionKinds.None), OrbitLock.None, "nothing protected");
        Equal(Protections.Lock(ProtectionKinds.Orbit), OrbitLock.Whole, "the whole orbit");
        Equal(Protections.Lock(ProtectionKinds.Orbit | ProtectionKinds.Block), OrbitLock.Whole, "the whole orbit covers a block");
        foreach (ProtectionKinds kind in Protections.PartialKinds) Equal(Protections.Lock(kind), OrbitLock.Partial, kind + " alone");
        Equal(Protections.Words(ProtectionKinds.None), "none", "no lock");
        Equal(Protections.Words(ProtectionKinds.Orbit), "whole orbit", "a whole-orbit lock");
        Equal(Protections.Words(ProtectionKinds.Orbit | ProtectionKinds.Position), "whole orbit", "the whole orbit names no part");
        Equal(Protections.Words(ProtectionKinds.Position), "positions", "a position lock");
        Equal(Protections.Words(ProtectionKinds.Identity), "identities", "an identity lock");
        Equal(Protections.Words(ProtectionKinds.Block), "a block", "a block lock");
        Equal(Protections.Words(ProtectionKinds.Block | ProtectionKinds.Position), "positions, a block", "two kinds, in a fixed order");
        Equal(Protections.Words(ProtectionKinds.Position | ProtectionKinds.Identity | ProtectionKinds.Block), "positions, identities, a block", "three kinds");
        // Every combination has its own words, so no two kinds share a mark.
        var all = Enumerable.Range(0, 16).Select(i => (ProtectionKinds)i).ToArray();
        Equal(all.Where(k => !k.HasFlag(ProtectionKinds.Orbit)).Select(Protections.Words).Distinct().Count(), 8, "each partial combination has its own words");
        Check(all.All(k => !Protections.Words(k).Contains("partial") && !Protections.Words(k).Contains("safe")), "no lock reads partial or safe");

        // The sample mid-solve state holds every kind, solved or not, and the rail's cells carry them.
        var demo = new SolvingDemo(Atlas);
        var cells = OrbitRailModel.Cells(demo.State, false);
        foreach (ProtectionKinds kind in new[] { ProtectionKinds.Orbit, ProtectionKinds.Position, ProtectionKinds.Identity, ProtectionKinds.Block })
            Check(cells.Any(c => c.Protection.HasFlag(kind)), kind + " is on the sample rail");
        Check(cells.All(c => c.Lock == Protections.Lock(c.Protection) && c.Protection == demo.State.Residuals[c.Orbit].Protection), "a cell's lock is its residual's protection");
        RailCell working = cells.Single(c => c.Working);
        Check(!working.Solved && working.Lock == OrbitLock.Partial && working.Protection == (ProtectionKinds.Position | ProtectionKinds.Block), "the working orbit locks positions and a block");
    }
}
