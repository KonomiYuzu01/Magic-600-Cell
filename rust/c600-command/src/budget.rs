//! The budget a program row's invocation declares, and the ceiling it must fit (KN7.7, BM3, DET.27, ERR.27; LP2
//! draft).
//!
//! - An invocation of a row an entry serves declares its budget explicitly in the envelope, beside the epoch and the
//!   token ([`crate::Invocation::budget`]), never in the row's arguments or its table entry (KN7.7).
//! - The layer admits it before a job ID exists, so a refusal uses nothing:
//!   - no declaration is `KN7:input.budget_not_declared`;
//!   - a dimension over the ceiling is `KN7:resource_limit.budget_ceiling`, naming the dimension, the request and the
//!     ceiling.
//! - The ceiling is the session profile's ([`Profile::budget_ceiling`]), or the host's where the host sets a lower one
//!   ([`crate::CommandLayer::set_budget_ceiling`]), dimension by dimension. The safety stop is not checked here.
//! - The admitted budget travels on the job ([`crate::ProgramJob::budget`]) for the host to run the entry under.
//! - The request digest covers a declared budget, so a retry with another budget is other content (KN5.21).
//! - A workspace row's completion keeps the budget in its journal entry, so a replay reads the budget its job ran
//!   under.
//!
//! The dimensions are the runtime meter's (`c600_run::meter::Amounts`). The layer depends on no language crate (KN2.6,
//! LI.20), so it keeps its own type with the same field names, and a host maps them by name.

use c600_kernel::json::{self, Value};

use crate::receipt::{code, Error};
use crate::store::Profile;

/// A deterministic budget in the runtime meter's four dimensions (DET.27).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Budget {
    pub steps: u128,
    pub kernel_cost: u128,
    pub memory: u128,
    pub output: u128,
}

impl Budget {
    /// The budget ceiling of every profile (BM3; owner, 21 November 2026, option 1:
    /// `docs/wiki/decisions/owner-decisions-2026-11-21-ceilings-e3.md`): steps 2^21, memory 2^27 and output 2^24,
    /// each a measured peak of the shipped entries times four, rounded up to a power of two; kernel cost keeps the
    /// draft's 2^40 until the rows are priced, and is then measured again. It is `c600-tools`' `tool::LIMITS`.
    pub const CEILING: Budget = Budget { steps: 1 << 21, kernel_cost: 1 << 40, memory: 1 << 27, output: 1 << 24 };

    /// The draft ceiling every profile had before the owner's values (steps 2^20, kernel cost 2^40, memory 2^32,
    /// output 2^26). The frozen candidate 013900ff and the C9 sitting's build admit under it.
    pub const DRAFT_CEILING: Budget = Budget { steps: 1 << 20, kernel_cost: 1 << 40, memory: 1 << 32, output: 1 << 26 };

    /// The dimensions' names and amounts, in the meter's order.
    pub fn dimensions(&self) -> [(&'static str, u128); 4] {
        [("steps", self.steps), ("kernel_cost", self.kernel_cost), ("memory", self.memory), ("output", self.output)]
    }

    /// The lower amount of the two in each dimension.
    pub fn min(self, other: Budget) -> Budget {
        Budget {
            steps: self.steps.min(other.steps),
            kernel_cost: self.kernel_cost.min(other.kernel_cost),
            memory: self.memory.min(other.memory),
            output: self.output.min(other.output),
        }
    }

    /// The digest and journal form: each dimension's amount as a canonical decimal, since an amount may pass the
    /// JSON integer range.
    pub fn to_json(&self) -> Value {
        let [steps, kernel_cost, memory, output] = self.dimensions().map(|(name, amount)| (name, decimal(amount)));
        json::object([steps, kernel_cost, memory, output])
    }

    /// Reads [`Budget::to_json`]'s form strictly: the four dimensions and no other field, each a canonical decimal.
    pub fn from_json(value: &Value) -> Option<Budget> {
        (value.as_object()?.len() == 4).then_some(())?;
        let amount = |name: &str| {
            let text = value.get(name)?.as_str()?;
            let amount: u128 = text.parse().ok()?;
            (amount.to_string() == text).then_some(amount)
        };
        Some(Budget {
            steps: amount("steps")?,
            kernel_cost: amount("kernel_cost")?,
            memory: amount("memory")?,
            output: amount("output")?,
        })
    }

    /// Admits `declared` under `ceiling`: the declared budget, or the refusal of a missing declaration or of the first
    /// dimension, in the meter's order, over the ceiling.
    pub fn admit(declared: Option<Budget>, ceiling: Budget) -> Result<Budget, Error> {
        let Some(budget) = declared else {
            let detail = "the invocation declares no budget: a program row's envelope carries one (KN7.7)";
            return Err(Error::new(code::BUDGET_NOT_DECLARED, detail));
        };
        let over =
            budget.dimensions().into_iter().zip(ceiling.dimensions()).find(|((_, asked), (_, most))| asked > most);
        match over {
            Some(((name, asked), (_, most))) => {
                let detail = format!("the budget's `{name}` {asked} is over the ceiling {most}");
                Err(Error::new(code::BUDGET_CEILING, detail))
            }
            None => Ok(budget),
        }
    }
}

fn decimal(amount: u128) -> Value {
    Value::String(amount.to_string())
}

impl Profile {
    /// The profile's budget ceiling (BM3): [`Budget::CEILING`] for every profile (owner, 21 November 2026).
    pub fn budget_ceiling(self) -> Budget {
        Budget::CEILING
    }
}
