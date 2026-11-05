// LX-5 experiment: System.Numerics.BigInteger and shipped .NET libraries only.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Numerics;
using System.Security.Cryptography;
using System.Text;

class Candidate
{
    class Case
    {
        public string Id, Size, Op, File;
        public bool Measured;
        public BigInteger[] X;
    }
    static BigInteger Floor(BigInteger a, BigInteger b)
    {
        var q = BigInteger.DivRem(a, b, out var r);
        return r != 0 && a.Sign != b.Sign ? q - 1 : q;
    }
    static BigInteger[] Rat(BigInteger a, BigInteger d)
    {
        if (d == 0) throw new Exception("zero denominator");
        var g = BigInteger.GreatestCommonDivisor(a, d) * d.Sign;
        return new[] { a / g, d / g };
    }
    static BigInteger[] Q(BigInteger a, BigInteger b, BigInteger d)
    {
        if (d == 0) throw new Exception("zero denominator");
        var g = BigInteger.GreatestCommonDivisor(BigInteger.GreatestCommonDivisor(a, b), d) * d.Sign;
        return new[] { a / g, b / g, d / g };
    }
    static int QSign(BigInteger a, BigInteger b)
    {
        if (a == 0) return b.Sign;
        if (b == 0 || a.Sign == b.Sign) return a.Sign;
        return a.Sign * (a * a - 5 * b * b).Sign;
    }
    static string Join(BigInteger[] x) => string.Join(",", x.Select(n => n.ToString(CultureInfo.InvariantCulture)));
    static string Word(string file)
    {
        using var stream = System.IO.File.OpenRead(file);
        using var hash = IncrementalHash.CreateHash(HashAlgorithmName.SHA256);
        var block = new byte[65536];
        var counts = new ulong[65536];
        BigInteger length = 0, total = 0;
        int n;
        while ((n = stream.Read(block, 0, block.Length)) != 0)
        {
            if (n % 2 != 0) throw new Exception("odd ia16 bytes");
            hash.AppendData(block, 0, n);
            ulong subtotal = 0;
            for (int i = 0; i < n; i += 2)
            {
                int letter = block[i] | (block[i + 1] << 8);
                checked { counts[letter]++; subtotal += (uint)letter; }
            }
            length += n / 2; total += subtotal;
        }
        var hist = new List<string>();
        for (int i = 0; i < counts.Length; i++) if (counts[i] != 0) hist.Add(i + ":" + counts[i]);
        return length + "|" + total + "|" + Convert.ToHexString(hash.GetHashAndReset()).ToLowerInvariant() + "|" + string.Join(",", hist);
    }
    static string Evaluate(Case c, string wordDir)
    {
        if (c.Op == "word") return Word(Path.Combine(wordDir, c.File));
        var x = c.X;
        BigInteger a = x[0], b = x.Length > 1 ? x[1] : 0, result;
        switch (c.Op)
        {
            case "add": result = a + b; break;
            case "sub": result = a - b; break;
            case "mul": result = a * b; break;
            case "div": result = Floor(a, b); break;
            case "rem": result = a - Floor(a, b) * b; break;
            case "gcd": result = BigInteger.GreatestCommonDivisor(a, b); break;
            case "cmp": result = a.CompareTo(b); break;
            case "shl": result = a << checked((int)b); break;
            case "shr": result = a >> checked((int)b); break;
            case "bits": result = BigInteger.Abs(a).GetBitLength(); break;
            case "dec": result = BigInteger.Parse(a.ToString(CultureInfo.InvariantCulture), CultureInfo.InvariantCulture); break;
            case "hex":
                string h = BigInteger.Abs(a).ToString("x", CultureInfo.InvariantCulture).TrimStart('0');
                if (h == "") h = "0";
                result = BigInteger.Parse("0" + h, NumberStyles.AllowHexSpecifier, CultureInfo.InvariantCulture) * (a.Sign < 0 ? -1 : 1);
                return (a.Sign < 0 ? "-" : "") + h + "|" + result;
            case "sum": result = x.Aggregate(BigInteger.Zero, (s, n) => s + n); break;
            case "rat_add": return Join(Rat(a * x[3] + x[2] * b, b * x[3]));
            case "rat_mul": return Join(Rat(a * x[2], b * x[3]));
            case "rat_cmp":
                var r = Rat(a, b); var s = Rat(x[2], x[3]);
                result = (r[0] * s[1]).CompareTo(s[0] * r[1]); break;
            case "q_add": case "q_mul": case "q_conj": case "q_norm": case "q_sign":
                var left = Q(a, b, x[2]);
                var p = left[0]; var q = left[1]; var d = left[2];
                if (c.Op == "q_conj") return Join(new[] { p, -q, d });
                if (c.Op == "q_norm") return Join(Rat(p * p - 5 * q * q, d * d));
                if (c.Op == "q_sign") return QSign(p, q).ToString(CultureInfo.InvariantCulture);
                var right = Q(x[3], x[4], x[5]);
                var u = right[0]; var v = right[1]; var e = right[2];
                return Join(c.Op == "q_add" ? Q(p * e + u * d, q * e + v * d, d * e)
                    : Q(p * u + 5 * q * v, p * v + q * u, d * e));
            default: throw new Exception("unknown operation");
        }
        return result.ToString(CultureInfo.InvariantCulture);
    }
    static void Main(string[] args)
    {
        CultureInfo.CurrentCulture = CultureInfo.InvariantCulture;
        var values = new Dictionary<string, BigInteger>();
        var cases = new List<Case>();
        foreach (var line in System.IO.File.ReadLines(args[0]))
        {
            var f = line.Split('\t');
            if (f[0] == "value") values.Add(f[1], BigInteger.Parse(f[2], CultureInfo.InvariantCulture));
            else cases.Add(new Case { Id = f[1], Size = f[2], Op = f[3], Measured = f[4] == "m", File = f[5],
                X = f[3] == "word" ? Array.Empty<BigInteger>() : f.Skip(5).Select(k => values[k]).ToArray() });
        }
        bool bench = args.Length > 3 && args[3] == "bench";
        var selected = cases.Where(c => !bench || c.Measured).ToList();
        var answers = new Dictionary<string, string>();
        var times = new StringBuilder();
        if (bench)
        {
            int samples = int.Parse(args[4]), seriesCount = int.Parse(args[5]);
            for (int series = 0; series < seriesCount; series++) foreach (var c in selected)
            {
                var warm = Evaluate(c, args[1]);
                for (int sample = 0; sample < samples; sample++)
                {
                    long start = Stopwatch.GetTimestamp();
                    var result = Evaluate(c, args[1]);
                    long elapsed = Stopwatch.GetTimestamp() - start;
                    if (result != warm) throw new Exception("nonrepeatable result: " + c.Id);
                    answers[c.Id] = result;
                    long ns = checked((long)((decimal)elapsed * 1000000000 / Stopwatch.Frequency));
                    times.Append(c.Id).Append('\t').Append(series).Append('\t').Append(sample).Append('\t').Append(ns).Append('\n');
                }
            }
            System.IO.File.WriteAllText(args[2] + ".timings.tsv", times.ToString(), new UTF8Encoding(false));
        }
        else foreach (var c in selected) answers[c.Id] = Evaluate(c, args[1]);
        System.IO.File.WriteAllText(args[2], string.Concat(selected.Select(c => c.Id + "\t" + answers[c.Id] + "\n")), new UTF8Encoding(false));
    }
}
