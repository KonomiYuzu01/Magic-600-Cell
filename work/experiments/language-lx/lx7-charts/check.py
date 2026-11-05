"""LX-7 source/synthetic acceptance. No window and no device dispatch."""
import sys
sys.dont_write_bytecode = True
import copy
from fractions import Fraction as F
from hashlib import sha256
import json
import math
import os
from pathlib import Path
import shutil
import struct
import subprocess
from freeze import verify_manifest, clean_build
from generate_hlsl import generate, function
from make_fixtures import make
from reference import HERE, cases, cpu, exact, expectations, json_bytes, programs, record, summarize, input_bytes
from subset import OK, SINGULAR, NONFINITE, Refusal, admit, evaluate, f32, bits
sys.path.insert(0, str(HERE / "device"))
from compare_device import compare
from run import FLAGS


def require(condition, message):
    if not condition:
        raise ValueError(message)


def round_exact(value):
    """Independent Fraction -> IEEE binary32, integer quotient/remainder ties-to-even."""
    sign = 0x80000000 if value < 0 else 0
    value = abs(value)
    if not value:
        return 0.0
    n, d = value.numerator, value.denominator
    exponent = n.bit_length() - d.bit_length()
    if (n < (d << exponent)) if exponent >= 0 else ((n << -exponent) < d):
        exponent -= 1
    step = max(exponent - 23, -149)
    numerator, denominator = (n, d << step) if step >= 0 else (n << -step, d)
    q, remainder = divmod(numerator, denominator)
    if 2 * remainder > denominator or (2 * remainder == denominator and q % 2):
        q += 1
    if q >= 2**24:
        q >>= 1
        step += 1
    if step > 104:
        encoded = 0x7f800000
    elif q < 2**23:
        require(step == -149, "invalid subnormal oracle")
        encoded = q
    else:
        encoded = ((step + 150) << 23) | (q - 2**23)
    return struct.unpack("<f", struct.pack("<I", sign | encoded))[0]


def rational_execution(program, point, d):
    env = dict(zip(("x", "y", "z", "w", "d"), (*point, d), strict=True))
    if not all(math.isfinite(v) for v in env.values()):
        return NONFINITE, None

    def get(arg):
        return env[arg] if isinstance(arg, str) else arg

    for dest, op, args in program.instructions:
        values = [F(get(arg)) for arg in args]
        a = values[0]
        if op == "exclude":
            if abs(a) <= values[1]:
                return SINGULAR, None
            continue
        if op == "div" and values[1] == 0:
            return NONFINITE, None
        if op in ("abs", "neg"):
            v = abs(a) if op == "abs" else -a
        else:
            b = values[1]
            if op == "add": v = a + b
            elif op == "sub": v = a - b
            elif op == "mul": v = a * b
            elif op == "div": v = a / b
            else: raise AssertionError(op)
        env[dest] = round_exact(v)
        if not math.isfinite(env[dest]):
            return NONFINITE, None
    return OK, tuple(get(arg) for arg in program.outputs)


def admission_and_arithmetic():
    for name in ("perspective", "stereographic", "gnomonic", "bounded"):
        admit((HERE / "charts" / (name + ".chart")).read_text(encoding="utf-8"))
    refusals = {"recursion": "E_RECURSION", "unbounded": "E_UNBOUNDED_LOOP",
                "float64": "E_LITERAL", "impure": "E_CALL"}
    samples = [((HERE / "charts/rejected" / (name + ".chart")).read_text(encoding="utf-8"), code)
               for name, code in refusals.items()]
    samples += [("output([x], y, z)", "E_SYNTAX"), ("import os\noutput(x,y,z)", "E_SYNTAX"),
                ("output(16777217,y,z)", "E_LITERAL"), ("output(1e100,y,z)", "E_LITERAL"),
                ("output(True,y,z)", "E_LITERAL"), ("output(x**2,y,z)", "E_OPERATION"),
                ("for _ in range(9):\n a=x\noutput(x,y,z)", "E_BOUNDS"),
                ("for _ in range(d):\n a=x\noutput(x,y,z)", "E_UNBOUNDED_LOOP"),
                ("x=1\noutput(x,y,z)", "E_NAME"), ("output(x,y,z)\na=x", "E_OUTPUT"),
                ("a=x", "E_OUTPUT"), ("output(missing,y,z)", "E_NAME"),
                ("output(x.real,y,z)", "E_SYNTAX"),
                ("for _ in range(8):\n for _ in range(8):\n  for _ in range(8):\n   a=x*x\noutput(x,y,z)", "E_BOUNDS")]
    for source, code in samples:
        for operation in (admit, lambda s: function("refused", s)):
            try:
                operation(source)
            except Refusal as error:
                require(error.code == code, "unexpected admission refusal: " + error.code)
            else:
                raise ValueError("unsafe source admitted or generated")
    bounded = admit((HERE / "charts/bounded.chart").read_text(encoding="utf-8"))
    require(evaluate(bounded, (8, 2, 3, 0), 1) == (OK, (1, 2, 3)), "bounded loop result")
    require(evaluate(admit("output((x+y)-x,y,z)"), (2**24, 1, 0, 0), 1)[1][0] == 0, "per-step rounding")
    require(evaluate(admit("output(x/w,y,z)"), (1, 2, 3, 0), 1) == (NONFINITE, None), "total zero division")
    require(evaluate(admit("output(x,y,z)"), (math.nan, 2, 3, 0), 1) == (NONFINITE, None), "total nonfinite input")
    require(bits(evaluate(admit("output(-x,y,z)"), (0, 0, 0, 0), 1)[1][0]) == 0x80000000, "signed zero")
    golden = [(F(1) + F(1, 2**24), 0x3f800000), (F(1) + F(3, 2**24), 0x3f800002),
              (F(1, 2**149), 1), (F(1, 2**150), 0), (F(3, 2**150), 2),
              (F(2**128 - 2**104), 0x7f7fffff), (F(2**128 - 2**103), 0x7f800000),
              (F(-1, 2**150), 0x80000000), (F(1, 3), 0x3eaaaaab)]
    for value, expected in golden:
        require(bits(round_exact(value)) == expected, "integer rounding oracle golden")
        require(bits(f32(float(value))) == expected, "struct rounding golden")
    return {"accepted_sources": 4, "refused_sources": len(samples), "rounding_goldens": len(golden)}


def cpu_checks(fixtures):
    checked = programs()
    records, builtin_rows = [], {}
    for case in cases(fixtures):
        result = cpu(case, checked)
        # Independently round every camera input, product and sum using Fractions.
        vertex = [round_exact(v) for v in case["vertex"]]
        pose = []
        for row in case["matrix"]:
            products = [round_exact(F(round_exact(q)) * F(v)) for q, v in zip(row, vertex, strict=True)]
            a = products[0]
            for b in products[1:]:
                a = round_exact(F(a) + F(b))
            pose.append(a)
        program = checked["perspective" if case["stage_id"] == 0 else case["stage"]]
        expected = rational_execution(program, pose, round_exact(case["d"]))
        require(result == expected, "CPU versus exact per-operation oracle: " + case["id"])
        encoded = record(result)
        records.append(encoded)
        suffix = case["id"].split("/", 1)[1]
        if case["stage_id"] == 0:
            builtin_rows[suffix] = encoded
        elif case["stage_id"] == 1:
            require(encoded == builtin_rows[suffix], "generated versus independent built-in CPU bits")
    require(json_bytes(records) == (HERE / "cpu-records.json").read_bytes(), "frozen CPU bits mismatch")
    require(json_bytes(summarize(fixtures, records)) == (HERE / "cpu-summary.json").read_bytes(), "error summary mismatch")
    # Independent rational closed-form sentinels, including the inclusive boundary.
    simple = {"stage": "perspective", "d": F(2), "vertex": (F(1), F(2), F(3), F(1)),
              "matrix": [[F(i == j) for j in range(4)] for i in range(4)]}
    require(exact(simple) == (OK, (F(2), F(4), F(6))), "exact perspective sentinel")
    simple["vertex"] = (F(1), F(2), F(3), F(2) - F(1, 4096))
    require(exact(simple) == (SINGULAR, None), "inclusive exact exclusion")
    return records


def comparator_checks(fixtures, records):
    # In-memory synthetic capture tests exercise binding/refusal, never claim a GPU.
    capture = {"format": "lx7-device-capture-v1", "evidence_class": "synthetic", "sandbox_lead": True,
               "requested_adapter": "synthetic fixture; no device", "compile_flags": FLAGS,
               "device": {"adapter": "synthetic fixture; no device", "software": False,
                          "shader_model": "6.2", "records": len(records)},
               "manifest_sha256": sha256((HERE / "manifest.json").read_bytes()).hexdigest(),
               "input_sha256": sha256(input_bytes(fixtures)).hexdigest(),
               "hlsl_sha256": sha256((HERE / "generated.hlsl").read_bytes()).hexdigest(),
               "dxil_sha256": "0" * 64, "executable_sha256": "0" * 64,
               "output_sha256": sha256(b"".join(struct.pack("<4I", *r) for r in records)).hexdigest(),
               "records": records}
    require(compare(capture)["evidence_class"] == "synthetic", "synthetic comparison classification")
    for field in ("manifest_sha256", "input_sha256", "hlsl_sha256", "output_sha256"):
        bad = dict(capture, **{field: "0" * 64})
        try:
            compare(bad)
        except ValueError:
            pass
        else:
            raise ValueError("capture binding failed to refuse " + field)
    for replacement in ((0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff), (0, 0x7fc00000, 0, 0), (1, 1, 0, 0)):
        bad = copy.deepcopy(capture)
        bad["records"][0] = replacement
        bad["output_sha256"] = sha256(b"".join(struct.pack("<4I", *r) for r in bad["records"])).hexdigest()
        try:
            compare(bad)
        except ValueError:
            pass
        else:
            raise ValueError("invalid coverage/output accepted")


def compile_hlsl(build):
    sdk = os.environ.get("ProgramFiles(x86)")
    pinned = Path(sdk) / "Windows Kits/10/bin/10.0.26100.0/x64/dxc.exe" if sdk else None
    dxc = str(pinned) if pinned and pinned.is_file() else shutil.which("dxc.exe")
    if not dxc:
        return {"status": "not run here", "reason": "dxc.exe unavailable"}
    env = dict(os.environ, TMP=str(build / "tmp"), TEMP=str(build / "tmp"))
    try:
        version = subprocess.run([dxc, "--version"], cwd=HERE, env=env, capture_output=True, timeout=15)
    except OSError as error:
        return {"status": "not run here", "reason": "dxc.exe could not start", "os_error": error.errno}
    require(version.returncode == 0, "DXC starts but version probe fails")
    version_text = version.stdout.decode("utf-8", errors="replace").strip()
    require("\\" not in version_text and ":/" not in version_text and len(version_text) <= 512, "unexpected DXC version output")
    with (build / "dxc.log").open("wb") as log:
        result = subprocess.run([dxc, *FLAGS, "generated.hlsl", "-Fo", "build/generated.dxil"],
                                cwd=HERE, env=env, stdout=log, stderr=subprocess.STDOUT, timeout=120)
    if result.returncode:
        diagnostics = (build / "dxc.log").read_text(encoding="utf-8", errors="replace")
        # Emit only local compiler text without absolute directory prefixes.
        diagnostics = diagnostics.replace(str(HERE), "work/experiments/language-lx/lx7-charts")
        print(diagnostics[:4000])
        raise ValueError("DXC compilation failed (exit " + str(result.returncode) + ")")
    binary = (build / "generated.dxil").read_bytes()
    require(binary.startswith(b"DXBC"), "DXC did not produce a shader container")
    return {"status": "passed", "version": version_text, "flags": FLAGS, "dxil_sha256": sha256(binary).hexdigest()}


def main():
    build = HERE / "build"
    require(not build.exists(), "build exists; do not overlap experiment runs")
    (build / "tmp").mkdir(parents=True)
    report = {"format": "lx7-check-result-v1", "status": "failed", "python": sys.version.split()[0],
              "evidence_class": ["source", "synthetic"], "device": "not run here", "W3": "not run here",
              "decision": "open", "decider": "owner"}
    try:
        manifest = verify_manifest()
        report["manifest_sha256"] = sha256((HERE / "manifest.json").read_bytes()).hexdigest()
        report["candidate_sha256"] = manifest["candidate_sha256"]
        report["checks"] = admission_and_arithmetic()
        fixtures = json.loads((HERE / "fixtures.json").read_bytes())
        require(json_bytes(make()) == (HERE / "fixtures.json").read_bytes(), "fixture regeneration mismatch")
        require(json_bytes(expectations(fixtures)) == (HERE / "expectations.json").read_bytes(), "exact expectation mismatch")
        records = cpu_checks(fixtures)
        comparator_checks(fixtures, records)
        first, second = generate(), generate()
        require(first == second == (HERE / "generated.hlsl").read_bytes(), "HLSL generation is not frozen/deterministic")
        report["checks"].update({"vertices": len(fixtures["vertices"]), "poses": len(fixtures["poses"]),
                                 "stage_cases": len(records), "hlsl_sha256": sha256(first).hexdigest(),
                                 "cpu_exact_operation_oracle": "passed", "capture_binding_refusals": "passed"})
        report["dxc"] = compile_hlsl(build)
        report["status"] = "passed"
        print("PASS: frozen digests, admission, rational/float32 reference, capture validation, deterministic HLSL")
        print("DXC: " + report["dxc"]["status"])
        print("Device: not run here by check.py; W3: not run here; precision decision: open")
        return 0
    except (ValueError, AssertionError, OSError, subprocess.TimeoutExpired) as error:
        report["failure"] = str(error) if isinstance(error, (ValueError, AssertionError)) else type(error).__name__
        print("FAIL: " + report["failure"])
        return 1
    finally:
        clean_build()
        report["build_products_removed"] = not build.exists()
        (HERE / "check-result.json").write_bytes(json_bytes(report))


if __name__ == "__main__":
    raise SystemExit(main())
