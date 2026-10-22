use super::{Change, Journal, Plan, Snapshot, planning, validation};
use crate::{Error, Result, Scope};
use serde::Serialize;
pub trait Backend {
    fn snapshot(&mut self, scope: &Scope, change: Option<&Change>) -> Result<Snapshot>;
    /// One exact mutation, no automatic retry. A failure may follow remote acceptance.
    fn apply(&mut self, plan: &Plan) -> Result<()>;
}
#[derive(Debug, Serialize)]
pub struct Receipt {
    pub schema: &'static str,
    pub plan_sha256: String,
    pub write_attempted: bool,
    pub state: &'static str,
    pub replay_allowed: bool,
    pub sign_in_verified: bool,
}
pub fn execute(
    backend: &mut impl Backend,
    journal: &Journal,
    p: &Plan,
    approval: &str,
    now: impl Fn() -> u64,
) -> Result<Receipt> {
    planning::validate(p, now())?;
    let hash = crate::digest(p)?;
    if hash != approval {
        return Err(Error("approval_mismatch"));
    }
    let fresh = backend.snapshot(&p.scope, Some(&p.change))?;
    validation::snapshot(&fresh, now())?;
    if !planning::same_state(&fresh, &p.before)? {
        return Err(Error("remote_state_drift"));
    }
    let admission = journal.admit(&hash, &p.scope)?;
    let mut receipt = Receipt {
        schema: "zix-microsoft-exchange-receipt/v1",
        plan_sha256: hash,
        write_attempted: true,
        state: "outcome_unknown",
        replay_allowed: false,
        sign_in_verified: false,
    };
    if backend.apply(p).is_err() {
        return Ok(receipt);
    }
    let after = match backend.snapshot(&p.scope, Some(&p.change)) {
        Ok(s) => s,
        Err(_) => return Ok(receipt),
    };
    if validation::snapshot(&after, now()).is_err() || after.scope != p.scope {
        return Ok(receipt);
    }
    let target = after.recipients.iter().find(|r| match &p.change {
        Change::CreateShared { address, .. } => &r.primary == address,
        _ => Some(r.id.as_str()) == p.change.mailbox(),
    });
    let verified = super::postcondition::verify(&p.before, &after, &p.change)?;
    if verified {
        receipt.state = "verified";
        receipt.sign_in_verified = target.is_some_and(|r| r.sign_in_blocked == Some(true));
        admission.completed()?;
    }
    Ok(receipt)
}
