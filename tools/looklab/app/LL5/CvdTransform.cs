using System;

namespace LookLab.App.LL5;

// Machado severity-one coefficients from Colour.SimulateCvd. One f32 upload
// feeds both the drawing shader and the headless preview colour path.
internal static class CvdTransform
{
    public static float[][] Rows(string mode) => mode switch
    {
        "normal" => new[] { new[] { 1f, 0f, 0f }, new[] { 0f, 1f, 0f }, new[] { 0f, 0f, 1f } },
        "protan" => new[] { new[] { .152286f, 1.052583f, -.204868f }, new[] { .114503f, .786281f, .099216f }, new[] { -.003882f, -.048116f, 1.051998f } },
        "deutan" => new[] { new[] { .367322f, .860646f, -.227968f }, new[] { .280085f, .672501f, .047413f }, new[] { -.011820f, .042940f, .968881f } },
        "tritan" => new[] { new[] { 1.255528f, -.076749f, -.178779f }, new[] { -.078411f, .930809f, .147602f }, new[] { .004733f, .691367f, .303900f } },
        _ => throw new FormatException("CVD: unknown mode " + mode)
    };

    public static float[] Apply(float[] linear, float[][] rows, bool enabled)
    {
        if (!enabled) return (float[])linear.Clone();
        var result = new float[3];
        for (int i = 0; i < 3; i++)
            result[i] = Math.Clamp(rows[i][0] * linear[0] + rows[i][1] * linear[1] + rows[i][2] * linear[2], 0, 1);
        return result;
    }
}
