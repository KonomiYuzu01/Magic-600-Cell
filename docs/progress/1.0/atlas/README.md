# 0.4 function atlas

An interactive page for reviewing the reverse engineering of 0.4 before the stage 2.3 redesign: every 0.4 function unit, what it does in 0.4, and the 1.0 decision it received. It is a reading aid; the sources below stay authoritative.

Build it from the repository root:

```
python docs/progress/1.0/atlas/build.py
```

The script writes `work/atlas/atlas.html` (ignored by Git; `--out` picks another path). The page is self-contained except for two loads: the d3 library from cdnjs for the dependency map, and Google Fonts. Every other view works without them.

## What it reads

| Source | Used for |
|---|---|
| `dispositions/units.json` | the 503 units, their names, entry points, dependencies and screening links |
| `dispositions/dispositions-s1.json` to `-s6.json` | the 1.0 decision, purpose, reason, flows and requirements of each unit or split part |
| `dispositions/flows.json` | the 15 provisional flows |
| `inventory/claude-source-s*.json`, `inventory/bottom-up-s*.json` | the 0.4 behaviour and evidence rows from the two independent inventories |
| `inventory/shards.json` | the six source areas |
| `command-table.json`, `inventory/top-down.json` | the H-08 command table and the 0.4 functions each command comes from |
| `inventory/screening-map.json` | the 0.4.1 screening findings and their code locations |

The build fails when the data disagree: a unit names an unknown flow, a unit cites an inventory row that does not exist, or the decision counts differ from the `all` row of `dispositions/dispositions.md`. The page header shows the commit it was built from.

## Views

- **Flows:** the units under each flow, in solving-workflow order, coloured by decision.
- **Dependency map:** units as dots, linked where one uses another, grouped by source area or by first flow.
- **Flow × area:** how many units serve each flow from each source area.
- **Units:** one row per unit.
- **Commands:** the H-08 command table with gates, preview, undo and contexts.
- **Screening:** the 0.4.1 findings with the units they attach to.

Selecting any unit opens its record: purpose and reason, 1.0 requirements, the 0.4 behaviour from both inventories with file and line evidence, entry points, the units it uses and is used by, units with the same purpose, and its screening findings.
