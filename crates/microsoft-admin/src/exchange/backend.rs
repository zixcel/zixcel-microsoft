use super::{Backend, Change, Plan, PowerShell, Snapshot};
use crate::{Error, Result, Scope};
impl Backend for PowerShell {
    fn wait_for_readback(&mut self) {
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
    fn snapshot(&mut self, scope: &Scope, change: Option<&Change>) -> Result<Snapshot> {
        if let Some(c) = change {
            c.validate_syntax()?;
        }
        let value = self.invoke(scope, "snapshot", change)?;
        let mut s: Snapshot =
            serde_json::from_value(value).map_err(|_| Error("invalid_exchange_snapshot"))?;
        if s.scope != *scope {
            return Err(Error("exchange_scope_mismatch"));
        }
        if let Some(graph) = &mut self.graph {
            use crate::graph::Transport;
            let me = graph
                .get("/v1.0/me?$select=id")
                .map_err(|_| Error("subject_unverified"))?;
            let org = graph
                .get(&format!("/v1.0/organization/{}?$select=id", scope.tenant))
                .map_err(|_| Error("tenant_unverified"))?;
            if me["id"].as_str() != Some(&scope.subject)
                || org["id"].as_str() != Some(&scope.tenant)
            {
                return Err(Error("graph_scope_mismatch"));
            }
            for r in &mut s.recipients {
                let selected = change.is_some_and(|c| {
                    c.mailbox() == Some(r.id.as_str())
                        || matches!(c,Change::CreateShared{address,..} if *address==r.primary)
                });
                if !selected {
                    continue;
                }
                let v = graph
                    .get(&format!("/v1.0/users/{}?$select=id,accountEnabled", r.id))
                    .map_err(|_| Error("sign_in_unverified"))?;
                if v["id"].as_str() != Some(&r.id) {
                    return Err(Error("mailbox_identity_mismatch"));
                }
                r.sign_in_blocked = v["accountEnabled"].as_bool().map(|b| !b);
            }
        }
        Ok(s)
    }
    fn apply(&mut self, p: &Plan) -> Result<()> {
        let v = self.invoke(&p.scope, "apply", Some(&p.change))?;
        if v["acknowledged"] != true {
            return Err(Error("exchange_not_acknowledged"));
        }
        Ok(())
    }
}
