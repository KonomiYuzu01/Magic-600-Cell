# 0.4.1 screening packets

Read-only Codex screening of the 0.4 code, prepared for 0.4.1 step 4. The six shards are independent and run concurrently. Shards A to C cover critical-path code and use Astra on the standard tier; D to F use Sol on the fast tier. Each packet names its command.

After the runs, the integrator merges the findings into one ranked list, answers each with `adopt`, `reject_with_evidence` or `needs_verification`, and turns adopted findings into falsifying experiments and then `--kind implement` packets. Raw results stay under `work/reviews/`; only the sanitized ranked list is published.
