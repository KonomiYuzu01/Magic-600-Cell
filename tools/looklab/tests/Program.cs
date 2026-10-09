using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Threading.Tasks;
using LookLab.Core;

internal sealed class SkipException : Exception
{
    public SkipException(string message) : base(message) { }
}

internal sealed class TestFolder : IDisposable
{
    public string Path { get; } = System.IO.Path.Combine(System.IO.Path.GetTempPath(), "looklab-test-" + Guid.NewGuid().ToString("N"));
    public TestFolder() => Directory.CreateDirectory(Path);
    public void Dispose() => Directory.Delete(Path, true);
}

internal static partial class Program
{
    private static string Root = "", Lab = "";
    private static Geometry? geometry;
    private static TurnData? turn;
    private static ParameterSchema? schema;
    private static CommandCatalogue? commands;
    private static Geometry Mesh => geometry ??= new Geometry(Root);
    private static TurnData Turn => turn ??= new TurnData(FileAt("fixtures/w3-turn.json"));
    private static ParameterSchema Schema => schema ??= ParameterSchema.Load(FileAt("data/parameters.json"));
    private static CommandCatalogue Commands => commands ??= CommandCatalogue.Load(System.IO.Path.Combine(Root, "docs/progress/1.0/command-table.json"));
    private static string FileAt(string path) => System.IO.Path.Combine(Lab, path.Replace('/', System.IO.Path.DirectorySeparatorChar));
    private static string Read(string path) => File.ReadAllText(FileAt(path));

    private static int Main(string[] args)
    {
        if (args.Length != 1) { Console.Error.WriteLine("LookLab.Tests: expected repository root"); return 2; }
        Root = System.IO.Path.GetFullPath(args[0]); Lab = System.IO.Path.Combine(Root, "tools/looklab");
        var tests = new List<(string Name, Action Run)>
        {
            ("HarnessIsolationAndRefusals", LL5HarnessTests),
            ("LL5CvdPreviewCoefficientsAgainstCore", LL5CvdReference),
            ("LL5DrawingFragmentProbeAndWrongMatrixBinding", LL5ShaderProbe),
            ("LL5PaletteValuesCallerMarksAndGamutReasons", LL5PalettePresentation),
            ("LL5CostTableNullMeasuredSourceAndMappings", LL5CostPresentation),
            ("OfflineBuildContract", BuildContract),
            ("ManifestMissingFileEntryAndMismatch", ManifestFailures),
            ("NpyDtypeShapeAndOrderRefusals", NpyRefusals),
            ("SbReferenceAll2700Samples", SbReference),
            ("VerifiedUploadInputsReproduce2700Projections", UploadInputs),
            ("LookStateValidatedSetterAndTransactionalLoad", LookStateValidation),
            ("PresetPrivatePathBoundaryAndFirstSave", PresetPathBoundary),
            ("DrawingShaderBodiesAndBindingPacking", ShaderAdapter),
            ("DrawingShaderAdapterRefusals", ShaderAdapterRefusals),
            ("LL3SeededCandidatesAndReplay", LL3Candidates),
            ("LL3SwipeRecordsAndPromotion", LL3SwipeRecords),
            ("LL3CompareSharedCameraAndClock", LL3Compare),
            ("LL3CaptureBoundsEncodingAndGalleryCopy", LL3CaptureRules),
            ("LL3CaptureValidationAndGalleryScanner", LL3PythonChecks),
            ("FixtureProvenanceAndTurnDigest", FixtureProvenance),
            ("SbFixtureRequiredInputAndPortableOutput", SbFixturePortable),
            ("ProjectionKindsAndCameraOrder", ProjectionKinds),
            ("FullCellAdjacencyAndVertexIncidence", CellIncidence),
            ("RingsExactCoverClosedChainsAndPinnedGraph", RingFixture),
            ("RingColouring4Through8Deterministic", RingColouring),
            ("RingColouringTasteJsOracle", RingColouringJs),
            ("SlotToCellFullBoundary", SlotCells),
            ("TurnSnapshotsGeneratorAndInverse", TurnSnapshots),
            ("TurnSyntheticThreeCycleInverseAndDigest", TurnCycle),
            ("TurnClockBeforeAndAtDAnd2D", TurnBoundaries),
            ("TurnClockFractionalDurationPhaseAndBoundary", TurnFractionalBoundary),
            ("TurnClockMidpointAndStaleBindingRefusal", TurnMidpoint),
            ("ParameterSchemaAllKindsAndTasteRanges", ParameterContract),
            ("ParameterSchemaRefusals", ParameterRefusals),
            ("DefaultPresetCanonicalBytes", DefaultRoundTrip),
            ("GeneratedPresetCanonicalBytes", GeneratedRoundTrips),
            ("PresetStrictLoadRefusals", PresetRefusals),
            ("PresetCultureInvariantDeAndFr", PresetCultures),
            ("PresetExplicitMigrationHook", PresetMigration),
            ("PresetParseUsesMigratedText", PresetParseMigration),
            ("PresetVersionGateBeforeChangedKeys", PresetVersionGate),
            ("TasteImportOnePopulatedFiveNull", ImportNulls),
            ("TasteImportMalformedAndVersionRefusals", ImportRefusals),
            ("TasteImportSmallerSetAndInvalidExportLeaveOtherFiles", ImportReplacement),
            ("ColourRoundTripsAndGamutMapping", ColourConversions),
            ("PaletteClassPairsSameRingAndThresholdInputs", PaletteGraph),
            ("PaletteGamutFailureIsStructuredAndFailsCheck", PaletteGamutFailure),
            ("PaletteGamutFailureTasteJsOracle", PaletteGamutFailureJs),
            ("ColourAndPaletteTasteJsOracle", ColourJs),
            ("EasingEndpointsMonotonicityAndTurnClamp", EaseTests),
            ("EasingTasteJsOracle", EaseJs),
            ("LayoutDefaultsCoverEveryCommandContext", LayoutDefaults),
            ("LayoutValidationRefusals", LayoutRefusals),
            ("DraftCoreFlowsAndLayoutReports", FlowDefaults),
            ("FlowFormatUnknownCommandAndUnreachableRefusals", FlowRefusals),
            ("FlowHandComputedTravelAndShannonTime", FlowTiny),
            ("LL4KnownFlowReportsMatchEveryCoreNumber", LL4KnownFlowReports),
            ("LL4UnreachableTailClearsReportAndStep", LL4RefusalAndReset),
            ("LL4ReportJsonExactNumbersAndPrivateWrites", LL4ReportJsonAndPrivateWrites),
            ("LL4HeadlessHarnessResultsAndFaultRequirements", LL4HeadlessHarness),
            ("CostAllNullAndMeasuredSource", CostDefaults),
            ("CostValidationRefusals", CostRefusals),
            ("ThemeSetAllStructureMatchesAndDiffers", ThemeSets),
            ("ThemeSetFieldOfViewIsStructure", ThemeFieldOfView),
            ("ReadmeFormatsParametersAndPrivacy", ReadmeContract)
        };
        int passed = 0, failed = 0, skipped = 0;
        foreach (var (name, run) in tests)
        {
            try { run(); Console.WriteLine("PASS " + name); passed++; }
            catch (SkipException ex) { Console.WriteLine("SKIP " + name + ": " + ex.Message); skipped++; }
            catch (Exception ex) { Console.WriteLine("FAIL " + name + ": " + ex); failed++; }
        }
        Console.WriteLine($"LookLab.Tests: {tests.Count} tests; {passed} passed, {skipped} skipped, {failed} failed");
        return failed == 0 ? 0 : 1;
    }

    private static void Check(bool condition, string message)
    {
        if (!condition) throw new Exception(message);
    }
    private static void Equal<T>(T actual, T expected, string message = "values differ") => Check(EqualityComparer<T>.Default.Equals(actual, expected), message + $": {actual} != {expected}");
    private static void Sequence<T>(IEnumerable<T> actual, IEnumerable<T> expected, string message = "sequences differ") => Check(actual.SequenceEqual(expected), message);
    private static void Near(double actual, double expected, double absolute = 1e-12, double relative = 0)
    {
        Check(double.IsFinite(actual) && double.IsFinite(expected) && Math.Abs(actual - expected) <= absolute + relative * Math.Abs(expected), $"{actual:R} != {expected:R}");
    }
    private static void Refuse(Action action, string id)
    {
        try { action(); }
        catch (FormatException ex) { Check(ex.Message.Contains(id, StringComparison.Ordinal), "refusal did not name " + id + ": " + ex.Message); return; }
        throw new Exception("expected refusal naming " + id);
    }

    private static (int Code, string Out, string Error) Run(string executable, IEnumerable<string> args, string input = "", string? workingDirectory = null)
    {
        if (executable == "python") executable = Environment.GetEnvironmentVariable("LOOKLAB_PYTHON") ?? executable;
        var info = new ProcessStartInfo(executable) { WorkingDirectory = workingDirectory ?? Root, UseShellExecute = false, CreateNoWindow = true,
            RedirectStandardInput = true, RedirectStandardOutput = true, RedirectStandardError = true };
        foreach (string arg in args) info.ArgumentList.Add(arg);
        foreach (string key in info.Environment.Keys.ToArray())
            if (System.Text.RegularExpressions.Regex.IsMatch(key, "KEY|SECRET|TOKEN", System.Text.RegularExpressions.RegexOptions.IgnoreCase))
                info.Environment.Remove(key);
        using Process process = Process.Start(info) ?? throw new Exception("could not start " + executable);
        Task<string> output = process.StandardOutput.ReadToEndAsync(), error = process.StandardError.ReadToEndAsync();
        process.StandardInput.Write(input); process.StandardInput.Close();
        if (!process.WaitForExit(60000)) { process.Kill(true); throw new Exception(executable + " timed out"); }
        return (process.ExitCode, output.GetAwaiter().GetResult(), error.GetAwaiter().GetResult());
    }

    private static JsonDocument Node(object input)
    {
        (int Code, string Out, string Error) result;
        try { result = Run("node", new[] { FileAt("tests/taste_oracle.mjs"), Root }, JsonSerializer.Serialize(input)); }
        catch (Win32Exception ex) when (ex.NativeErrorCode == 2 || ex.NativeErrorCode == 3) { throw new SkipException("node is not on PATH"); }
        Check(result.Code == 0, "Taste Lab JS oracle failed: " + result.Error + result.Out);
        return JsonDocument.Parse(result.Out);
    }
}
