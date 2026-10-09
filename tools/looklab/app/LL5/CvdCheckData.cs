using System;
using System.Linq;
using LookLab.Core;

namespace LookLab.App.LL5;

internal static class CvdCheckData
{
    // Three f32 products and additions, plus clamping, versus core doubles.
    // RGBA32F readback adds no 8-bit or half-float quantization.
    public const double Tolerance = 2e-6;
    public static double[][] LinearSamples { get; } = new[]
    {
        new[] { 0.0, 0.0, 0.0 }, new[] { 1.0, 1.0, 1.0 },
        new[] { 1.0, 0.0, 0.0 }, new[] { 0.0, 1.0, 0.0 }, new[] { 0.0, 0.0, 1.0 },
        new[] { .5, .5, .5 }, new[] { .25, .5, .75 }, new[] { .02, .04, .06 },
        new[] { .04045, .0031308, .9 }, new[] { 1.0, .01, .9 },
        new[] { .9, 1.0, .01 }, new[] { .01, .9, 1.0 }, new[] { .25, .1, .01 }
    }.Select(srgb => srgb.Select(Colour.SrgbToLinear).ToArray()).ToArray();

    public static double[] Reference(double[] linear, string mode) => mode == "normal" ? linear : Colour.SimulateCvd(linear, mode);

    public static void Require(float[] actual, double[] expected, string check)
    {
        for (int component = 0; component < 3; component++)
            if (!float.IsFinite(actual[component]) || Math.Abs(actual[component] - expected[component]) > Tolerance)
                throw new Exception($"{check}: component={component} actual={actual[component]:R} core={expected[component]:R}; tolerance=2e-6 absolute linear RGB");
    }
}
