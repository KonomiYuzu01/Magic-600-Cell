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
            ("StickerAnchorsAreaCentroidAndPin", StickerAnchorRule),
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
            ("A5ViewStateRoundTripAndRefusals", A5ViewStateRoundTripAndRefusals),
            ("A5KeptLookFollowsTheDefaultOnDisk", A5KeptLookFollowsTheDefaultOnDisk),
            ("A5DataWatchReportsAddedChangedAndRemoved", A5DataWatchReportsAddedChangedAndRemoved),
            ("A5OpeningLookAppliesInRangeValuesOnly", A5OpeningLookAppliesInRangeValuesOnly),
            ("A5LocalShapeCentresTheFocusCells", A5LocalShapeCentresTheFocusCells),
            ("A5TokensEveryThemeAssignsEveryRoleAndPassesContrast", A5TokensEveryThemeAssignsEveryRoleAndPassesContrast),
            ("A5TokensDensityFromOneUnitAndDisplayScale", A5TokensDensityFromOneUnitAndDisplayScale),
            ("A5TokensKnobsMoveSeedsInsideTheirRanges", A5TokensKnobsMoveSeedsInsideTheirRanges),
            ("A5TokensOverridesFlagNonStandardStateColours", A5TokensOverridesFlagNonStandardStateColours),
            ("A5TokensFileRefusals", A5TokensFileRefusals),
            ("A5AppearanceResolvesStoresAndRefuses", A5AppearanceResolvesStoresAndRefuses),
            ("A5IconSetNamesGlyphsAndFallbacks", A5IconSetNamesGlyphsAndFallbacks),
            ("A5CommandsRegistryKeymapAndPalette", A5CommandsRegistryKeymapAndPalette),
            ("A5SessionPreviewAndTwoHistories", A5SessionPreviewAndTwoHistories),
            ("A5InspectorFieldsAndFeedbackIndex", A5InspectorFieldsAndFeedbackIndex),
            ("A5ScriptsEntriesKeysAndSlots", A5ScriptsEntriesKeysAndSlots),
            ("A5EditorServiceDiagnosesFixesAndRegions", A5EditorServiceDiagnosesFixesAndRegions),
            ("A5EditorQueueResultsAndKeyRules", A5EditorQueueResultsAndKeyRules),
            ("A5EditorPackagesMountsSlotsAndConsole", A5EditorPackagesMountsSlotsAndConsole),
            ("A5GalleryMotionTableAndCharacters", A5GalleryMotionTableAndCharacters),
            ("A5GalleryPalettesIconsAndTokenLint", A5GalleryPalettesIconsAndTokenLint),
            ("A5GalleryFocusLayersAndVirtualRows", A5GalleryFocusLayersAndVirtualRows),
            ("A5SolvingStripFactsAndInvalidTuple", A5SolvingStripFactsAndInvalidTuple),
            ("A5SolvingRailOrderMarksAndJump", A5SolvingRailOrderMarksAndJump),
            ("A5SolvingDraftLineGoalAndRoute", A5SolvingDraftLineGoalAndRoute),
            ("A5SolvingKeyMapHeldPinnedAndNamed", A5SolvingKeyMapHeldPinnedAndNamed),
            ("A5CommitBarInlineParameters", A5CommitBarInlineParameters),
            ("A5ScreenChecksKeyboardAndWords", A5ScreenChecksKeyboardAndWords),
            ("A5HistoryBranchesInFull", A5HistoryBranchesInFull),
            ("A5PaletteReadoutsAndPanelStates", A5PaletteReadoutsAndPanelStates),
            ("A5InspectorLadderAndPreviewRange", A5InspectorLadderAndPreviewRange),
            ("A6DraftFailedCheckStopsAtThePreview", A6DraftFailedCheckStopsAtThePreview),
            ("A6TypedLineNeverCommits", A6TypedLineNeverCommits),
            ("A6PrefixRecheckedAsItsOwnPreview", A6PrefixRecheckedAsItsOwnPreview),
            ("A6HeadMoveMakesAReadyPreviewStale", A6HeadMoveMakesAReadyPreviewStale),
            ("A6EditedStatementGetsItsOwnWitness", A6EditedStatementGetsItsOwnWitness),
            ("A6BranchConfirmOnlyForTheOpenQuestion", A6BranchConfirmOnlyForTheOpenQuestion),
            ("A6OnlyEvaluatedEffectsSayUntouched", A6OnlyEvaluatedEffectsSayUntouched),
            ("A6NextAcrossOrbitsCarriesEachDraft", A6NextAcrossOrbitsCarriesEachDraft),
            ("A6ReplayAndSidesResolveThroughTheRegistry", A6ReplayAndSidesResolveThroughTheRegistry),
            ("A6GalleryPaneStatesAreTheDocks", A6GalleryPaneStatesAreTheDocks),
            ("A6ScriptFramesByAuthorClass", A6ScriptFramesByAuthorClass),
            ("A6AnchorsFollowTheViewsProjection", A6AnchorsFollowTheViewsProjection),
            ("A6LibraryInSolve", A6LibraryInSolve),
            ("A6NumericGesturesStayOnTheGrid", A6NumericGesturesStayOnTheGrid),
            ("A6NonexistentAddressesAreNoWitness", A6NonexistentAddressesAreNoWitness),
            ("A6OneWitnessOneCollateral", A6OneWitnessOneCollateral),
            ("A6EmptyLibrarySelectionPreviewsNothing", A6EmptyLibrarySelectionPreviewsNothing),
            ("A6SurfaceActionsResolveThroughTheRegistry", A6SurfaceActionsResolveThroughTheRegistry),
            ("A6SignatureHas35SlotsInRailOrder", A6SignatureHas35SlotsInRailOrder),
            ("A6TemplateBindsItsRoles", A6TemplateBindsItsRoles),
            ("A5FrameLockedPairRefusesHidingAMainView", A5FrameLockedPairRefusesHidingAMainView),
            ("A5FrameStoredArrangementRoundTripAndWorkspaces", A5FrameStoredArrangementRoundTripAndWorkspaces),
            ("LayoutValidationRefusals", LayoutRefusals),
            ("LL6PairedLayoutRulesAndStageRouting", PairedLayoutRefusals),
            ("LL6ArrangementSingleSplitAndInset", LL6Arrangement),
            ("LL6ShaderProjectionAndMovingRestAnchors", LL6ProjectionAndRestAnchors),
            ("LL6FocusNearPlaneAndDenominatorRegressions", LL6FocusRegressions),
            ("LL6SeededFocusSweepAndEmptyTransitions", LL6FocusSweep),
            ("LL6SeededCrossingsOfEveryGuard", LL6GuardContinuity),
            ("LL6FollowerCapsConvergenceAndBadDt", LL6Follower),
            ("LL7HopfFactsCoversPathPhasesScrewsAndBead", LL7HopfFacts),
            ("LL7HopfFixtureAgreesWithKeptScript", LL7HopfFixture),
            ("LL7ChartsOrthonormalPositiveAndOneBead", LL7Charts),
            ("LL7OrientationEveryCellAndLayout", LL7Orientation),
            ("LL7SeparationUnshrunkRingsAndSheet", LL7Separation),
            ("LL7FitCornersAllAnchorsAndSeededRotations", LL7Fit),
            ("LL7TurnTargetsFullW3AndInconsistentTable", LL7TurnTargets),
            ("LL7BlendHandoffRetainedInputResidualBounds", LL7BlendHandoff),
            ("LL7ProjectOverloadLegacyIdentityAndLayouts", LL7ProjectOverload),
            ("LL7CellLayoutRowsMatchPlacement", LL7CellLayout),
            ("LL7LayoutFramingAndLegacyIdentity", LL7LayoutFraming),
            ("LL7DragSignsCameraOrderAndOrthonormalSpin", LL7Drag),
            ("LL7SharedColouringEquitableClassesAndReports", LL7Colouring),
            ("LL7G1PresetsCanonicalAndOnlyStatedChanges", LL7G1Presets),
            ("LL7StereographicCapHidesPoleAndFitsFrame", LL7StereographicCap),
            ("LL8CommittedShot", LL8Committed),
            ("LL8TrackCameraTurnsAndBindings", LL8Track),
            ("LL8TrackDigestRepeatableAndSensitive", LL8Digest),
            ("LL8ShotParseRules", LL8ParseRules),
            ("LL8IntegerFramesNearBoundary", LL8IntegerFrames),
            ("LL8HeroMustShowTurnInEveryLook", LL8Spots),
            ("LL8ManifestAndStrictContext", LL8ManifestContext),
            ("LL8ReadmeFormatsAndCommand", LL8Readme),
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
            ("ReadmeFormatsParametersAndPrivacy", ReadmeContract),
            ("D02RolesExistInBothThemes", D02RolesExistInBothThemes),
            ("D02MotionRowsStaticUnderReducedMotion", D02MotionRowsStaticUnderReducedMotion),
            ("D02DraftRowsAbsentFromTable", D02DraftRowsAbsentFromTable),
            ("D02WordsMatchRecord", D02WordsMatchRecord),
            ("D02SlotLateResultIgnored", D02SlotLateResultIgnored),
            ("D02SlotCancelLatestRestores", D02SlotCancelLatestRestores),
            ("D02SlotCancelOlderChangesNothing", D02SlotCancelOlderChangesNothing),
            ("D02SlotCompletedWithoutCoverageIsError", D02SlotCompletedWithoutCoverageIsError),
            ("D02SlotWordsCoverSevenStates", D02SlotWordsCoverSevenStates),
            ("D02NumeralKindsStayApart", D02NumeralKindsStayApart),
            ("D02EntryRevertsOnlyUnchanged", D02EntryRevertsOnlyUnchanged),
            ("D02EntryFollowsUndoRedoBranchReopen", D02EntryFollowsUndoRedoBranchReopen),
            ("D02EntryIsChromeOnly", D02EntryIsChromeOnly),
            ("D02UndefinedSwitchesOnLatticeFlagNotPendingN", D02UndefinedSwitchesOnLatticeFlagNotPendingN),
            ("D02MarkOnEveryViewBorder", D02MarkOnEveryViewBorder),
            ("D02PanelsMoveFreely", D02PanelsMoveFreely),
            ("D02TotalNPendingUntilSeamFactsAll", D02TotalNPendingUntilSeamFactsAll),
            ("D02TotalNIncompleteAfterCutOff", D02TotalNIncompleteAfterCutOff),
            ("D02TotalNIsNotSumOfCaps", D02TotalNIsNotSumOfCaps),
            ("D02BlockedCapReadoutUndefined", D02BlockedCapReadoutUndefined),
            ("D02Above6MaxUndefinedAtZero", D02Above6MaxUndefinedAtZero),
            ("D02TotalsMatchSample", D02TotalsMatchSample),
            ("D02CapClassesExclusiveAndExhaustive", D02CapClassesExclusiveAndExhaustive),
            ("D02NoNumeralAtBlockedCap", D02NoNumeralAtBlockedCap),
            ("D02UncertainRimRowIsPending", D02UncertainRimRowIsPending),
            ("D02CapMapMatchesRim", D02CapMapMatchesRim),
            ("D02CapLayoutIsRingSheet", D02CapLayoutIsRingSheet),
            ("D02NoDefaultThreshold", D02NoDefaultThreshold),
            ("D02ThresholdCountsBlockedApart", D02ThresholdCountsBlockedApart),
            ("D02ThresholdNeverReorders", D02ThresholdNeverReorders),
            ("D02NoJobBeforeNaming", D02NoJobBeforeNaming),
            ("D02HoverNamesNothing", D02HoverNamesNothing),
            ("D02MeetingCapsAre80", D02MeetingCapsAre80),
            ("D02Q3bTwelveTwists", D02Q3bTwelveTwists),
            ("D02CancelledJobNeverCompletes", D02CancelledJobNeverCompletes),
            ("D02C2SmallFixes", D02C2SmallFixes),
            ("D02FocusLayerValidatesWhole", D02FocusLayerValidatesWhole),
            ("D02FocusLayerCountsExact", D02FocusLayerCountsExact),
            ("D02FocusLayerComposes", D02FocusLayerComposes),
            ("D02FocusLayerPreviewBeforeApply", D02FocusLayerPreviewBeforeApply),
            ("D02FocusLayerKindsApart", D02FocusLayerKindsApart),
            ("D02FocusLayerPresetsSaveLoad", D02FocusLayerPresetsSaveLoad),
            ("D02WordLengthIsExpanded", D02WordLengthIsExpanded),
            ("D02PlaybackStepsByPartThenTwist", D02PlaybackStepsByPartThenTwist),
            ("D02OffLatticeWordSignatureUndefined", D02OffLatticeWordSignatureUndefined),
            ("D02CrossingSentenceNeedsReachability", D02CrossingSentenceNeedsReachability),
            ("D02NuBandAlignsWith35Cells", D02NuBandAlignsWith35Cells),
            ("D02C5ValuesAreDialsNotBars", D02C5ValuesAreDialsNotBars),
            ("D02NuBandUndefinedOffLattice", D02NuBandUndefinedOffLattice),
            ("D02CorrectionsAre36InIdOrder", D02CorrectionsAre36InIdOrder),
            ("D02CorrectionSetCheckMatchesSample", D02CorrectionSetCheckMatchesSample),
            ("D02CoversUndefinedIsListed", D02CoversUndefinedIsListed),
            ("D02CoversTableHas80RowsBothDirections", D02CoversTableHas80RowsBothDirections),
            ("D02HoverInCoversMarksNoCapMapCell", D02HoverInCoversMarksNoCapMapCell),
            ("D02FocusLayerCountsMatchSample", D02FocusLayerCountsMatchSample),
            ("D02RepairOrderEqualsSample", D02RepairOrderEqualsSample),
            ("D02RepairModelHasNoSortMember", D02RepairModelHasNoSortMember),
            ("D02RepairNoFocusOnOpen", D02RepairNoFocusOnOpen),
            ("D02RepairDigitsOnlyInContext", D02RepairDigitsOnlyInContext),
            ("D02RepairEmptyNamesCapRevisionAndCoverage", D02RepairEmptyNamesCapRevisionAndCoverage),
            ("D02RepairCutOffIsNotEmpty", D02RepairCutOffIsNotEmpty),
            ("D02RepairTwoTwistRowHasTwoSteps", D02RepairTwoTwistRowHasTwoSteps),
            ("D02StageOnlyFromPick", D02StageOnlyFromPick),
            ("D02SealedSpanRemovedWhole", D02SealedSpanRemovedWhole),
            ("D02TypedTwistsAreOrdinaryWord", D02TypedTwistsAreOrdinaryWord),
            ("D02ResearchOriginRefusedInSolvingSession", D02ResearchOriginRefusedInSolvingSession),
            ("D02BoundaryCountZeroOverScriptedSession", D02BoundaryCountZeroOverScriptedSession),
            ("D02PaneLayoutIsBoundingBox", D02PaneLayoutIsBoundingBox),
            ("D02PanesNeverOverlap", D02PanesNeverOverlap),
            ("D02FifthPaneRefused", D02FifthPaneRefused),
            ("D02HitResolvesItsView", D02HitResolvesItsView),
            ("D02GroundCutsBothOpenings", D02GroundCutsBothOpenings),
            ("D02PortErrorLogShowsEachFailureOnce", D02PortErrorLogShowsEachFailureOnce),
            ("D02PortCaughtListenerReachesTheLog", D02PortCaughtListenerReachesTheLog),
            ("D02PortPlacesMapPositionsAsLookLab", D02PortPlacesMapPositionsAsLookLab),
            ("D02PortPlacesFollowTheCoresProjection", D02PortPlacesFollowTheCoresProjection),
            ("D02PortPlacesFocusAsTheCore", D02PortPlacesFocusAsTheCore),
            ("D02PortLayoutTrackerNamesAndRestoresSteps", D02PortLayoutTrackerNamesAndRestoresSteps),
            ("D02PortAppearanceRestoresInPlace", D02PortAppearanceRestoresInPlace),
            ("D02PortViewFieldsKeepEachViewAndWriteTheScope", D02PortViewFieldsKeepEachViewAndWriteTheScope),
            ("D02PortViewLinkMarksEachState", D02PortViewLinkMarksEachState),
            ("D02HeldSeatShowsTwelveTwistsWithOrders", D02HeldSeatShowsTwelveTwistsWithOrders),
            ("D02NoDuplicateChordAmongD02Bindings", D02NoDuplicateChordAmongD02Bindings),
            ("D02GlobalNamesItsChain", D02GlobalNamesItsChain),
            ("D02ExposedFacesUnavailableOnBorderAndTier1", D02ExposedFacesUnavailableOnBorderAndTier1),
            ("D02CapPromptListsCapsInIdOrder", D02CapPromptListsCapsInIdOrder),
            ("D02CapPromptEnterNamesTheChosenCap", D02CapPromptEnterNamesTheChosenCap),
            ("D02CapMapClickSaysWhy", D02CapMapClickSaysWhy),
            ("D02LocalCapDrawsTheLowestCapUntilOneIsNamed", D02LocalCapDrawsTheLowestCapUntilOneIsNamed),
            ("D02VerdictWordsExact", D02VerdictWordsExact),
            ("D02NotApplicableCountsAsNoCheck", D02NotApplicableCountsAsNoCheck),
            ("D02CutOffIsNeverAVerdict", D02CutOffIsNeverAVerdict),
            ("D02PinnedInputNeverStaleOnCommit", D02PinnedInputNeverStaleOnCommit),
            ("D02RenameChangesNoDigest", D02RenameChangesNoDigest),
            ("D02EveryResearchControlInvokesARow", D02EveryResearchControlInvokesARow),
            ("D02LocalPlanDrawsCellZeroOnTheBasePuzzle", D02LocalPlanDrawsCellZeroOnTheBasePuzzle),
            ("D02LocalPlanKeepsTheJumbleRule", D02LocalPlanKeepsTheJumbleRule),
            ("D02TintModesExclusive", D02TintModesExclusive),
            ("D02ContactColouringIsProper", D02ContactColouringIsProper),
            ("D02FourClassesAtWj300AndDeepExports", D02FourClassesAtWj300AndDeepExports),
            ("D02OneClassAtIa40", D02OneClassAtIa40),
            ("D02ClassStickyAcrossCommitUndoRedoBranchReopen", D02ClassStickyAcrossCommitUndoRedoBranchReopen),
            ("D02OverflowNeverReusesAcrossContact", D02OverflowNeverReusesAcrossContact),
            ("D02OffLatticeFromFlagNotHeights", D02OffLatticeFromFlagNotHeights),
            ("D02StraddlerLayerOnlyAtBlockedCap", D02StraddlerLayerOnlyAtBlockedCap)
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
